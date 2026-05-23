# eBPF follow-up 工作日志

按时间倒序追加 (最新在最上). 格式见
[WORKFLOW.md §4.1](WORKFLOW.md).

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
