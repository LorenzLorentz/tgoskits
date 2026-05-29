# eBPF follow-up 工作日志

按时间倒序追加 (最新在最上). 格式见
[WORKFLOW.md §4.1](WORKFLOW.md).

---

## 2026-05-29 — T3 D3: profile demo (kprobe syscall 频次画像)

- author: claude (代 LorenzLorentz)
- base: `feat/starry-ebpf-userspace` @ `9bc47f9b6`
- 目标: demo-stack-design.md §2.2 D3 `profile` — kprobe + HashMap, 与 D1
  同色 (无 ringbuf 依赖), 但采集面是**整条 syscall 分发路径的频次直方图**
  (perf-top-for-syscalls), 区别于 D1 的"单 syscall 精确计数"与 D2 的 sched
  ringbuf.

### 内核侧能力复核 (定设计前逐条对代码核实)

- **helper 集** (`kbpf_basic::helper::init_helper_functions`, `ebpf/mod.rs:57`):
  注册了 map lookup/update/delete、`bpf_ktime_get_ns`、`bpf_perf_event_output`、
  **`bpf_probe_read` (id 4)**、`trace_printf`、ringbuf 一族。**未注册**
  `bpf_get_smp_processor_id` (8) 与 `bpf_get_current_pid_tgid` (14) →
  demo-stack-design §2.2 原拟的 `bpf_get_smp_processor_id` + `PT_REGS_IP` 路线
  **不可用** (且 kprobe 入口 IP 恒等于探测点地址, 当 key 只会得到 1 个桶,
  违反 §4.2 "≥3 distinct" 判据). 改设计如下。
- **kprobe ctx ABI**: `KprobePerfCallBack::call` 把 `&mut PtRegs` 当单指针 ctx
  喂给 rbpf (`perf/kprobe.rs:150` + `perf/bpf.rs::execute_with_ptregs`);
  aya `ProbeContext::arg(0)` 在 x86_64 读 `PT_REGS_PARM1=rdi`. 与 kret/
  syscall_ebpf demo 同路, 已验可读寄存器。
- **syscall 号取值**: kprobe 挂 `starry_kernel::syscall::handle_syscall(uctx:
  &mut UserContext)` → arg0(rdi)=`&UserContext`. x86_64 `UserContext{tf:TrapFrame,
  ..}`, `TrapFrame.rax` 是首字段, `sysno()==rax` (`axcpu/x86_64/context.rs:120`),
  即 `&UserContext` 偏移 0. 故一次 `bpf_probe_read::<u64>(uctx)` 读 8 字节即得
  syscall 号, **无需追结构偏移**。
- **attach 路径**: aya `KProbe::attach` 在 `FEATURES.bpf_perf_link()==false` 时走
  `perf_attach_either` = ioctl `PERF_EVENT_IOC_SET_BPF` + `IOC_ENABLE`, 二者
  `perf/mod.rs::ioctl` 都实现. 启动日志里的 `bpf: unsupported command 28`
  (BPF_LINK_CREATE) / `18` (BTF_LOAD) 与 CPUMAP/DEVMAP "not implemented" 都是
  aya **加载期能力探测**的噪声, 非致命 (程序成功 load + attach)。

### 改动 (本次)
1. `user/ebpf/profile/` (新, 3-crate aya workspace, 仿 syscall_ebpf/sched_trace):
   - `profile-ebpf`: `#[kprobe] handle_syscall` → `bpf_probe_read(arg0)` 取
     syscall 号 → `HashMap<u32,u64>[sysno]++`. 无 loop (verifier 友好)。
   - `profile` (loader, 纯 sync 无 tokio): attach kprobe, SIGTERM/SIGINT 落
     `AtomicBool` → 排空 + 按计数降序打印 `PROFILE_BEGIN..PROFILE_END
     total=.. distinct=.. top1_*`. 进程退出关 fd → kprobe Drop unregister。
   - `profile-common`: 记录 `SYSNO_OFFSET_IN_USERCONTEXT=0` 的 ABI 假设。
2. `scripts/axbuild/src/starry/user_ebpf.rs`: `PROGRAMS` + 单测 `expected` 加
   `"profile"`。
3. `test-suit/starryos/ebpf/qemu-smp1/profile/`: `qemu-x86_64.toml` +
   `sh/profile.sh` (反 fallback 验证)。
4. **内核修复** `kernel/src/syscall/mod.rs`: `handle_syscall` 加
   `#[inline(never)]` (见下根因)。

### 实跑发现 + 根因 (§4.2/4.3) — docker 实跑 (2026-05-29)
首跑 (未加 `#[inline(never)]`): kallsyms 命中 `handle_syscall`、program load +
attach 成功、loader 走到 dump, 但 `total=0` —— 连 loader 自身的 `nanosleep`
都没计到, 即 **kprobe 一次都没 fire**。

**可证伪根因**: `handle_syscall` 只有一个调用点 (`task/user.rs:36`
`ReturnReason::Syscall => handle_syscall(&mut uctx)`); release 下 LLVM 把整个
分发器 inline 进该 run loop, 同时仍发出独立符号 (故 kallsyms 命中) —— int3 落
在那份**从不被执行**的离线副本上, 探针永不触发。这与 commit `1f6579f41` 修
`sys_getpid` 的 gap #3 **完全同型**。修法: `handle_syscall` 加 `#[inline(never)]`,
强制调用点真的 `call` 到带 int3 的符号。

附带过程: 改源后 `handle_syscall` 地址漂移, 而 kallsyms 是 build.rs `nm` 上一
轮二进制的两遍机制 (仅 `rerun-if-changed=ext_linker.ld` 触发重算). 故需 `touch
ext_linker.ld` 强制 build.rs 重跑, 连编两遍令 kallsyms 收敛回当前 .text 布局
(kallsyms 串在 .rodata, 不挪 .text 地址, 两遍稳定)。

### 反 fallback 自检 (§4.6)
1. **绕路?** 否. 数据全程 kprobe→rbpf VM→HashMap→user 态 `bpf(MAP_GET_NEXT_KEY/
   LOOKUP)` 迭代, 不读 trace_pipe / dmesg. 断言 `total≥1000` (非 `>0`)、
   `distinct≥3`、`top1≥20%` (整数判 `top1*5≥total`)。
2. **依赖未通子系统?** 否. 只用已注册 helper (`bpf_probe_read` + map) 与已端到
   端的 kprobe+HashMap 路, 不碰 ringbuf / smp_id / pid helper。
3. **Linux 语义对齐?** syscall 号即 x86_64 saved `rax`; top1=1=write (dd bs=1
   的 2 万次 write), 与 Linux `perf top -e raw_syscalls` 同义。
4. **新代码可被触发?** `dd if=/dev/zero of=/dev/null bs=1 count=20000` 造 2 万
   read+2 万 write 主导热点, 必触发。

### 验证 (§4.5)
- [x] `cargo xtask starry user-ebpf build --program profile --arch x86_64`: 通过
  (bpf-linker 在 `/cargo-target/cargo-bin/bin`, 需加 PATH; loader musl 链接 OK)。
- [x] qemu x86_64 (`-g ebpf -c profile`): **PASS** —
  `PROFILE_END total=40310 distinct=33 top1_sysno=1 top1_count=20001 top1_pct=49.6`
  / `PROFILE_PASS ... 3 re-attach cycles clean`. 端到端: kprobe attach →
  handle_syscall fire (每条 syscall) → rbpf VM → `bpf_probe_read` 取 sysno →
  HashMap → user 态降序直方图。3 次 attach/detach (`write_kernel_text` 改内核
  文本) 无 panic、样本数稳定 (40294) → unregister 路径不泄漏。
- [x] `cargo fmt --all -- --check` (主 workspace) + profile crate fmt: 通过。
- [ ] qemu riscv64/aarch64/loongarch64: 待补 (sysno 取值偏移与 arg ABI 架构相关;
  `profile-common::SYSNO_OFFSET_IN_USERCONTEXT` 标了 x86_64=0 的假设, 其它架构
  `UserContext` 首字段非 rax 时需调整 + 重验)。

### 内核修复 PR 边界
`#[inline(never)] handle_syscall` 属"让 kprobe 能挂到 syscall 分发器"的单点修复,
与 commit `1f6579f41` 的 `sys_getpid` 同类, 可并入同一 kernel PR 或独立小 PR;
与 demo crate / test-suit 提交分开。

### 已知未做 (follow-up)
- rootfs 自动安装 + `-c ebpf` group 入主 CI: 同 D2 条, 二进制架构相关, 暂走
  standalone 脚本 + 手跑。
- 多架构: 见上验证矩阵未勾项。

---

## 2026-05-29 — T3 D2: sched_trace demo (raw tp + perf ringbuf)

- author: claude (代 LorenzLorentz)
- base: `feat/starry-ebpf-userspace` @ `ade35a8e6`
- 目标: demo-stack-design.md §2.3 D2 `sched_trace` — `sched:sched_switch`
  raw tracepoint → BPF prog 写 perf ringbuf → 用户态 `PerfEventArray` reader.

### 内核侧支持复核 (用户要求"确认 ebpf 内核相关支持情况")

D2 的两个前置在审计后已分别落地, 本次逐条对代码复核 (不是只读 journal):

1. **TP-P0-1 sched_switch (已通)**:
   - `kernel/src/tracepoint/sched.rs`: `define_event_trace!(sched_switch,
     TP_PROTO(prev_tid: u64, next_tid: u64, prev_state: u32))` + `#[impl_interface]
     impl SchedTracepoint`.
   - `os/arceos/modules/axtask/src/run_queue.rs:701-708`: `switch_to` 在架构切换
     前 `call_interface!(SchedTracepoint::on_sched_switch(prev.id, next.id,
     prev.state() as u32))`, feature `tracepoint-hooks` 默认由 `kernel/Cargo.toml`
     ax-feat (line 52) → `axfeat/Cargo.toml:67` → `ax-task/tracepoint-hooks` 级联开启.
   - raw tp attach: `perf/raw_tracepoint.rs::bpf_raw_tracepoint_open` →
     `find_ext_tracepoint_by_name("sched_switch")` 现可命中.
2. **PERF-P0-1/2/3 perf mmap ringbuf (已通)**:
   - `perf/bpf.rs::BpfPerfEventWrapper::device_mmap`: 分配 `(1+2^N)` 连续页 →
     `BpfPerfEvent::do_mmap` 初始化 `perf_event_mmap_page`; `pages` 字段在 inner
     之后声明, Drop 时归还 (P0-3 leak 已闭). `write_event` 在 `pages.is_some()`
     时真正写 (不再静默丢).
   - `perf/mod.rs:136 PerfEvent::device_mmap` ← `syscall/mm/mmap.rs:176` 的
     `fl.device_mmap(offset,length)` 调到; `bpf_perf_event_output` 经
     `PERF_FILE` 表 → `write_event`.
3. **helper / ABI 复核**:
   - `bpf_perf_event_output` + `bpf_ktime_get_ns` 都在
     `kbpf_basic::helper::init_helper_functions` 注册 (`ebpf/mod.rs:57`);
     `transform.rs::ebpf_time_ns` 返回 `monotonic_time_nanos`.
   - `BPF_F_CURRENT_CPU` 在 kbpf-basic `perf_event_output` 解析为
     `current_cpu_id()` → map[cpu] → fd, 与 aya `PerfEventArray::output` 一致.
   - **raw tp 上下文 ABI**: ktracepoint `basic_macro.rs:137`
     `args = [AsU64::as_u64($arg)..]`, 即每个 TP_PROTO 字段 widen 成一个 u64 slot.
     sched_switch → `[prev_tid, next_tid, prev_state(u32→u64)]`, BPF prog 按
     `*const [u64;3]` 读 (与 `user/ebpf/rawtp` 同形态).
   - `kbpf-basic` map 分发支持 `BPF_MAP_TYPE_PERF_EVENT_ARRAY` (`map/mod.rs:268`).
   - `BPF_MAP_TYPE_RINGBUF` 的 map-fd mmap (PERF-P1-1) 仍未做 → **必须走
     legacy PerfEventArray (perf-buffer), 不能用 aya `RingBuf`**. 本 demo 即如此.

→ 结论: D2 所需内核能力全部就绪, 可以落 demo.

### 改动 (本次)
1. `user/ebpf/sched_trace/` (新, 3-crate aya workspace, 仿 rawtp/syscall_ebpf):
   - `sched_trace-common`: `#[repr(C)] SchedSwitchEvent { prev_tid, next_tid,
     prev_state, _pad, ts_ns }` (32B, 共享 ABI).
   - `sched_trace-ebpf`: `#[raw_tracepoint(tracepoint="sched_switch")]` 读
     `[u64;3]` → 填 `SchedSwitchEvent` (ts 用 `bpf_ktime_get_ns`) →
     `PerfEventArray::output(&ctx, &ev, 0)`.
   - `sched_trace` (loader, 纯 sync, 无 tokio): `PerfEventArray::open` 每 cpu
     一个 buffer (attach 前先开, 避免早期丢), `for_each` 解析 `PerfEvent::Sample`
     → `println!("prev=.. next=.. state=.. ts=..")`. stdout 行缓冲, SIGTERM 不丢
     已打印行; 进程退出关 fd → raw tp 自动 unregister.
2. `scripts/axbuild/src/starry/user_ebpf.rs`: `PROGRAMS` + 单测 `expected` 加
   `"sched_trace"`.
3. `test-suit/starryos/ebpf/sched_trace.sh` (新): 反 fallback 验证脚本.

### 反 fallback 自检 (§4.6)
1. **绕路?** 否. 数据来自 user 态 `PerfEventArray` (mmap'd ringbuf), 不读
   trace_pipe 文本, 不读 dmesg. 脚本断言 `prev=` 记录数 ≥ 100 (非 `>0`) +
   ≥2 个不同 next tid (反证 probe 装错位置).
2. **依赖未通子系统?** 否 — PERF-P0-1 + TP-P0-1 均已在代码中复核为通.
   未使用 `BPF_MAP_TYPE_RINGBUF` map mmap (PERF-P1-1 未做), 故用 PerfEventArray.
3. **Linux 语义对齐?** sched_switch 三元组 (prev/next tid + prev_state) 与
   Linux `sched:sched_switch` 关键字段对齐; comm/prio 走 saved_cmdlines (见
   2026-05-23 条). 同样 aya 程序在 host Linux 可跑作对照 (待用户在 docker 后做).
4. **新代码可被触发?** `sh -c 'while :; do :; done'` × 2 制造调度抖动即触发.

### 验证 (§4.5) — docker 实跑 (2026-05-29)
- [x] `cargo xtask starry user-ebpf build --program sched_trace --arch x86_64`:
  **通过** (eBPF 字节码经 bpf-linker 0.10.3 编出, loader musl 链接 OK,
  `/cargo-target/x86_64-unknown-linux-musl/release/sched_trace` 1.5MB ELF).
- [x] qemu x86_64 (`--test-group ebpf -c sched_trace`): **FAIL — 内核 panic**.

#### 实跑发现 (复现现状, §4.2)
启动 → `Initialized 5 tracepoints` (sched_switch/_fork/_exit + openat/mkdirat) →
loader 加载 prog + 建 PERF_EVENT_ARRAY map + attach raw tp 成功 → **下一次
sched_switch 触发即 panic**:
```
tracepoint/mod.rs:115:18: sleeping or rescheduling is not allowed in
atomic context: irq_enabled=false, preempt_count=1
```
归类 **A (内核 panic)**.

**可证伪根因 (§4.3)**: `sched_switch` 在 `axtask::run_queue::switch_to` 内触发
(原子上下文: IRQ off + preempt_count=1), 而 ktracepoint 的 fire path
`KernelTraceOps::read_tracepoint_state` (`tracepoint/mod.rs:115`) 对
`KernelExtTracePoint = Arc<ax_sync::Mutex<ExtTracePoint>>` (mod.rs:14/26) 做
`ext_tp.lock()` —— `ax_sync::Mutex` 是**会睡眠**的锁, 在原子上下文加锁即触发
axtask 的 "atomic context" 守卫 panic. 这条路径仅在 static key 打开 (即有
consumer attach) 后才走, 所以 boot 不 panic、attach 后第一次切换才炸. → TP-P0-1
(commit `6863ee62f`) 落地时**从未端到端验过带 BPF consumer 的 sched_switch**,
是预存内核 bug, 影响**任何** sched_switch consumer, 不只本 demo.

次要 (非致命): `kbpf_basic::preprocessor:85 relocation for ty: 0 not implemented,
instruction index: 16` —— 这是 kbpf-basic 把一条 `src=0` 的普通 `LD_DW_IMM`
(纯 64-bit 立即数加载) 误报成 relocation; map 引用走 `BPF_PSEUDO_MAP_FD` (src=1)
是被正确处理的, 故本条仅是噪声日志, 不影响 map 接线.

#### 修复方向 (留独立 kernel PR, 不与 demo 混)
让 sched_switch fire path 原子安全:
1. `KernelExtTracePoint` 的 `ax_sync::Mutex` → 自旋锁 (`kspin`/`SpinNoIrq`):
   register/unregister (syscall 上下文) 与 fire (原子上下文) 都不睡. callback 执行
   (rbpf VM, 已在 `spin::Mutex` 下; perf `write_event` 写 mmap 页无 alloc;
   `PollSet::wake` 走唤醒队列) 需逐一确认原子安全.
2. 若后续要支持 `echo 1 > events/sched/sched_switch/enable` (Default callback
   走 trace_pipe), 还需把 `raw_pipe` / `cmdline_cache` 两把 `ax_sync::Mutex`
   也改原子安全或在原子上下文跳过.
本 demo crate 本身正确 (编译 + 加载 + attach 均通); 阻塞点在内核 TP 基础设施.

#### 修复落地 + 端到端通过 (2026-05-29 二次实跑)

用户选 "修内核并重跑验证". 实跑中逐个炸出并修掉**三个**独立内核 bug
(均为首次有 BPF consumer 走 raw-tp + perf 路径才暴露, 全是预存 latent bug):

1. **TP fire path 原子上下文睡眠锁** (上文 panic #1):
   `KernelExtTracePoint` 由 `ax_sync::Mutex` → `ax_kspin::SpinNoPreempt`
   (`tracepoint/mod.rs`). fire path (`read_tracepoint_state`) 在 `switch_to`
   原子上下文加锁不再睡. `raw_pipe`/`cmdline_cache` 仍保留 `ax_sync::Mutex`
   (只在阻塞的 trace_pipe 文本路径用, 不在原子上下文).

2. **raw tp 回调 `Ctx` downcast 失败** (panic #2 `raw_tracepoint Ctx mismatch`):
   `ktracepoint::RawTraceEventFunc` 把 payload 存为 `Box<dyn Any+Send+Sync>`,
   `call` 传给闭包的是 `&self.data` —— 闭包看到的具体类型是**那个 Box 本身**,
   不是 `Ctx`. 故 `data.downcast_ref::<Ctx>()` 永远失败. 改成先 downcast 到
   `Box<dyn Any+Send+Sync>` 再 downcast 到 `Ctx` (`perf/raw_tracepoint.rs`).
   (用 host `rustc` 小程序复现确认了这个 deref-vs-unsize 强制转换行为.)

3. **`write_kernel_text` 非 LIFO 嵌套 `SpinNoIrq` 泄漏 IRQ-disabled** (panic #3
   `shm.rs:387 ... irq_enabled=false, preempt_count=0`):
   `mm/access.rs::write_kernel_text` 先取 `kernel_aspace().lock()` (SpinNoIrq A,
   存 IRQ=on/关 IRQ), 再进 `stop_machine` 取 `STOP_MACHINE_LOCK` (SpinNoIrq B,
   此时存的是 IRQ=**off**); A 的 guard 被 move 进闭包**先于** B 释放 (恢复
   IRQ=on), B 后释放又把 IRQ 恢复成它存的 **off** —— 两把 IRQ-save 锁交叉了存档,
   函数返回时 IRQ 被遗留为 disabled. 平时调 `write_kernel_text` 后会 sysret 从用户
   上下文恢复 IF 而掩盖; 但 `disable_key` 发生在 `do_exit`→`close_all_fds` 里
   (raw tp fd drop → unregister → disable_key), 紧接着 `clear_proc_shm` 仍在内核态
   且带着被泄漏的 IRQ-off, 撞上原子上下文守卫. 修法: 把 `kernel_aspace().lock()`
   挪进 `stop_machine` 闭包内 (LIFO 嵌套), 与已正确的 kprobe
   `set_writeable_for_address` 路径一致. **此 bug 影响任何在 IRQ-on 内核态调
   `write_kernel_text` 且不接 sysret 的场景, 与 eBPF 无关.**

   用 `error!("DBG ... irqs_enabled()")` 在 `do_exit`/raw-tp Drop 两处打点,
   精确定位到 `unregister` 前后 IRQ 由 on 变 off, 锁定 #3 根因后回退打点.

- [x] qemu x86_64 (`--test-group ebpf`): **PASS** —
  `sched_trace: captured 608 sched_switch records` /
  `SCHED_TRACE_PASS: 608 records, 9 distinct next tids` (≥100 + ≥2 distinct
  两条强断言都过). 端到端链路: raw tp attach → sched_switch fire → rbpf VM →
  `bpf_perf_event_output` → mmap'd PerfEventArray ringbuf → 用户态 `for_each` 排空.
- [x] `cargo fmt --all -- --check`: 通过.
- [ ] qemu riscv64: 待补 `build-riscv64gc-*.toml` + `qemu-riscv64.toml` +
  riscv64 loader 二进制后验 (三处内核修复均为架构无关逻辑, 预期同样通过).

#### 内核修复 PR 边界
三个修复都属"让 tracepoint eBPF 基础设施在原子/退出路径正确工作", 同一 feature
启用面, 归一个 kernel PR (与 demo crate / test-suit 的提交分开). 涉及文件:
`kernel/src/tracepoint/mod.rs`, `kernel/src/perf/raw_tracepoint.rs`,
`kernel/src/mm/access.rs`.

### 已知未做 (follow-up)
- **rootfs 自动安装 + `-c ebpf` 测试 group toml**: 现 harness 的 sh-pipeline 只
  从 case `sh/` 注入文件, 无声明式"注入预编译二进制"字段; eBPF 二进制是架构相关
  产物, 不宜入库. 按 demo-stack-design §5 ("暂不入主 CI, 先本地手跑"), 本次只交付
  standalone 脚本 + 手跑步骤; CI group 接线留独立 follow-up (需新增 build→stage
  二进制到 rootfs 的步骤, 改动面要单独验).
- aya_log 路径现在 PERF-P0-1 通了, 理论上 `info!` 可恢复 (WORKFLOW §7.2 欠账),
  但 D2 走显式 PerfEventArray 更稳, 不依赖 aya_log 的 RINGBUF-map mmap.

---

## 2026-05-23 — T1 开工: 落地 TP-P0-1 + TP-P0-2 (sched 一组)

- author: claude (代 LorenzLorentz)
- base: `feat/starry-ebpf-userspace` @ `2f2533968`
- 目标残缺项: TP-P0-1 (sched_switch hook) + TP-P0-2 (sched_process_fork / _exit)
- 范围决策: 用户在 4 个 P0 选项中选了 "sched 一组". 不在本 PR 内动 syscall
  (TP-P0-3/P0-4 留给独立 PR), 不在本 PR 内动 perf-ringbuf (T2 独立 PR).
  符合 [WORKFLOW.md §5](WORKFLOW.md) 黑名单: "一个 PR 同时改 tracepoint +
  perf-ringbuf + demo 三件套" → 禁止. T1 内部多项 P0 共享 PR (改动面同色) 是允许的.

### 复现现状 (§4.2)
未跑 qemu (本机 macOS 不能 build, 见 [WORKFLOW.md §2.1](WORKFLOW.md)).
通过 `grep -rn 'ktracepoint::define_event_trace!' os/StarryOS/kernel/` 确认
仓库内只有 `syscalls:sys_enter_openat` (`syscall/fs/fd_ops.rs:140`) 与
`syscalls:sys_mkdirat` (`syscall/fs/ctl.rs:92`) 两个事件, `sched:*` 整个
subsystem 缺位. 归类 **B** (业务异常退出, attach 直接 EINVAL).

### Claim (§4.3)
**TP-P0-1**: `bpf(PERF_EVENT_OPEN, type=TRACEPOINT, name="sched_switch")` /
`bpf(BPF_RAW_TRACEPOINT_OPEN, "sched_switch")` 在
`perf/raw_tracepoint.rs:114` 调 `find_ext_tracepoint_by_name("sched_switch")`
返 `None`. 反向验证: 加入 `ktracepoint::define_event_trace!(sched_switch,...)`
+ 实际触发点 → `find_*` 命中 + `events/sched/sched_switch/format` 可读.

**TP-P0-2**: 同上, `sched_process_fork` / `_exit` 同样缺. 修复后必须在 qemu 内
`ls /sys/kernel/debug/tracing/events/sched/` 看到 3 个目录.

### 改动 (§4.4)
**ax-task 侧 (跨 crate hook, 沿用 `ax-crate-interface` 已有模式)**:
1. `os/arceos/modules/axtask/Cargo.toml`: 新 feature `tracepoint-hooks = ["multitask"]`.
2. `os/arceos/modules/axtask/src/sched_tracepoint.rs` (新): `#[def_interface] pub trait SchedTracepoint { fn on_sched_switch(prev_tid, next_tid, prev_state); }`.
3. `os/arceos/modules/axtask/src/lib.rs`: cfg-mod + `pub use SchedTracepoint`.
4. `os/arceos/modules/axtask/src/run_queue.rs::switch_to`: 在 task-ext `on_leave/on_enter` 之后、`(*prev_ctx_ptr).switch_to(...)` 之前, `cfg(feature="tracepoint-hooks")` 下调用 `call_interface!(crate::sched_tracepoint::SchedTracepoint::on_sched_switch(...))`. **关键**: `prev_task.state()` 必须在架构 switch 之前采样 (`exit_current` 等会先把 prev 置为 `Exited`/`Blocked`).
5. `os/arceos/api/axfeat/Cargo.toml`: 级联 `tracepoint-hooks = ["ax-task/tracepoint-hooks"]`.

**starry-kernel 侧**:
6. `kernel/Cargo.toml`: ax-feat 加 `"tracepoint-hooks"`; 新增 `ax-crate-interface.workspace = true` 直接依赖 (impl_interface 用).
7. `kernel/src/tracepoint/sched.rs` (新): 3 个 `define_event_trace!` (`sched_switch`, `sched_process_fork`, `sched_process_exit`) + `#[impl_interface] impl SchedTracepoint for SchedTracepointImpl` 把 hook 路由到 `trace_sched_switch`.
8. `kernel/src/tracepoint/mod.rs`: `mod sched;` + `pub use sched::{trace_sched_process_exit, trace_sched_process_fork}`.
9. `kernel/src/syscall/task/clone.rs::do_clone`: `spawn_task` 与 `add_task_to_table` 之后、vfork-wait 之前调 `trace_sched_process_fork(parent_tid, child_tid)`. 覆盖 `sys_clone` / `sys_clone3` / `sys_fork` / `sys_vfork` (都汇聚到 `do_clone`).
10. `kernel/src/task/ops.rs::do_exit` 头部: `trace_sched_process_exit(curr.id().as_u64(), exit_code)`.

### 反向自检 (§4.6)
1. **绕路?**
   - **是 (有限)**: `sched:sched_process_fork` / `_exit` 用的是 **scheduler task id** (`TaskInner::id()`), 不是 Linux 语义里的 user-visible TID (`Thread::tid()`). 在 Starry 里多数情况下两者相等, 但非 leader `execve` 之后会分裂. 选 scheduler id 的理由: 与 `sched_switch` 的 prev/next id 保持一致 (axtask 那侧只看得到 scheduler id). 不影响 D2 demo (raw tp + perf ringbuf, 只比对相对值).
   - **不是绕路**: 没有用 `dmesg | grep` 代替 ringbuf; 没把 `Unsupported` 改成 `Ok(())`; 没在 BPF prog 里靠 `aya_log` (本 PR 不涉及 user demo).
2. **依赖未通子系统?**
   - **是**: 单跑本 PR, 用户态仍拿不到 ringbuf 数据 (`PERF-P0-1` 未做). 但本 PR 不声称 demo 已通 — D2 sched_trace demo 需 T2 完成后才能验. 当前 PR 只声称 `find_ext_tracepoint_by_name("sched_switch") == Some(_)` + `/sys/kernel/debug/tracing/events/sched/{switch,process_fork,process_exit}/{enable,format,id,filter}` 可读 + `trace_pipe` 看得到记录.
3. **Linux 语义对齐?**
   - sched_switch 字段对齐了关键三元组 (prev_tid / next_tid / prev_state); **未对齐** `prev_comm` / `next_comm` / `prev_prio` (Linux 有). 理由: `comm` 已由 `KernelTraceOps::trace_cmdline_push` 写进 `saved_cmdlines`, trace_pipe / libtraceevent 解析时按 pid 反查, 不需要进 payload; `prio` 在 starry 调度器里语义不直接映射 (RR vs CFS), 留 P1.
   - sched_process_fork / _exit 同理: 单 tid + (exit) exit_code 满足"事件发生" + "进程身份"两个轴. comm 走 saved_cmdlines.
4. **新代码可被触发?**
   - sched_switch: 几乎每次 yield/调度都触发. `cat /sys/kernel/debug/tracing/trace_pipe` 即可看到流水.
   - sched_process_fork: `for i in 1 2 3; do sh -c 'true'; done` 触发 3 次 fork (busybox 内 `sh -c` 通常 fork+exec).
   - sched_process_exit: 同上, 每个子进程退出一次.

### 验证 (§4.5)
- [ ] `cargo fmt --all -- --check`: 本机 macOS 跑不了 (busybox 工作流共享陷阱). 用户在 docker 内验.
- [ ] `cargo xtask clippy --package starry-kernel --package ax-task`: 同上.
- [ ] `cargo xtask starry build --arch {x86_64,riscv64,aarch64}`: 同上.
- [ ] `cargo xtask starry build --arch loongarch64`: 同上 (PR-A 已修).
- [ ] qemu x86_64: `ls /sys/kernel/debug/tracing/events/sched/` 应有 3 项; `cat events/sched/sched_switch/format` 应含 `prev_tid`, `next_tid`, `prev_state` 字段; `cat trace_pipe` 应不停打印 sched 记录 (注意先 `echo 1 > .../sched_switch/enable`).
- [ ] qemu x86_64 反向证伪: `echo 0 > .../sched_switch/enable` 后 `trace_pipe` 不再有 sched 记录.

### 已知未做 (留给 follow-up)
- TP-P0-3 (sys_clone raw tp) + TP-P0-4 (sys_enter_{read,write,execve,clone,exit_group}): 用户没选, 留独立 PR. `user/ebpf/rawtp` 在本 PR 后仍 EINVAL.
- TP-P1-1 (trace_pipe poll) / TP-P1-2 (tracing_on) / TP-P1-3 (events/<sys>/enable 组开关): 与本 PR 不耦合.
- D2 sched_trace demo: 等 T2 (perf-ringbuf) 完成后再做 (本 PR 的 sched_switch 触发是 D2 的前置).
- 预存 fmt 差异: `scripts/axbuild/src/starry/user_ebpf.rs:115` (PR-D 漏 fmt). 不在本 PR 修, 留 style PR.

### Task 状态 (本 session)
- #1 scout fork/exit hooks + crate_interface trait location: completed
- #2 axtask SchedTracepoint trait: completed
- #3 kernel/src/tracepoint/sched.rs: completed
- #4 wire switch_to + do_clone + do_exit hook calls: completed
- #5 self-scan diff (fmt / clippy traps): completed (本机限制下做静态自检)
- #6 journal: in_progress → completed (本条)

---

## 2026-05-23 — 审计 + 工作流落地

- author: claude (代 LorenzLorentz)
- 基线: `feat/starry-ebpf-userspace` @ `2f2533968`
  (PR-A eBPF runtime + PR-D 用户态程序的并集; PR-B/C LKM 不在范围)
- 产出 (4 个 .md, 共 ~1800 行, 不动代码):
  - `docs/ebpf-followup/tracepoint-audit.md`
  - `docs/ebpf-followup/perf-ringbuf-audit.md`
  - `docs/ebpf-followup/demo-stack-design.md`
  - `docs/ebpf-followup/WORKFLOW.md`
- 关键发现 (审计落点):
  1. **tracepoint 覆盖面 < 1%**: 全仓只有 `syscalls:sys_enter_openat` 与
     `syscalls:sys_mkdirat` 两个 `define_event_trace!` 调用 (`syscall/fs/{fd_ops,ctl}.rs`).
     `sched:sched_switch` / `sys_clone` 等 demo 必需的事件全部缺失.
  2. **`user/ebpf/rawtp` 直接不工作**: 它 attach `sys_clone` raw tp,
     `find_ext_tracepoint_by_name("sys_clone") == None` → bpf(BPF_RAW_TRACEPOINT_OPEN) 返 EINVAL.
  3. **perf ringbuf 静默丢失**: `PerfEvent::file_mmap/device_mmap` 默认
     `Err(NoSuchDevice)`, userland mmap 失败; `BpfPerfEventWrapper.phys_addr`
     永远 None, `write_event` 直接 `Ok(())` 不写 (`perf/bpf.rs:52-65`).
     **所有 BPF prog 内的 `aya_log_ebpf::info!` 都被丢弃**.
  4. **HashMap / kprobe 路径完全可用**: 与 ringbuf 不耦合的 demo (D1
     syscall_count, D3 profile_kprobe) 可以在 ringbuf 修通之前先跑通.
- 决策: 三任务排序定为 T2 (ringbuf) ≈ T1 (tracepoint) 平行启动, T3
  分两阶段 (D1/D3 不依赖 ringbuf; D2 等齐). 详见
  [WORKFLOW.md §1.2](WORKFLOW.md) 依赖图.
- 反 fallback 黑名单 ([WORKFLOW.md §5](WORKFLOW.md)):
  - 不允许 BPF 内 `info!` 当成"通了" — 它在 ringbuf 修通前永远静默.
  - 不允许把 `Unsupported` 改成 `Ok(())`.
  - 不允许 `dmesg | grep` 代替 ringbuf 验证.
  - 不允许弱断言 (`[ -n "$_t" ]` / `grep -q invoke`); 必须断言计数 ± 容差.
- 验证状态 (本 session, 仅 docs):
  - `git ls-tree HEAD os/StarryOS/docs/ebpf-followup/` ✅ 4 文件齐.
  - `wc -l docs/ebpf-followup/*.md`:
    - tracepoint-audit.md ~190 行
    - perf-ringbuf-audit.md ~190 行
    - demo-stack-design.md ~220 行
    - WORKFLOW.md ~350 行
    - journal.md (本文件)
  - **docker baseline 已跑** (.docker-run.sh restore 到本工作树, **未**
    `.gitignore`, `git status` 显示为 untracked. 提交前用户决定是否
    `git add` 这个脚本; busybox_fix 历史曾把它 commit 过, 本分支默认不加):
    - 容器: `starryos-dev-local:latest` (基于
      `docker.cnb.cool/starry-os/arceos-build:latest` + libudev-dev/pkg-config/e2fsprogs).
    - 工具链: rustc 1.97.0-nightly (ca9a134e0 2026-04-26),
      cargo 1.97.0-nightly (eb9b60f1f 2026-04-24).
    - `cargo --version` / `rustc --version` ✅
    - `cargo metadata --no-deps` ✅ workspace 223 packages, root `/workspace`.
    - `cargo fmt --all -- --check` ❌ **预存 1 处 fmt diff**:
      `scripts/axbuild/src/starry/user_ebpf.rs:115` — `bail!()` 调用被
      rustfmt 改写成 closure block 形式. 是 PR-D 提交时本机未跑 fmt 漏掉
      的, 与本 follow-up 工作无关. 任何 T1/T2/T3 PR 都应先把这个 diff 修掉
      (在 PR 内顺手 + PR body 注明 "顺手修 PR-D 漏 fmt 的 user_ebpf.rs"),
      或单独开一个 `style(starry/xtask): rustfmt user_ebpf.rs` 微 PR.
    - build / qemu 未跑 (在本审计 session 范围之外, 与 audit 结论无关;
      由具体 PR 启动时再跑, 见 WORKFLOW §6.3).
- 下一步:
  1. 用户阅读 4 个 .md, 决定要不要把 .docker-run.sh restore 回到本分支
     (它在 `.gitignore` 内的 `*.sh` 不会被忽略, 默认会进 git;
     是否入库需要用户决定).
  2. 选 T1 / T2 / T3 任一项开 PR, 按 WORKFLOW §4 流程走.
  3. 推荐 first PR = T3 D1 (syscall_count): 改动最小,
     完全不依赖 ringbuf 修复, 可以最早证明端到端链路.
- Task 状态:
  - #1 审计 tracepoint: completed
  - #2 审计 perf/ringbuf: completed
  - #3 设计 demo 栈: completed
  - #4 写 WORKFLOW.md: completed
  - #5 docker baseline: 留给用户启动 docker 后跑 (本机磁盘/网络条件
    下首次镜像拉取 5-10 min, 在 audit 不动代码的情况下未消耗时间).

---
