# OS Biglab 总结报告

**作者**: 王鹏杰 (Joseph Joshua)
**日期**: 2026 年 6 月 2 日

## 1. 概述

本报告总结在 StarryOS（基于 ArceOS 模块构建的 Linux 兼容内核）上为期八周的内核开发工作。核心产出为：构建了一套 AI 驱动的内核开发框架（harness），完成了多线程 execve、文件锁、BusyBox 组件适配等内核功能支持，以及将 eBPF 运行时和 LKM（Loadable Kernel Module）机制从独立分支迁移至 tgoskits 主线。

工作主线如下：

```
BigLabA (实验基础) ──→ BigLabB (实验框架)
      │                      │
  5 个基础实验           tg-arceos-tutorial
  个性化实验教程         5 个练习层次
  扩展实验实践

EXP2 (内核功能支持) ──→ EXP3 (应用支持) ──→ EXP4 (eBPF/LKM)
      │                      │                     │
  多线程 execve (#273)   BusyBox 适配          eBPF 运行时 (#850)
  文件锁 (#472)          4 个 PR               LKM 加载器 (#851)
                                               内核模块示例 (#880)
                                               用户态 eBPF 程序 (#886)
```

贯穿所有实验的核心基础设施是 **AI 驱动的内核开发框架**（EXP1），该框架以 Designer / Developer / Reviewer / Auditor 四角色流水线组织工作，使 AI 代理在严格的工程护栏下参与内核开发。

---

## 2. EXP1: AI 驱动的内核开发框架

### 2.1 动机

内核开发有三个核心痛点：

**测试反馈周期长。** 内核修改后在 QEMU 中启动 StarryOS 并运行测试用例，单次迭代（build + rootfs + qemu + test）需要数分钟。在发现、修复、验证的循环里，大量时间耗在等待编译和启动上。

**Linux 对照成本高。** 判断 StarryOS 的一个行为是否正确，需要在 Linux 上编译运行相同的测试程序，阅读 man page 确认预期行为，有时还需阅读 Linux 内核源码确认实现细节。

**重复性工作多。** 每个 bug 的修复流程高度相似（写测试、跑 Linux 对照、改内核、跑 StarryOS 验证、写报告），但随着修复数量增加，状态追踪和报告维护的负担线性增长。

### 2.2 架构：四角色流水线

框架基于 Claude Code 的技能（Skills）和代理（Agents）插件架构，将内核开发流程拆分为四个角色：

```
Designer ──→ Developer ──→ Reviewer ──→ Auditor
   │              │             │             │
 接受任务       执行工作流     本地测试      审计 workflow
 阅读代码库     更新状态       寻找 bug      审计 PR
 确认上游重叠   记录完成情况   确认对齐      确认意图实现
```

**Designer（设计者）**：接受任务，阅读代码库，确认上游重叠。核心技能：`workflow-design`、`upstream-overlap-check`。

**Developer（开发者）**：执行工作流，更新状态并记录完成情况。核心技能：`kernel-quality-review`、`linux-compare`（当 Linux 兼容性被要求时）。

**Reviewer（审查者）**：本地测试，寻找 bug，确认和 Linux/POSIX/Unix 对齐。核心技能：`multilayer-test`、`bug-triage`、`linux-compare`、`concurrent-bug-checker`、`misalignment-checker`、`cross-syscall-bug-checker`、`kernel-quality-review`、`ci-monitor`。

**Auditor（审计者）**：审计 workflow，审计 PR，确认是否实现最初的意图。核心技能：`audit`、`kernel-quality-review`、`bug-triage`、`upstream-overlap-check`。

### 2.3 技能分配

框架包含 9 个核心技能和 3 个专业代理，构成 16 个基础设施脚本。技能涵盖从 PR 发现到提交的全流程：

| 技能 | 用途 |
|------|------|
| `hunt-bugs` | 发现 → 测试 → 对比 → 修复 → 报告 主循环 |
| `test-app` | Linux 应用兼容性测试 |
| `benchmark` | 性能基准测试 |
| `audit-kernel` | 内核内部审计（锁顺序、并发、内存泄漏） |
| `review-quality` | 代码质量门禁 |
| `check-upstream` | 上游 PR 去重检查 |
| `start-submission` | 准备 PR 提交 |
| `evolve` | 自主目标选择 + 持续开发循环 |
| `report` | 结构化报告生成 |

### 2.4 工程护栏

三道核心工程护栏确保质量：

**结构化输出强制。** 所有 AI 代理通过 JSON Schema 返回可验证的结果，使流水线中的每个步骤输出可以被后续步骤无歧义消费。

**状态记忆。** `known.json` 作为所有已发现缺陷的单一事实来源，维护状态机以防止重复修复，可以随时生成进度统计和分类报告。

**审查者否决权。** `review-quality` 和 `kernel-reviewer` 代理独立于修复代理运行。它们的使命是找问题，不是批准。审查维度包括：修复后行为是否精确匹配 Linux、是否存在 TOCTOU 窗口或 UAF 风险、锁获取顺序是否一致、错误路径上是否正确回滚。审查失败意味着修复必须重新修改，直到所有维度通过。

### 2.5 Demo：用 Harness 重新解决 EXP2

以多线程 execve 问题为例，使用 glm5.1 和 deepseek-v4 两个模型，在 Harness 框架下完成修复。Reviewer 审查 4 轮，Auditor 审查 2 轮，展示了框架在复杂并发问题上的能力。

---

## 3. EXP2: 内核功能支持

### 3.1 多线程 execve [#273](https://github.com/rcore-os/tgoskits/pull/273)

**问题**：此前 `sys_execve` 在调用进程有多个线程时会直接返回 `EWOULDBLOCK`，任何多线程父进程调用 `execve` 都会失败——包括 `rustc`/LLVM、`cargo` 的进程 spawn，以及任何从线程池驱动 `std::process::Command` 的程序。

**实现**：分两阶段处理。第一阶段是可失败的：构建新的地址空间（路径解析 + ELF 加载）但不提交。第二阶段是不可逆的：杀死所有 sibling 线程，提交新地址空间。

**关键设计决策**：

1. **并发 execve 序列化**：通过 per-process `exec_lock` 序列化。锁等待是 yield-loop + `exit_request` 探测，匹配 Linux 的 "killable but not signal-interruptible" 语义。
2. **CLOEXEC 快照时机**：在 sibling teardown 之后快照 CLOEXEC fd，确保迟到的 `fcntl(F_SETFD)` / `open(O_CLOEXEC)` 不会丢失。
3. **Non-leader execve**：通过 `de_thread` leader transfer 实现。调用者将其 `Thread::tid` 重命名为 TGID，重新映射全局 task table、signal child list 和 `proc.tg.threads`，使 `gettid() == getpid()` 在新映像中成立。
4. **信号重置**：匹配 Linux 的 `flush_signal_handlers` + `do_execveat_common` 语义，自定义处理函数恢复为 `SIG_DFL`，显式 `SIG_IGN` 的信号保留。

**开发历程**：该特性经历了 5 次 rebase（从 4 月 25 日到 4 月 27 日，每次保留备份分支），5 月 9-12 日完成核心实现（包括 de_thread leader transfer 和 NULL argv/envp 处理），5 月 15-19 日进行并发 bug 修复（信号处理竞态、robust-futex TID 对齐等），最终于 5 月 20 日合入主线。

**评审迭代**：Reviewer 周睿老师在第一次提交时指出了大量问题，包括 `try_lock` 并发语义不够细粒度、CLOEXEC 的快照时机错误、vfork 的睡眠要能够被 zap 打断等。这些问题可归纳为两类：并发 bug 和跨 syscall 交互。

**已解决**：
- 在可失败阶段破坏线程组
- execve 并发的阻塞语义更细粒度
- CLOEXEC 的竞态条件
- vfork 相关阻塞
- `execve(path, NULL, NULL)` 对齐 Linux 标准
- 多线程 execve 成功回归测试充分

**遗留问题**：
- execve 后没有 `do_thread`，因此不恒定满足 `gettid() == getpid()`

### 3.2 文件锁 [#472](https://github.com/rcore-os/tgoskits/pull/472)

**问题**：`sys_fcntl` 的所有 advisory lock 命令（`F_SETLK` / `F_SETLKW` / `F_GETLK` 及对应的 `F_OFD_*`）以及 `sys_flock` 全部返回 `Ok(0)` 而不实际加锁。依赖文件锁做并发协调的软件（dpkg、sqlite、postfix、nginx pid file 等）行为不可预测。

**实现**：新增 `lock.rs`，作为完整的 advisory lock 子系统。维护 `FCNTL_LOCKS` 和 `FLOCK_LOCKS` 两类锁表，均以 `(device, inode)` 为 key，两张表互不影响。

锁归属设计：
- **POSIX 锁**：owner = pid
- **OFD 锁**：owner = open file description，用 `Arc::as_ptr` 作身份指纹，持 `Weak` 用于 close 检测
- **flock 锁**：owner 同 OFD

范围语义：half-open `[start, end)`，`l_len == 0` 表示"到文件尾"（存为 `i64::MAX`）。同 owner 设新锁前先移除/分裂旧区间再插入。OFD 自动释放：`Weak::strong_count() == 0` 即剪枝。

**评审迭代**：Reviewer 周睿老师指出的问题包括：(1) 与 Linux/POSIX 语义不一致，如负的 `l_len`、唤醒和返回值模式；(2) 锁的问题，如子进程退出时失败路径的回滚、一致性。总结问题：(1) 语义对齐；(2) 并发 bug；(3) 失败路径处理。

**代码量**：+3201 / -18 行，涉及 41 个文件。

---

## 4. EXP3: BusyBox 应用功能支持

StarryOS 中 Linux 应用的功能支持通过 BusyBox 组件测试验证。BusyBox 工作横跨约 5 周（4 月下旬至 5 月下旬），约 48 个非合并提交，最终达到 **320 PASS / 0 FAIL** 的测试覆盖（覆盖 riscv64、aarch64、x86_64、loongarch64 四个架构）。

测试进展：

```
初始覆盖 → 282 PASS (#378) → 283 PASS (#722) → 302 PASS (#668) → 320 PASS (#993)
```

### 4.1 开发阶段

**Phase 1 — 基础覆盖（4 月下旬至 5 月上旬）**：TTY 修复、初始测试注入、mkdir 修复、tmpfs 硬链接修复。

**Phase 2 — vfork/exec 和 procfs（5 月 7-9 日）**：PR #377 实现 vfork + 修复 CLONE_VM 下的 execve（对 BusyBox daemon 至关重要）；PR #452 实现 `/proc/stat`、`/proc/cpuinfo`、`/proc/uptime`，修复 `/proc/meminfo` 和 `sysinfo()`。

**Phase 3 — 网络/块设备（5 月 10-12 日）**：块设备支持（arch/blkid/blkdiscard/blockdev + loop）、busybox_ipaddr/iplink、ARP 表、arping、nice 优先级 syscall 等大量 PR。

**Phase 4 — 大规模扩展（5 月 16-18 日）**：PR #668 为关键提交——新增 `/proc/net/dev`、socket ioctl for ifconfig/ifenslave、ICMP loopback echo reply、21 个新 applet 测试，一次将测试数从 283 提升至 302。

**Phase 5 — 特定 applet 深入（5 月 18-24 日）**：6 个并行 feature branch 分别瞄准特定 applet。

**Phase 6 — 稳定化（5 月 25-28 日）**：修复 SIGSTOP（挂起而非杀死）、时间控制与快速失败、PR #993 新增 7 个高副作用 applet 的安全失败覆盖（insmod、fdflush、raidautorun、killall5、rdev、setlogcons、resize）。

### 4.2 重点 applet 分析

#### busybox_acpid [#722](https://github.com/rcore-os/tgoskits/pull/722)

**问题**：acpid 会把自己变成后台进程并关闭输出。

**解决**：通过 usage banner 验证 acpid 可被正确调用。

#### busybox_add_shell [#751](https://github.com/rcore-os/tgoskits/pull/751)

**问题**：`/bin/shell` 已经存在时静默退出。

**解决**：实现真实的 `/etc/shells` 重写路径测试。初始版本使用 `--help` banner 探测（`busybox_add_shell` 分支），后改为真实的 `/etc/shells` 重写 round-trip 测试（`busybox_add_shell_realfs` 分支）。

#### busybox_crond [#741](https://github.com/rcore-os/tgoskits/pull/741)

**问题**：之前让 crond 在前台运行（`-fc` 参数），但实际上 crond 可以在后台运行。

**解决**：重写测试，让 crond 真正 daemonize 并验证 cron 任务实际执行。这是 vfork (#377) 的直接受益者——daemonize 需要 fork 后父进程退出、子进程继续。

#### busybox_crontab [#750](https://github.com/rcore-os/tgoskits/pull/750)

**问题**：创建文件失败，但返回 0 说自己运行成功。根因是内核将 `O_TRUNC` 和 `O_APPEND` 视为冲突的标志（`ax-fs-ng` 中的 `open flags` 检查），而 POSIX/Linux 允许两者同时设置。

**解决**：修复 `ax-fs-ng` 中 `O_TRUNC | O_APPEND` 的错误拒绝，并重写了更严格的测试（先添加、再 ls、最后删除）。

#### busybox run-parts [#517](https://github.com/rcore-os/tgoskits/pull/517)

**问题**：`run-parts` 执行脚本时，如果脚本不是 ELF 格式（如 shell 脚本），execve 失败后直接报错。

**解决**：在 execve 中增加 fallback——非 ELF 文件尝试通过 `/bin/sh` 执行。

### 4.3 暴露的内核缺陷

BusyBox 测试暴露了多个内核层面的 bug：

| 缺陷 | PR | 根因 |
|------|-----|------|
| `O_TRUNC \| O_APPEND` 被拒 | #750 | `ax-fs-ng` open flags 检查过严 |
| 非 ELF 脚本无法执行 | #517 | execve 缺少 `/bin/sh` fallback |
| daemonize 失败 | #377 | vfork + CLONE_VM execve 未正确实现 |
| SIGSTOP 杀死进程而非挂起 | #925 | 信号处理实现错误 |
| procfs 数据缺失 | #452, #668 | 多个 `/proc` 条目未实现 |
| 网络工具不可用 | #668 | `/proc/net/dev`、socket ioctl 缺失 |

### 4.4 提 PR 前的审计流程

设计了四条审计规则：

1. **是否绕开了真实的路径？** — 确保测试覆盖实际执行路径
2. **BusyBox 是否在偷偷 Fallback？** — 内核报错后上层应用是否装作没事发生
3. **行为是否与原生 Linux/POSIX 完美对齐？** — 在标准 Alpine 环境下跑，输出和返回码是否完全一致
4. **新增内核代码是否有伪装的 stub 实现？** — 确保不是空壳

---

## 5. EXP4: eBPF 功能支持

### 5.1 Stage 1: eBPF 运行时迁移 [#850](https://github.com/rcore-os/tgoskits/pull/850)

将 eBPF 运行时从独立仓库 `Starry-OS/StarryOS:ebpf-kmod` 迁移至 tgoskits 主线。完整的数据流为：

```
bpf() syscall → BpfMap / BpfProg → perf_event_open → kprobe / tracepoint → rbpf 执行
```

**变更内容**：

**ebpf/ 子模块**（替换 #805 的单文件 stub）：
- `mod.rs`：`sys_bpf` 真分派，调入 `kbpf-basic` 的 map_create / prog_load 等操作。显式实现 `BpfError ↔ AxError` 边界（`kbpf-basic` 的 `axerrno` 与 tgoskits 的 `ax-errno` 是不同 crate）。
- `map.rs`：`BpfMap` FileLike + `PollSetWrapper`。
- `prog.rs`：`BpfProg` FileLike，drop 时释放 preprocessor 暂存的 map `Arc`。
- `transform.rs`：实现 `KernelAuxiliaryOps`（perf_event_output / copy_from_user 等）与 `PerCpuVariantsOps`。

**perf/ 子模块**（全新）：
- `mod.rs`：`PerfEvent` FileLike，按 `PerfTypeId` 分派 kprobe / software / tracepoint / uprobe。
- `bpf.rs`：ringbuf write_event + `OwnedEbpfVm`（rbpf 解释器 + `Arc<BpfProg>` 一体）。
- `kprobe.rs`：Kprobe/Kretprobe，set_bpf_prog 时构建 `OwnedEbpfVm` 并注册回调。
- `tracepoint.rs`：适配 ktracepoint 0.6 新 API。

**代码量**：+1949 / -2054 行。

**开发历程**：
- 基础设施由 `feat/ebpf-integration-base` 分支整合，合并了 `pr-673-tp`（tracepoints）和 `pr-805-ebpf-observability` 两个前置 PR。
- 核心迁移提交 `d7a9818f5`（5 月 21 日）后经历了多轮评审迭代（5 月 22 日解决编译错误、5 月 26 日清理 clippy、5 月 31 日评审驱动的重构）。
- 5 月 31 日合入 `dev` 分支。

### 5.2 LKM 支持 [#851](https://github.com/rcore-os/tgoskits/pull/851)

实现 Loadable Kernel Module 机制，接受用户态 `.ko` 文件，解析内容并注册到 `MODULES` 表。

**三个 syscall**：`init_module`、`finit_module`、`delete_module`。

**构建系统**：借助 `cargo xtask` 系统实现 `cargo xtask starry kmod build` 构建链，将 Rust 模块编译为 `.ko`。

**取代旧实现 #849**：

| | #849（旧） | #851（新） |
|---|---|---|
| `resolve_symbol` | 桩，恒返回 `None` | 走 `kallsyms` 真实解析 |
| `finit_module` / `delete_module` | 桩，恒返回 `Unsupported` | 真实实现 |
| 用户态内存拷贝 | 裸 `from_raw_parts` | 经 `VmBytes` / `vm_load_string` 正规拷贝 |
| `printk` 等 C-ABI shim | 无 | 经 `lwprintf-rs` 实现 |
| 构建 `.ko` | 无 | `cargo xtask starry kmod build` 流水线 |
| 模块卸载 | `mem::forget`，不可卸载 | 注册表持有，`delete_module` 可卸载 |

**开发历程**：5 月 21 日初始移植后，经历了大量修复——编译错误、section 权限处理、errno 传播、printk varargs 转发、ELF partial link、ax-errno crate 冲突解决等。截至 6 月 2 日仍在活跃开发中。

### 5.3 内核模块示例 [#880](https://github.com/rcore-os/tgoskits/pull/880)

两个示例 LKM 模块：

- **hello**：能加载模块、能 init/exit、能解析符号。
- **kebpf**：能调用内核 API、能创建 fd 对象。通过 `starry_kernel::ebpf::transform`、`starry_kernel::file::add_file_like` 等公开接口与内核 eBPF 子系统交互。

此 PR 还将 `kernel/src/lib.rs` 中的 `ebpf | file | mm | perf` 从 `mod` 升为 `pub mod`，使 out-of-tree 模块可以访问这些子系统。

### 5.4 用户态 eBPF 程序 [#886](https://github.com/rcore-os/tgoskits/pull/886)

将 7 个 aya eBPF 三件套（用户态 loader + 共享类型 + eBPF 字节码）从源仓迁移至 `os/StarryOS/user/ebpf/`：

| 程序 | 测试目标 | 内核侧依赖 |
|------|----------|-----------|
| `kret` | kretprobe（`sys_getpid` 返回值） | `perf/kprobe.rs` |
| `rawtp` | raw tracepoint（`sys_clone`） | `perf/raw_tracepoint.rs` |
| `mytrace` | tracepoint（`syscalls:sys_enter_openat`） | `perf/tracepoint.rs` |
| `syscall_ebpf` | syscall 计数（kprobe + HashMap） | `perf/kprobe.rs` + `ebpf/map.rs` |
| `upb` / `upb2` | uprobe（用户函数 / musl libc） | `perf/uprobe.rs`（暂 `Unsupported`） |
| `async_test` | tokio + `core::arch::breakpoint` smoke | 仅内核 break 处理 |

构建入口为新增的 `cargo xtask starry user-ebpf build` 子命令。

### 5.5 Stage 2: 功能拓展

**Tracepoint 拓展**：新增 `sched: sched_switch`、`sched: sched_process_fork`、`sched: sched_process_exit` 三个 tracepoint。

| 问题 | 关联情境 | 修复点 |
|------|----------|--------|
| `sched:sched_switch` 未定义 | 调度追踪 | `run_queue.rs` 中的 `switch_to` 路径 |
| `sched:sched_process_fork` / `sched_process_exit` 未定义 | 调度追踪 | `clone.rs` 中的 exit/clone 路径 |

**mmap(perf_fd) → ringbuf**：`PerfEvent` 没有覆写 `device_map`，走默认实现返回 `Err`，用户态 `mmap` 失败，`write_event` 会静默丢失（检测到 `phys_addr` 为 `None` 直接丢弃）。旧有代码的空 `Drop` 实现还会导致内存泄漏。

### 5.6 Bug 修复

**VM 持有 prog 指令的生命周期**：
- 问题：`unsafe` 扩展 slice 为 `'static`，绕过编译器。
- 解决：绑定 prog 和 vm（`Arc<BpfProg>`），确保指令的内存不会被提前回收。

**`&self` → `&mut self` 的 UB**：
- 问题：强行把 `&self` 转为 `&mut self` 获取可变引用。编译器优化时仍认为是 `&self`，可能读了寄存器里的脏数据或做了预期外的指令重排。
- 解决：将数组元素改为 `UnsafeCell<T>`，内容声明为可变。

### 5.7 Demo 实现

**syscall_count**：在 syscall 入口注 kprobe；每次触发把 syscall 号当 key，计数 +1；用户态每 N 秒迭代 map 输出。

**sched_trace**：在 `switch_to` 里注 `sched:sched_switch` 的 tracepoint；把信息（prev_tid, next_tid, prev_state, ts_ns）写入 ringbuf；用户态 perf buffer reader 实时打印。

**profile_kprobe**：在调度入口注 kprobe；获取 PC；`map[caller_pc]++`；用户态打印 top-k caller。

### 5.8 Linux vs. StarryOS eBPF 对比

| 维度 | StarryOS ebpf-kmod | tgoskits PR #850 | Linux eBPF |
|------|-------------------|-----------------|------------|
| syscall 分发 | kebpf 模块注册 handler | 内核直接实现 | 内核直接实现 |
| 程序加载 | 直接读取创建 | 直接读取创建 | Verifier 校验后创建 |
| map 管理 | BpfMap + fd | BpfMap + fd | 完整 map fd 生命周期 |
| attach | perf/kprobe/tracepoint/rawtp | perf/kprobe/tracepoint/rawtp | 大量类型 |
| 执行 | rbpf 解释执行 | rbpf 解释执行 | JIT 或解释执行 |
| 输出 | perf event output | perf event output + ringbuf mmap | perf buffer / ringbuf / map 等 |

---

## 6. BigLabA: 实验基础

### 6.1 Task1: 5 个基础实验

完成五个基础内核实验，涵盖内核开发的各个方面。

### 6.2 Task2: 个性化实验教程设计

设计了两个个性化实验教程：

1. **调度算法实验**：理解和实现不同的 CPU 调度策略
2. **同步互斥机制的可观测系统**：观察和理解内核中的同步原语

### 6.3 Task3: 扩展实验实践

完成三个扩展实验：

1. **七巧板**：图形化应用在 StarryOS 上的适配
2. **双人羽毛球**：实时交互应用
3. **Doom 游戏**：复杂图形应用的内核支持

---

## 7. BigLabB: tg-arceos-tutorial

设计了 5 个层次递进的练习，覆盖从应用到系统调用的完整栈：

| 练习 | 层次 | 做了什么 | 核心训练点 |
|------|------|----------|-----------|
| `exercise-printcolor` | 应用/输出层 | 在 ArceOS 的串口输出里打印 ANSI 彩色字符串 | `no_std` app、`axstd::println!`、ANSI escape |
| `exercise-hashmap` | 标准库适配层 | 让 `axstd::collections::HashMap` 可用 | `no_std + alloc` 下怎么补 HashMap |
| `exercise-altalloc` | 内核内存管理层 | 实现一个 bump 风格的内核全局分配器 | `GlobalAlloc` 背后的 byte/page allocator |
| `exercise-ramfs-rename` | 文件系统层 | 在 ramfs 根文件系统中支持 `fs::rename` | VFS 路径分发、ramfs 目录项重命名 |
| `exercise-sysmap` | 用户态/系统调用层 | 加载用户程序，并实现 `mmap` 支持文件映射 | ELF loader、用户地址空间、syscall emulation |

---

## 8. PR 汇总

### 已合入 PR

| PR # | 日期 | 标题 | 类型 |
|------|------|------|------|
| [#273](https://github.com/rcore-os/tgoskits/pull/273) | 05-20 | feat(starry): support multi-threaded execve | Feature |
| [#472](https://github.com/rcore-os/tgoskits/pull/472) | 05-12 | feat(starry): implement advisory file locks (fcntl POSIX/OFD, flock) | Feature |
| [#517](https://github.com/rcore-os/tgoskits/pull/517) | 05-24 | fix(starry): retry non-ELF via /bin/sh in execve | Fix |
| [#722](https://github.com/rcore-os/tgoskits/pull/722) | 05-19 | test(busybox): cover busybox_acpid via usage banner | Test |
| [#741](https://github.com/rcore-os/tgoskits/pull/741) | 05-24 | test(busybox): cover busybox_crond daemon round-trip | Test |
| [#750](https://github.com/rcore-os/tgoskits/pull/750) | 05-24 | test(starryos): add busybox crontab regression | Test |
| [#751](https://github.com/rcore-os/tgoskits/pull/751) | 05-20 | test(busybox): exercise busybox add-shell real /etc/shells rewrite path | Test |
| [#850](https://github.com/rcore-os/tgoskits/pull/850) | 05-31 | feat(starry-kernel): port eBPF runtime (ebpf/, perf/, kprobe wiring) | Feature |
| [#993](https://github.com/rcore-os/tgoskits/pull/993) | 05-28 | test(busybox): add safe-failure coverage for 7 high-side-effect applets | Test |

### 在途 PR

| PR # | 标题 | 类型 | 状态 |
|------|------|------|------|
| [#851](https://github.com/rcore-os/tgoskits/pull/851) | feat(starry-kernel): port LKM loader + cargo xtask starry kmod build | Feature | Open |
| [#880](https://github.com/rcore-os/tgoskits/pull/880) | feat(starry-modules): port hello + kebpf loadable kernel modules | Feature | Open |
| [#886](https://github.com/rcore-os/tgoskits/pull/886) | feat(starry-user): port aya eBPF userspace programs + cargo xtask user-ebpf | Feature | Open |

---

## 9. 感悟与建议

### 9.1 感悟

**1. 成熟内核上的工作与重新设计截然不同。** 在一个已经成熟的内核上工作，需要考虑已有的架构和 API，需要对齐标准预期、核对边界条件，并确保改动的 scope 不会太大、易于 review 和验证。这种"在约束中演进"的开发模式对工程能力的要求不亚于从零构建。

**2. AI 工程是一个复杂的问题。** 同样的任务，一个裸的 SOTA 闭源模型写出的代码全是 bug，需要返工十余次；但引入总结经验、借鉴先进实践的 harness 后，弱很多的开源模型就能做得很好。关键不在于模型本身的能力，而在于围绕模型构建的工程护栏。

### 9.2 建议

**1. 考虑引入 Linux 近些年的新子系统/特性或复现论文。** 从设计、权衡到开发、验证，避免只是跑通应用、支持新硬件的纯工程实践。这样可以让同学们有更多思考、学到更多东西，而不是纯粹指挥 AI。

**2. 学习软工课的模式。** 每个助教带 1-3 个 3-5 人小组做 biglab，更多的人可以做一个更大的项目/问题，既能提高深度，也能锻炼团队协作。

---

## 10. 最终产出

```
BigLabA (实验基础):
  ├── 5 个基础实验
  ├── 个性化实验教程 (调度算法 / 同步互斥可观测系统)
  └── 3 个扩展实验 (七巧板 / 双人羽毛球 / Doom)

BigLabB (实验框架):
  ├── tg-arceos-tutorial (5 层递进练习)
  └── 从应用到系统调用的完整覆盖

EXP1 (AI 开发框架):
  ├── 四角色流水线 (Designer / Developer / Reviewer / Auditor)
  ├── 9 技能 + 3 代理 + 16 脚本
  └── 三道工程护栏 (结构化输出 / 状态记忆 / 审查者否决权)

EXP2 (内核功能支持):
  ├── 多线程 execve (#273): +1732 行, 21 文件
  └── 文件锁 (#472): +3201 行, 41 文件

EXP3 (应用支持):
  ├── ~48 个非合并提交, 跨 5 周
  ├── 6 个并行 feature branch
  ├── 内核修复: vfork/execve, 信号处理, procfs, 网络, ax-fs-ng
  ├── 9 个已合入 PR (#377, #452, #517, #665, #668, #722, #741, #750, #751, #993)
  └── 320 PASS / 0 FAIL (4 个架构)

EXP4 (eBPF/LKM):
  ├── eBPF 运行时 (#850): +1949 行
  ├── LKM 加载器 (#851): +828 行, 在途
  ├── 内核模块示例 (#880): 在途
  └── 用户态 eBPF 程序 (#886): +11609 行, 在途
```

感谢聆听，敬请指正。
