# 稳定 eBPF demo 栈设计

> 目标: 在 `feat/starry-ebpf-userspace` 之上, 落地 3 个**可重复跑通、可证伪**
> 的 eBPF demo, 覆盖三类用法 (kprobe + map / raw tracepoint / 调度追踪),
> 并以反 fallback 的方式做验证.
> 前置依赖: [tracepoint-audit](tracepoint-audit.md) §3 P0 + [perf-ringbuf-audit](perf-ringbuf-audit.md) §3.1.

## 1. demo 选型 (排序 = 可工作的优先级)

| # | demo 名 | 类型 | 已有 user 程序 | 内核侧依赖 | 当前阻塞 |
|---|---|---|---|---|---|
| **D1** | `syscall_count` | kprobe + HashMap | `user/ebpf/syscall_ebpf` | ✅ kprobe + HashMap 已通 (PR-A); 不需要 ringbuf | 仅 aya_log 不可用 → 改用 HashMap iter 出数据 |
| **D2** | `sched_trace` | raw tracepoint `sched:sched_switch` | 新建 `user/ebpf/sched_trace` (基于 `rawtp` 改) | ❌ tracepoint 未定义 (TP-P0-1); ⚠️ ringbuf 不通 (PERF-P0-1) | 两件都要修, **不推荐第一个做** |
| **D3** | `profile_kprobe` | kprobe + HashMap (按 caller 计 PC 频次) | 新建 `user/ebpf/profile` (基于 `syscall_ebpf`) | ✅ 同 D1 | aya_log 同样可避开 |

→ **优先级**: D1 → D3 → D2. D1/D3 可以在 PERF-P0 修复之前先跑通 (走 map iter); D2 等 PERF-P0 + TP-P0-1 同时到位.

## 2. demo 详细设计

### 2.1 D1: syscall_count (最稳)

**功能**: 在内核某 syscall 入口注 kprobe; 每次触发把 syscall 号当 key, 计数 + 1; 用户态每 N 秒迭代 map 输出.

**为何稳**:
1. 内核 kprobe 路径 (PR-A `perf/kprobe.rs` + `kbpf-basic` kprobe) 已端到端: `lookup_symbol_addr` → `register_kprobe` → `set_bpf_prog` → `KprobePerfCallBack::call` → rbpf VM 执行.
2. HashMap map (`BPF_MAP_TYPE_HASH`) 是 kbpf-basic 完整实现, 不依赖 mmap; 用户态 `bpf(BPF_MAP_LOOKUP_ELEM/GET_NEXT_KEY)` 已支持 (`ebpf/mod.rs:228-231`).
3. 用户态 loader 已有 `user/ebpf/syscall_ebpf/syscall_ebpf/src/main.rs`, 仅需 (a) 删掉 BPF prog 里的 `info!` (aya_log 走 ringbuf, 当前丢), (b) 验证脚本断言 HashMap 内的实际计数, 不依赖 log.

**改动清单** (不动 demo 选型, 改测试 + 内核辅助):

| 文件 | 改动 |
|---|---|
| `user/ebpf/syscall_ebpf/syscall_ebpf-ebpf/src/main.rs:32` | 把 `info!(&ctx, "[{}] invoke syscall {}", time, syscall_num);` 注释掉. Reason: aya_log → bpf_perf_event_output → ringbuf, 当前丢失. 不能用它当成"通了". |
| `user/ebpf/syscall_ebpf/syscall_ebpf/src/main.rs` | 把"每 5s 打印一次 map iter"改成"运行 30s 后输出 + 写文件 `/tmp/syscall_count.txt`". 测试脚本读文件断言. |
| `test-suit/starryos/ebpf/syscall_count.sh` (新增) | 见 §4 验证脚本 |

**完成判据**:
1. qemu x86_64 / riscv64 内启动 `syscall_count <mangled_sys_getpid>`, 主线触发 100 次 `getpid()`, 30 s 后 `/tmp/syscall_count.txt` 里 `getpid_sysno` 对应 count ≥ 90 ≤ 110.
2. 关闭程序后再起一次, count 重置, 不残留 (kprobe Drop unregister 路径正确, 防 leak).
3. 与 Linux 对照: 同样代码在 host Linux 上跑 (用 mainline aya), count 同范围内.

### 2.2 D3: profile (syscall 频次画像, kprobe + HashMap)

> **实现说明 (2026-05-29, 实跑后定稿)**: 原拟"`__schedule` 注 kprobe + `bpf_get_smp_processor_id` + `PT_REGS_IP` 取 caller PC"的方案**在本内核不可行**, 已改为 syscall 频次直方图. 改动原因 (对代码核实):
> 1. `kbpf_basic::helper::init_helper_functions` **没注册** `bpf_get_smp_processor_id`(8) / `bpf_get_current_pid_tgid`(14); 调用未注册 helper 会让 rbpf VM 报错。可用的只有 map 一族 + `bpf_ktime_get_ns` + `bpf_perf_event_output` + **`bpf_probe_read`(4)** + `trace_printf` + ringbuf。
> 2. kprobe 入口处 `PT_REGS_IP` 恒等于探测点地址本身 (int3 落点), 当 key 只会得到**单一桶**, 直接违反下方"≥3 distinct"判据 —— 拿不到真正的 caller PC (那需要读栈上返回地址, 跨架构脆弱)。
>
> 故采集面改为"整条 syscall 分发路径的 syscall 号频次", 仍是 **kprobe + HashMap** (与 D1 同色, 无 ringbuf 依赖), 但语义是 perf-top-for-syscalls, 区别于 D1 的"单 syscall 精确计数"。

**功能**: 在内核中央分发器 `starry_kernel::syscall::handle_syscall(uctx: &mut UserContext)` 注 kprobe; BPF prog 取 arg0(=`&UserContext`), 用 `bpf_probe_read` 读它的前 8 字节 (x86_64 上即 saved `rax` == syscall 号, 是 `TrapFrame` 首字段, 偏移 0), `HashMap<u32,u64>[sysno]++`。
**用户态**: loader attach 后等信号, SIGTERM/SIGINT 时排空 map、按计数降序打印直方图 + `PROFILE_END total=.. distinct=.. top1_*` 汇总行。

**为何稳**: 仍然 kprobe + HashMap 链路, 与 D1 同色, 无 ringbuf 依赖; 只用已注册 helper (`bpf_probe_read` + map)。syscall 号取值走偏移 0 (无需追结构布局)。

**新增 user crate**: `user/ebpf/profile/` (syscall_ebpf / sched_trace 的兄弟, 3-crate aya workspace)。
- `profile-ebpf/src/main.rs`: `#[kprobe] handle_syscall` → `bpf_probe_read::<u64>(arg0)` → `HashMap<u32,u64>`。无 loop (verifier 友好)。
- `profile/src/main.rs`: 纯 sync aya loader (无 tokio); 把它 attach 到用户传入的 mangled symbol (`grep handle_syscall /proc/kallsyms`)。
- `profile-common/src/lib.rs`: 记 `SYSNO_OFFSET_IN_USERCONTEXT=0` 的 x86_64 ABI 假设。

**内核侧前置 (实跑中发现并修)**: `handle_syscall` 必须 `#[inline(never)]`。release 下 LLVM 把它 inline 进唯一调用点 (`task/user.rs` run loop), 同时仍发独立符号 (kallsyms 命中) —— int3 落在从不执行的离线副本上, 探针**永不触发** (首跑 `total=0`)。与 commit `1f6579f41` 修 `sys_getpid` 的 gap #3 同型, 修在 `kernel/src/syscall/mod.rs`。

**完成判据** (已在 qemu x86_64 实测通过):
1. attach 后跑 syscall 重载 (`dd if=/dev/zero of=/dev/null bs=1 count=20000` 造 2 万 read+2 万 write), 退出时直方图 `total≥1000`(非 `>0`)、`distinct≥3`、top-1 占比 `≥20%`。实测 `total=40310 distinct=33 top1=write(1) 49.6%`。
2. `unregister_kprobe` 验证: 连续 attach/detach 3 次 (每次 detach 经 `write_kernel_text` 改内核文本), 无 panic、样本数稳定 → 卸载路径不泄漏。

### 2.3 D2: sched_trace (依赖 P0 全通)

**功能**: 在 `axtask` switch_to 注 `sched:sched_switch` raw tracepoint; BPF prog 把 `(prev_tid, next_tid, prev_state, ts_ns)` 写入 ringbuf; 用户态 perf buffer reader 实时打印.

**前置**: TP-P0-1 (sched_switch tracepoint 定义) + PERF-P0-1 (perf_fd mmap 通路).

**新增 / 修改**:

| 文件 | 改动 |
|---|---|
| `os/arceos/modules/axtask/src/run_queue.rs:559` `switch_to` | 在 `next_task.set_state(TaskState::Running);` 之后, 实际 `context_switch` 之前, 调一次 `crate_interface` 暴露的 hook (因为 axtask 在 `arceos` 子树, 不能直接 use starry_kernel) |
| `os/StarryOS/kernel/src/tracepoint/sched.rs` (新) | 用 `define_event_trace!(sched_switch, ...)` 定义事件 + 实现 hook impl (`crate_interface::impl_interface`) |
| `os/StarryOS/components/axtask-ext/` 或类似 | 暴露 hook trait (与 task-ext 风格一致), 让 axtask 调 |
| `user/ebpf/sched_trace/` (新, 3 件套) | aya `#[raw_tracepoint(tracepoint = "sched_switch")]` |

**为什么 cross-crate 要走 `crate_interface`**:
- `axtask` 是 ArceOS 模块, 不能依赖 starry-kernel (反向依赖).
- starry-kernel 不能在 axtask switch_to 那里直接 call.
- `crate_interface` 已经是 tgoskits 内 trait-based late-bind 的标准做法 (`components/crate_interface/`, 多处使用).

**完成判据**:
1. qemu x86_64 内, `sched_trace &` 启动后 5 秒内能看到 ≥ 100 次 sched_switch 记录.
2. 关闭程序 → raw tp 闭环 unregister; 等 10s 后内核 trace_pipe 不再出 sched 记录; `cat /proc/<sched_trace_pid>` 不存在 (进程已退).
3. **反 fallback**: 不允许只用 `dmesg | grep sched_switch` 验证. 必须从 user 态拿 ringbuf 数据.

## 3. 内核侧需要的新 hook 点

### 3.1 `sched:sched_switch` (TP-P0-1)

位置: `os/arceos/modules/axtask/src/run_queue.rs:559-595` 之间, 紧贴 `next_task.set_state(Running)`:

```rust
// 伪代码 (axtask 内, 通过 crate_interface 反向调 starry_kernel)
crate_interface::call_interface!(SchedTraceHook::on_switch(
    prev_task.id().as_u64(),
    next_task.id().as_u64(),
    prev_task.state(),   // TASK_RUNNING/TASK_BLOCKED/...
    monotonic_time_nanos(),
));
```

starry-kernel 侧 impl (在 `kernel/src/tracepoint/sched.rs`):

```rust
ktracepoint::define_event_trace!(
    sched_switch,
    TP_kops(crate::tracepoint::KernelTraceAux),
    TP_system(sched),
    TP_PROTO(prev_tid: u64, next_tid: u64, prev_state: u32, ts_ns: u64),
    TP_STRUCT__entry {
        prev_tid: u64, next_tid: u64, prev_state: u32, ts_ns: u64,
    },
    TP_fast_assign { prev_tid, next_tid, prev_state, ts_ns },
    TP_ident(__entry),
    TP_printk({
        format!("prev={} next={} state={} ts={}", __entry.prev_tid, __entry.next_tid, __entry.prev_state, __entry.ts_ns)
    })
);

#[crate_interface::impl_interface]
impl SchedTraceHook for SchedTraceImpl {
    fn on_switch(prev_tid: u64, next_tid: u64, prev_state: u32, ts_ns: u64) {
        trace_sched_switch(prev_tid, next_tid, prev_state, ts_ns);
    }
}
```

### 3.2 `syscalls:sys_clone` raw tracepoint (TP-P0-3)

位置: `kernel/src/syscall/task/clone.rs` 中 `sys_clone` 入口. 写 `define_event_trace!(sys_clone, ...)`, BPF raw tp 通过 `find_ext_tracepoint_by_name("sys_clone")` 就能 attach.

### 3.3 `syscalls:sys_enter_*` 批量补 (TP-P0-4) — 仅在 D2 / 后续需要时做

可选, 不挡 D1/D3.

## 4. 验证脚本 (反 fallback)

### 4.1 D1 syscall_count

`test-suit/starryos/ebpf/syscall_count.sh` (拟):

```sh
#!/bin/sh
set -e

# Find a stable syscall to probe. getpid 是无副作用的最稳选.
SYM=$(grep -m1 'sys_getpid' /proc/kallsyms | awk '{print $3}')
[ -n "$SYM" ] || { echo "FAIL: sys_getpid not in kallsyms"; exit 1; }

# Run loader in bg, attach kprobe.
/usr/bin/syscall_count "$SYM" &
PID=$!

# Wait for attach (loader prints "attach the kprobe to syscall_entry ok").
sleep 2

# Trigger 100 times.
for i in $(seq 1 100); do
    getpid_invoker  # 自带 user app, 直接 syscall(SYS_getpid)
done

# 等 loader 写出 /tmp/syscall_count.txt (30s 周期).
sleep 32
kill $PID 2>/dev/null
wait $PID 2>/dev/null

[ -s /tmp/syscall_count.txt ] || { echo "FAIL: no output"; exit 1; }
COUNT=$(awk -v sym="$SYM" '$0 ~ sym {print $2}' /tmp/syscall_count.txt)
[ -n "$COUNT" ] || { echo "FAIL: sym $SYM not found in map"; exit 1; }

# 容差: 100 ± 10%.
[ "$COUNT" -ge 90 ] || { echo "FAIL: count=$COUNT < 90 (lost samples?)"; exit 1; }
[ "$COUNT" -le 110 ] || { echo "FAIL: count=$COUNT > 110 (double-fire?)"; exit 1; }

echo "PASS syscall_count: $SYM count=$COUNT"
```

**反 fallback 自查**:
- `[ -n "$_t" ]` / `grep -q invoke` 这类弱断言不允许.
- 关掉 BPF prog 后 re-run, 不允许 count 复用旧值.
- 必须断言 `[ "$COUNT" -ge 90 ]` 而非 `[ "$COUNT" -gt 0 ]`.

### 4.2 D3 profile

实测脚本 `test-suit/starryos/ebpf/qemu-smp1/profile/sh/profile.sh` (反 fallback):
- `SYM=$(grep -m1 handle_syscall /proc/kallsyms | awk '{print $3}')` 取 mangled 符号; 找不到直接 FAIL。
- attach 后跑 `dd if=/dev/zero of=/dev/null bs=1 count=20000` 制造主导热点 (write)。
- SIGTERM 让 loader 排空 map、打印 `PROFILE_END total=.. distinct=.. top1_count=..`。
- 断言 (整数判, 不依赖浮点): `total≥1000` (非 `>0`)、`distinct≥3` (反证 "永远只 hit 1 个 key"=kprobe 装错地方)、`top1_count*5≥total` (即 top-1 ≥ 20%, 反证扁平/乱数据)。
- 卸载验证: 再 attach/detach 3 轮, 每轮样本数仍 ≥1000 → 探针未泄漏、内核文本归位。

### 4.3 D2 sched_trace

```sh
/usr/bin/sched_trace > /tmp/sched.out &
PID=$!
sleep 2
# 触发足量 sched_switch: 起一对相互让 CPU 的进程
(yes > /dev/null) & A=$!
(yes > /dev/null) & B=$!
sleep 3
kill $A $B
sleep 1
kill $PID
wait $PID 2>/dev/null

# 应该至少有 100 条 sched_switch (3 秒 × 多次切换/秒)
N=$(wc -l < /tmp/sched.out)
[ "$N" -ge 100 ] || { echo "FAIL: only $N switches captured"; exit 1; }
# 至少应该包含 prev/next 两侧的 tid
grep -q 'prev=' /tmp/sched.out || { echo "FAIL: no prev= records"; exit 1; }
grep -q 'next=' /tmp/sched.out || { echo "FAIL: no next= records"; exit 1; }

echo "PASS sched_trace: $N records"
```

**反 fallback 自查**:
- 不允许把 sched_switch hook 加在错误地方 (例如 timer tick) 让记录数膨胀.
- 不允许在 `cat /sys/kernel/debug/tracing/trace | wc -l` 上断言 — 那走的是 trace_pipe 文本路径, 不验证 ringbuf.

## 5. demo 落点 (rootfs / xtask)

- BPF 程序产物 (`*.ko`-style? 不, 是 ELF): 由 `cargo xtask starry user-ebpf build --program <name> --arch <arch>` 出, 安装到 rootfs `/usr/bin/<name>`.
- 测试脚本: `test-suit/starryos/ebpf/<name>.sh`, 与 busybox-tests.sh 同形态加入 normal QEMU test.
- 验证 group: 单独 `cargo xtask starry test qemu --arch x86_64 -c ebpf` (新 group 名).
- CI: 暂不入主 CI, 先本地手跑; demo 验证脚本稳定 ≥ 5 次后再纳入 .github/workflows/.

## 6. 与 PR-A/B/C/D 的关系

| demo | 依赖的已有 PR | 还需要新 PR |
|---|---|---|
| D1 syscall_count | PR-A (eBPF runtime) + PR-D (user 程序框架) + `user/ebpf/syscall_ebpf` | PR-Demo-1: aya_log 注释 + 验证脚本 + rootfs 安装 |
| D3 profile_kprobe | 同上 + `user/ebpf/syscall_ebpf` 作模板 | PR-Demo-3: 新 user crate + 验证脚本 |
| D2 sched_trace | 上述 + PR-RB-A (PERF-P0 mmap) + TP-P0-1 (sched_switch hook) | PR-Demo-2: sched tp 定义 + user crate + 验证 |

## 7. 风险 / 已知未决

1. **kbpf-basic verifier 限制**: 当前 rbpf 0.4 不做真验证, 错误程序可能直接死循环. demo 程序的 loop 必须能终止 (BPF verifier 在 Linux 上会拒). 本 demo 三个程序都不含 loop.
2. **mangled symbol name**: `sys_getpid` 在 Rust 内核里实际符号可能是 `_ZN12starry_kernel...`. 用户必须用 `cat /proc/kallsyms | grep sys_getpid` 取真名 (`syscall_ebpf` loader 文档已经说了).
3. **profile 不是 sample-based**: D3 既非 `PERF_TYPE_HARDWARE` 采样, 也非"caller PC 直方图" (原设计) —— 见 §2.2 实现说明: `bpf_get_smp_processor_id` / `PT_REGS_IP` 在本内核都拿不到有用值. 实际是"每条 syscall 触发一次 kprobe + 按 syscall 号计数", 即 trigger-based 频次画像 (perf-top-for-syscalls), 与 perf-record 不同色. README 已写明.
4. **多 CPU 下 PerCpuArray 一致性**: aya `mytrace` 用 `PerCpuArray`; 跨 CPU 取 0 号 slot 时, 在 SMP 下记录 mix. demo 程序若有此需求, 必须用 `bpf_get_smp_processor_id` + per-cpu key —— 但该 helper 当前**未注册** (见 §2.2), 故 profile demo 用全局 `HashMap` 而非 per-cpu (smp1 下无歧义; SMP 下计数为各 CPU 之和, 语义仍正确).
5. **profile 架构相关性**: syscall 号取值假设 `&UserContext` 偏移 0 == syscall-号寄存器 (x86_64 `rax`). 其它架构 `UserContext`/`TrapFrame` 首字段不同时需调 `profile-common::SYSNO_OFFSET_IN_USERCONTEXT` 再验 (见 §8 矩阵未勾项)。

## 8. 验证矩阵 (汇总)

| demo | qemu x86_64 | qemu riscv64 | qemu aarch64 | qemu loongarch64 |
|---|---|---|---|---|
| D1 | 必跑 | 必跑 | 必跑 | 与 PR-A loongarch64 fix 同步 |
| D3 | 必跑 | 必跑 | 选跑 | 选跑 |
| D2 | 必跑 (依赖 PR-RB-A) | 必跑 | 选跑 | 选跑 |

所有 demo: 每次 push 前在本地 docker 跑一次完整 §4 验证脚本.
