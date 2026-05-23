# Tracepoint 支持审计

> 审计时点: 2026-05-23, 分支 `feat/starry-ebpf-userspace` @ `2f2533968`
> (PR-A eBPF runtime + PR-D 用户态程序的并集).
> 上游对照: Linux 6.x 的 `Documentation/trace/tracepoints.rst` /
> `events/syscalls/`, busybox-style userspace; 同行项目 `Starry-OS/StarryOS:ebpf-kmod`.

## 1. 现状 (按"哪些是真的"分类)

### 1.1 已工作的部分

| 维度 | 现状 | 证据 (代码位置) |
|---|---|---|
| `ktracepoint` crate 接入 | `kbpf-basic 0.5` + `ktracepoint = "0.6"` | `os/StarryOS/kernel/Cargo.toml:138` |
| 全局 init | `tracepoint_init()` 收集所有 `define_event_trace!` 静态注册项, 把 `ExtTracePoint` 装进 `TRACE_STATE.ext_tracepoints: BTreeMap<u32, Arc<Mutex<ExtTracePoint>>>` | `kernel/src/tracepoint/mod.rs:234` |
| Static-key 启用机制 | 由 ktracepoint 内部维护; `ExtTracePoint::register(TraceCallbackType::Event)` 加入第一个 callback 时自动打开 | `kernel/src/perf/tracepoint.rs:108-120` (注释明确说明 enable/disable 是 no-op) |
| `debugfs/tracing/events/<sys>/<event>/{enable,format,id,filter}` | 由 `init_events()` 自动构建 | `kernel/src/tracepoint/mod.rs:255-323` |
| `tracefs` 根目录文件: `trace_pipe`, `trace`, `saved_cmdlines`, `saved_cmdlines_size` | 全部挂上 | `kernel/src/tracepoint/mod.rs:328-368` |
| `trace_pipe` 阻塞读 | `block_on(interruptible(poll_fn))` + `PollSet::register(waker)` | `kernel/src/tracepoint/trace_pipe.rs:38-60` |
| `trace_pipe` 跨多次 read_at 不丢前缀 | `TextDrain` 保存未读尾巴 | `kernel/src/tracepoint/mod.rs:130-198` |
| BPF tracepoint 程序 attach (Linux `perf_event_open(PERF_TYPE_TRACEPOINT)`) | 走 `perf_event_open_tracepoint` → `lookup_ext_tracepoint(id)` → `ExtTracePoint::register(TraceCallbackType::Event(TraceEventFunc))` | `kernel/src/perf/tracepoint.rs:143-150` |
| BPF raw tracepoint attach (Linux `bpf(BPF_RAW_TRACEPOINT_OPEN)`) | `find_ext_tracepoint_by_name` + `TraceCallbackType::RawEvent(RawTraceEventFunc)` | `kernel/src/perf/raw_tracepoint.rs:114-120` + `kernel/src/ebpf/mod.rs:215-218` |
| `cmdline cache` (`saved_cmdlines`) | `TraceCmdLineCache::new(4096)`; 每次 trace 触发时 `trace_cmdline_push(pid)` 把当前进程的 `exe_path` basename 塞进去 | `kernel/src/tracepoint/mod.rs:88-100` |

### 1.2 实际注册的 tracepoint 全集 (致命狭窄面)

`grep -rn 'ktracepoint::define_event_trace!' os/StarryOS/kernel/` 现状:

| # | 事件名 | system | 位置 | 触发时机 |
|---|---|---|---|---|
| 1 | `sys_enter_openat` | `syscalls` | `kernel/src/syscall/fs/fd_ops.rs:140` | `sys_openat` 入口 |
| 2 | `sys_mkdirat` | `syscalls` | `kernel/src/syscall/fs/ctl.rs:92` | `sys_mkdirat` 入口 |

**就这两个**。

参考 Linux 6.x 内置事件数量级:
- `syscalls:sys_enter_*` × ~400 (每个 syscall 一个 enter + 一个 exit)
- `sched:sched_{switch,wakeup,wakeup_new,migrate_task,process_fork,process_exit,...}` × ~20
- `irq:{irq_handler_entry,softirq_entry,...}`, `timer:*`, `kmem:*`, `block:*`, `net:*` … 数百

→ 我们当前覆盖率 < 1%。

### 1.3 用户态 eBPF 程序对 tracepoint 的依赖矩阵

源: `os/StarryOS/user/ebpf/<prog>/<prog>-ebpf/src/main.rs`.

| program | 类型 | attach 目标 | 现状 |
|---|---|---|---|
| `mytrace` | `#[tracepoint]` aya | `syscalls:sys_enter_openat` (perf_event_open by tp_id) | ✅ tracepoint 存在; ⚠️ aya_log 走 ringbuf, 见 perf 审计 §2 |
| `rawtp` | `#[raw_tracepoint(tracepoint = "sys_clone")]` aya | `sys_clone` (raw tp by name) | ❌ tracepoint **不存在**, `find_ext_tracepoint_by_name("sys_clone") → None`, attach 会得 EINVAL |
| `kret` | `#[kretprobe]` aya | 用户传 mangled symbol | ✅ 与 tracepoint 无关 (走 kprobe) |
| `syscall_ebpf` | `#[kprobe]` aya | 用户传 mangled symbol | ✅ 同上 |
| `upb` / `upb2` | `#[uprobe]` aya | 用户函数 / musl libc 函数 | ❌ uprobe 内核侧 `Unsupported` (`kernel/src/perf/uprobe.rs:22`), 与 tracepoint 无关 |
| `async_test` | 非 eBPF, 仅 `core::arch::breakpoint` | n/a | ✅ |

→ 仅 `mytrace` 一个程序能命中现有 tracepoint, 且仍受限于 aya_log → ringbuf 路径未通.

## 2. 与 Linux 语义的差距 (按可见用户面)

### 2.1 控制面 (`/sys/kernel/debug/tracing/`)

| Linux 文件 | tgoskits 现状 | 差距 |
|---|---|---|
| `events/<sys>/<event>/enable` | ✅ `EventEnableObj` (control.rs:11) | 仅接受 `"0"` / `"1"`, 不接受 `"*"` (一组事件同时 enable) |
| `events/<sys>/<event>/format` | ✅ `TracePointFormatFile` | 取决于 ktracepoint 0.6 输出, 未与 Linux `format` 文本逐字节比对 |
| `events/<sys>/<event>/id` | ✅ `TracePointIdFile` | OK |
| `events/<sys>/<event>/filter` | ✅ `EventFilterObj` + `TraceFilterFile::write` | 未审计 filter DSL 与 Linux 的语义对齐度 |
| `events/<sys>/enable` (整组 enable) | ❌ 不存在 | 必须逐 event 写 1 |
| `events/enable` (全局) | ❌ 不存在 | 同上 |
| `events/header_page` | ❌ 不存在 | `libtraceevent` 解析记录时会读这个; 当前 tgoskits 直接走文本 trace_pipe, 不走 perf-style 二进制 buffer, 暂可忽略 |
| `tracing_on` | ❌ 不存在 | 全局开关 — 用户态调试时常用 `echo 0 > tracing_on` 暂停 |
| `current_tracer` / `available_tracers` | ❌ 不存在 | ftrace nop/function/function_graph 全无, 我们只做 tracepoint, 暂可标注为 "out of scope" |
| `set_event` | ❌ 不存在 | 用 `subsystem:event` 一次写多个 |
| `trace_pipe` | ✅ blocking read 可用 | 缺 `poll(2)` 集成 → DirectRwFsFileOps 没暴露 fd-level poll, libbpf-style nonblocking + epoll 用户态会一直 EAGAIN |
| `trace` | ✅ snapshot 形式可读 | OK |
| `per_cpu/cpu*/trace_pipe_raw` | ❌ 不存在 | libbpf 的 `perf_buffer__poll` 在没有 ringbuf 时走这个; 我们没有 |
| `saved_cmdlines` / `saved_cmdlines_size` | ✅ | OK |

### 2.2 数据面 (从内核到用户的传递通路)

```
[tracepoint fire]
    │
    ├─ trace_pipe_push_raw_record (mod.rs:78) ──► raw_pipe.lock().push_record  ──► trace_pipe 文本输出
    │
    └─ ExtTracePoint::register 装入的 callback list:
         ├─ TraceCallbackType::Event(TraceEventFunc)    — BPF tp prog (perf_event_open)
         └─ TraceCallbackType::RawEvent(RawTraceEventFunc) — BPF raw tp prog (BPF_RAW_TRACEPOINT_OPEN)
```

差距点:
- **BPF prog 输出回用户态**: aya_log 在 BPF prog 里通过 `bpf_perf_event_output` helper 写 ringbuf. 当前 kernel 端 `bpf_perf_event_output` → `BpfPerfEventWrapper::write_event` → 因 `phys_addr.is_none()` 静默丢弃 (`perf/bpf.rs:52-65`). 见 [perf-ringbuf 审计](perf-ringbuf-audit.md).
- **trace_pipe fd poll**: 用户态 `epoll_wait` 监听 `trace_pipe` 的常用场景拿不到就绪事件. 仅阻塞 `read(2)` 工作.
- **filter 语义**: ktracepoint `TraceFilterFile::write` 的 DSL 与 Linux 不一定一致, 未对照过.

### 2.3 ABI 层 (syscall / bpf cmd)

| syscall / bpf cmd | tgoskits 状态 | 备注 |
|---|---|---|
| `perf_event_open(2)` `PERF_TYPE_TRACEPOINT` | ✅ `perf/tracepoint.rs` | OK |
| `perf_event_open(2)` `PERF_TYPE_KPROBE` | ✅ `perf/kprobe.rs` | OK |
| `bpf(BPF_RAW_TRACEPOINT_OPEN)` | ✅ `ebpf/mod.rs:215` → `perf/raw_tracepoint.rs:114` | OK |
| `bpf(BPF_PROG_LOAD)` for `BPF_PROG_TYPE_TRACEPOINT` / `_RAW_TRACEPOINT` | ✅ 走通 kbpf-basic verifier | 未跑过最小 attach 闭环 (PR-A 编译至今卡在 printf-compat) |
| `bpf(BPF_PROG_TYPE_TRACING)` (`fentry`/`fexit`, BTF 路径) | ❌ | 与 BTF 关联, 远期 |
| `PERF_EVENT_IOC_SET_BPF` ioctl | ✅ `perf/mod.rs:122` | OK |
| `PERF_EVENT_IOC_ENABLE/DISABLE` ioctl | ✅ | OK |

## 3. 残缺清单 (按补完优先级)

### P0 — 任意 demo 都需要

| ID | 残缺项 | 影响 | 修复方向 |
|---|---|---|---|
| TP-P0-1 | `sched:sched_switch` tracepoint 未定义 | 无法做调度追踪 demo | 在 `os/arceos/modules/axtask/src/run_queue.rs:559` `switch_to()` 内加 `define_event_trace!(sched_switch, …)` + 调用点; 注意 axtask 是 `arceos` 内部模块, 不能直接 `use starry_kernel::tracepoint`, 需要在 starry-kernel 那侧 hook (见 demo 设计 §3.1) |
| TP-P0-2 | `sched:sched_process_fork` / `sched_process_exit` 未定义 | 调度追踪 demo 想关联进程命名时缺数据 | 同上, hook 点在 `kernel/src/task/clone.rs` exit/clone 路径 |
| TP-P0-3 | `syscalls:sys_clone` raw tracepoint 未定义 | `user/ebpf/rawtp` attach 直接 EINVAL | 在 `kernel/src/syscall/task/clone.rs` 入口加 `define_event_trace!(sys_clone, ...)` (raw tracepoint 用同一注册表; 无需额外形式) |
| TP-P0-4 | `syscalls:sys_enter_*` / `sys_exit_*` 只有 `openat` + `mkdirat` 两个 | syscall 计数 demo 想覆盖广面 syscall 时极受限 (但 syscall_ebpf demo 走 kprobe + kallsyms, 不依赖 tracepoint, 实际可绕开) | 优先补 `sys_enter_write`, `sys_enter_read`, `sys_enter_execve`, `sys_enter_clone`, `sys_enter_exit_group`; 用 `define_event_trace!` 批量, 或写一个 `tp_syscall!` 包装宏 |

### P1 — 提升可观察性 / 与 Linux 对齐

| ID | 残缺项 | 影响 | 修复方向 |
|---|---|---|---|
| TP-P1-1 | `trace_pipe` 不暴露 fd-level poll | userland nonblocking 读不可用 | 把 `TracePipeFile` 改成 `FileLikePollOps`-style: 注册到 `TRACE_STATE.pipe_event` 上, 让 `poll(2)`/`epoll` 能 wake. 现有 `PollSet` 已是这套机制, 只是 DirectRwFsFileOps 走的不是 FileLike 路径 |
| TP-P1-2 | `tracing_on` 全局开关缺失 | 难做"先 attach, 再统一启" | 在 `pseudofs::debug` 加 `tracing_on` 文件; 写 0/1 调 ktracepoint static-key 全局闸 (需查 ktracepoint 0.6 是否暴露 API; 若无, 在 `TraceState` 加一个 `AtomicBool` 把 `trace_pipe_push_raw_record` 包一层) |
| TP-P1-3 | `events/<sys>/enable` 组级开关缺失 | 与 Linux 工具链 (`trace-cmd`, `bpftrace`) 不兼容 | 在 `init_events` 的 `subsystem_root` 加一个 `enable` SpecialFsFile, 写时 fan-out 到该 subsystem 下所有 ExtTracePoint |
| TP-P1-4 | filter 语义未对齐 | bpftrace `WHERE` 子句不可用 | 写一份 ktracepoint 0.6 filter DSL → Linux filter DSL 的兼容性表, 不一致处补适配 |
| TP-P1-5 | 没有架构无关的 ASM-level tracepoint patching | 静态 key 命中开销在 disabled 路径仍有 (虽 ktracepoint 已用 static-keys) | 用 `static_keys::global_init()` (entry.rs 已调) 已经 OK; 但要核对 4 架构 (尤其 loongarch64) 是否真的能 patch — 跑 qemu 时观察启用前后 nop sled |

### P2 — 远期 / 可选

| ID | 残缺项 | 影响 |
|---|---|---|
| TP-P2-1 | `perf_event_attr.type = PERF_TYPE_HARDWARE/HW_CACHE/RAW` | 仅 SOFTWARE/TRACEPOINT/KPROBE/UPROBE 命中, perf-like profile 用 PMU counter 路径走不通 |
| TP-P2-2 | BTF-driven tracing | `BPF_PROG_TYPE_TRACING` (`fentry`/`fexit`) 不可用 |
| TP-P2-3 | `events/header_page` / `per_cpu/cpu*/trace_pipe_raw` 二进制 buffer | `libtraceevent` / `trace-cmd` 兼容 |

## 4. 反 fallback 检查清单 (写代码时自查)

为了避免"加了一个 trace 点就以为支持完整", 任何 P0/P1 修复必须满足:

1. **覆盖完整生命周期**: enable/disable 都过 → 在 `events/<sys>/<event>/enable` 写 `0` 再读 `trace_pipe` 必须停止产出新记录 (反向证伪 "register 不带 unregister" 的脏实现).
2. **不能只通过弱断言验证**: 不允许 `grep -q "<词>" trace`. 必须断言记录数 ± 容差 (例如 `for i in $(seq 1 100); do echo > /dev/null; done; n=$(wc -l < /tmp/trace); [ $n -ge 90 ] && [ $n -le 110 ]`). 见 [WORKFLOW.md §6.2](WORKFLOW.md).
3. **不能只在 host build 通过**: 必须 qemu x86_64 + qemu riscv64 各跑一遍 (loongarch64 留给后续, 见 P1-5).
4. **不能在 BPF prog 端用 `info!` (aya_log) 当成"看见了"**: aya_log 走 ringbuf, ringbuf 路径未通 (见 perf 审计), `info!` 会被静默丢弃. 用 HashMap iter 或 trace_pipe 验证.

## 5. 验证脚本骨架 (用于 demo + CI)

在 qemu 内跑 (rootfs 需提前 mount `debugfs`):

```sh
# (A) tracepoint 注册可见性
ls /sys/kernel/debug/tracing/events/syscalls/
# 期望: sys_enter_openat/ sys_mkdirat/ (P0-3/P0-4 后追加更多)

cat /sys/kernel/debug/tracing/events/syscalls/sys_enter_openat/format
# 期望非空, 含 "name: sys_enter_openat" 与 field 列表

# (B) enable / disable 闭环
echo 1 > /sys/kernel/debug/tracing/events/syscalls/sys_enter_openat/enable
( cat /sys/kernel/debug/tracing/trace_pipe & ) >/tmp/tp.log
sleep 0.5
for i in 1 2 3; do cat /etc/passwd > /dev/null; done
sleep 0.5
echo 0 > /sys/kernel/debug/tracing/events/syscalls/sys_enter_openat/enable
n=$(wc -l < /tmp/tp.log)
[ "$n" -ge 3 ] || { echo "FAIL: tp produced < 3 records ($n)"; exit 1; }
echo "PASS tp open enable+disable ($n records)"

# (C) BPF tracepoint attach (依赖 PR-D `mytrace`)
./mytrace &
sleep 1
for i in 1 2 3; do cat /etc/hostname > /dev/null; done
# 验证方式: TRACE_PIPE 输出含 sys_enter_openat (aya_log 不通时不要用 info! 验证)
```

## 6. 后续动作映射

| 任务 | 详见 |
|---|---|
| 写 sched_switch hook | demo-stack-design.md §3.1 |
| 写 sys_clone raw tp | demo-stack-design.md §3.2 |
| ringbuf mmap 接通 (用于 aya_log) | perf-ringbuf-audit.md §3 |
| 反 fallback 流程 | WORKFLOW.md §5–§6 |
