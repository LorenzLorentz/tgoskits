# OS BigLab 总结报告

王鹏杰

## 1. 概述

本报告总结在 StarryOS（基于 ArceOS 模块构建的 Linux 兼容内核）上为期八周的内核开发工作。核心产出有四条主线，贯穿其上的是一套 **AI 驱动的内核开发框架（harness）**：

```
BigLabA (实验基础) ──→ BigLabB (实验框架)
      │                      │
  5 个基础实验           tg-arceos-tutorial
  个性化实验教程         5 个层次递进练习
  3 个扩展实验

         ┌──────────────── EXP1: AI 驱动的内核开发框架 (贯穿全程) ────────────────┐
         │   Designer → Developer → Reviewer → Auditor 四角色流水线 + 三道护栏    │
         └───────────────────────────────────────────────────────────────────────┘
                │                    │                       │
   EXP2 (内核功能支持) ──→ EXP3 (应用兼容) ──────→ EXP4 (eBPF / LKM)
         │                    │                       │
   多线程 execve #273    BusyBox 兼容           eBPF 运行时 #850/#886
   文件锁 #472           320 PASS / 0 FAIL      LKM 加载器 #851 + kmod 模块
                                                 可运行 demo #1132
```

整个工作的方法论可以用一句话概括：**在一个成熟、分叉、依赖盘根错节的内核上做"约束中演进"——决定代码能否落地的不是"能不能编译"，而是有没有把基线、依赖、边界、验证标准提前固定下来。** EXP1 把这套方法固化为工程框架，EXP2/3/4 则是它在三个子系统上的实战。本报告自包含，不依赖各实验的独立报告即可阅读。

> 截至成稿，已合入主线的相关 PR 包括 #273 / #472 / #517 / #722 / #741 / #750 / #751 / #850 / #851 / #886 / #993 等；eBPF demo（#1132）与 kmod 模块仍在持续推进。

---

## 2. EXP1: AI 驱动的内核开发框架

### 2.1 动机

内核开发有三个核心痛点：**测试反馈周期长**（改完要 build + rootfs + qemu + test，单次迭代数分钟）；**Linux 对照成本高**（判断行为是否正确要在真实 Linux 上跑同样的程序、查 man page、读源码）；**重复性工作多，而且最贵的反馈来得最晚**——reviewer 指出的并发竞态、跨 syscall 状态不一致、和 Linux 语义不对齐，总是在人工 review 阶段才暴露，此时返工成本最高。框架的目标就是把这些晚到的、昂贵的反馈**前移**，让 AI 在提交给人类之前先自我证伪。

### 2.2 架构：四角色流水线

框架基于 Claude Code 的 Skills / Agents 插件机制，把内核开发拆成四个相互制衡的角色，角色之间只通过共享工作区文档（`workflow.md` / `validation.md` / `journal.md`，跨角色、跨会话的单一事实来源）通信：

```
Designer ──→ Developer ──→ Reviewer ──→ Auditor ──┐
 设计工作流    实现改动      找 bug       审计意图   │
 查上游重叠    记录证据      查对齐       PASS/FAIL  │
     ▲                                              │
     └────────────── 通过 / 重做 ────────────────────┘
```

- **Designer**：接受任务、读代码库、用 `upstream-overlap-check` 确认上游/本地是否已有重叠工作，产出 `workflow.md`。
- **Developer**：执行工作流、保持改动 scope 可控、在 `validation.md` 记录可复现证据。
- **Reviewer**：本地测试、主动找 bug、核对 Linux/POSIX/Unix 对齐——使命是**证伪而不是放行**。
- **Auditor**：独立于实现者，审计意图是否真正实现、验证是否充分、有没有"看起来过了"的伪装，给出 PASS/FAIL 裁决。

关键设计是 **Reviewer / Auditor 独立于 Developer**：找 bug 与写代码由目标相反的角色承担，把人类 reviewer 的对抗性反馈固化进流水线。

### 2.3 技能与脚本

框架共 **20 个技能 + 16 个确定性脚本**。技能按角色分工，脚本提供"无幻觉"的确定性能力（锁顺序图 `lock-order-graph.py`、危险模式扫描 `pattern-scanner.py`、Linux 对照执行 `linux-ref-test.sh`、并发压测 `stress-test.sh` 等），由技能在需要时调用。其中三个最关键的 Reviewer 技能——`concurrent-bug-checker`、`cross-syscall-bug-checker`、`misalignment-checker`——是 §2.6 对照实验的直接产物。

### 2.4 核心工作流

以 `hunt-bugs` 主循环为骨架，一次迭代分为：**发现**（模式扫描 → 分类 → 优先级）→ **测试 + 对照**（写 C 测试 → Linux 基线必须先过 → 跑 StarryOS → diff 返回值/errno/行为）→ **分析修复**（根因定位 → 最小修复 → 审查管道，不过则 `REVISE` 回环）→ **记录**（更新 journal / validation）。审查这一步并行调用三个证伪技能，分别盯死并发、跨 syscall、语义对齐三类最难靠跑测试发现的 bug。

### 2.5 三道工程护栏

1. **结构化输出强制**：所有审查/审计技能按固定 schema 把结论写进 `validation.md`（期望 vs 观察、证据、严重级别），每步输出可被下一步无歧义消费。
2. **状态记忆，单一事实来源**：`journal.md` + `validation.md` 让每个缺陷从发现到修复到回归都可追溯，防止重复修复。
3. **审查者否决权（anti-fallback）**：Reviewer/Auditor 的使命被显式定义为"找问题，不是批准"；审查失败强制重做；已知但暂不修的限制必须**显式记录**而非悄悄绕过。

### 2.6 对照实验：强模型裸跑 vs 弱模型 + 强护栏

这是 EXP1 最核心的发现，也是构建整套框架的直接动因。**同一类并发任务做了两轮：**

**第一轮——Claude Opus 裸写。** 用当时的 SOTA 闭源模型直接实现 EXP2 的多线程 execve（#273）和文件锁（#472），代码能编译、happy path 能跑，但被 reviewer（周睿老师）指出**大量**问题、反复返工十余次。归类后高度集中在三个方向：并发 bug（`try_lock`→EINTR、CLOEXEC 快照时机）、跨 syscall 交互（vfork 睡眠要能被 zap、锁随 close/exit 释放）、与 Linux/POSIX 不对齐（负 l_len、NULL argv、OFD l_pid）。**这些 bug 全部在人工 review 阶段才暴露，返工成本极高。**

**复盘——蒸馏成审查技能。** 我把这十余轮反馈系统总结，发现它们几乎可无损映射到三类检查清单，于是固化成 `concurrent-bug-checker` / `cross-syscall-bug-checker` / `misalignment-checker` 三个技能——把人类 reviewer 的对抗性直觉沉淀成可机械执行的工作流。

**第二轮——弱模型在框架下重做。** 换用能力弱很多的开源模型（**GLM-5.1 与 DeepSeek-V4**）在 harness 下重做多线程 execve（工作区 `test_harness/support_multi-threaded_execve`）。这一次 Reviewer/Auditor 在提交前就用上述技能反复证伪，从 journal 可见框架内部自己捕获并修掉了和第一轮同类的 bug：

| 内部捕获的缺陷 | 类别 |
|----------------|------|
| C3：clone 检查在 `add_thread()` 之前，新线程逃过 execve 标记（TOCTOU） | 并发 + 跨 syscall |
| BUG-MEMCORRUPT：强制移除线程的 `clear_child_tid`/`robust_list_head`/`rseq` 残留写脏新地址空间 | 跨 syscall |
| BUG-SIGSTATE：Phase 2/3 之间未检查 pending SIGKILL，违反 Linux killable 语义 | 与 Linux 不对齐 |
| U1：迟醒的强制移除线程覆盖 `tg.exit_code` | 并发 |

收敛过程可量化：**Reviewer 审查 4 轮、Auditor 审查 2 轮**，Auditor 首轮裁决 FAIL（4 个 critical）、修复后复审才 PASS，7 条关键路径逐条 source-level 追踪。

**结论：弱模型 + 强护栏 > 强模型裸跑。** 决定性差异不是模型参数量，而是有没有把过往 review 经验结构化成对抗性、可机械执行的审查护栏。这也是"AI 工程"的本质——**真正稀缺的不是更强的模型，而是把领域反馈固化成流程的能力。**

---

## 3. EXP2: 内核功能支持

补齐两个被大量真实软件依赖、却长期是"假成功"占位实现的能力。两者都不是"加个 syscall"——难点在并发正确性、跨 syscall 一致性、与 Linux/POSIX 逐位对齐。

### 3.1 多线程 execve [#273]（+1732 / -85，21 文件，05-20 合入）

**问题**：`sys_execve` 在调用进程有多个线程时直接返回 `EWOULDBLOCK`，任何多线程父进程的 execve 都失败——覆盖 `rustc`/LLVM、`cargo`、任何从线程池驱动 `std::process::Command` 的程序。

**实现：两阶段提交（point-of-no-return）**。核心难点是"杀 sibling、换映像"不可逆，故严格分成可失败阶段（路径解析 + ELF 加载，不提交，出错可干净返回）与不可逆阶段（杀 sibling → 快照 CLOEXEC → 提交新 aspace → 信号重置）。

**关键设计**：① 并发 execve 经 per-process `exec_lock` 序列化，等待是 yield-loop + `exit_request` 探测，匹配 Linux "killable but not signal-interruptible"；② CLOEXEC fd 在 sibling teardown **之后**快照，防止迟到的 `F_SETFD`/`O_CLOEXEC` 丢失；③ 非 leader execve 经 `de_thread` leader transfer，把 `tid` 重命名为 TGID、重映射 task table；④ 信号重置匹配 `flush_signal_handlers`，自定义 handler 恢复 `SIG_DFL`、显式 `SIG_IGN` 保留。

**评审迭代**：周睿老师首轮即 `CHANGES_REQUESTED`，配合 octopus-review 机器人，问题归为：scope（PR 顺带改 workspace `default-members` 影响面过大，移出）、并发（`try_lock` 返回 EINTR 而非阻塞、双重 ELF 加载、probe 未 drop）、跨 syscall / 失败路径（在可失败加载**之前**就杀 sibling、不可逆点二次加载失败应 `do_exit` 而非返回 error）、测试缺失（补 `test-mt-execve`）。对应 git 修复链：`address 4 multi-thread execve race issues` → `align robust-futex owner TID + defer CLOEXEC close` → `preserve pending signals across execve` → `accepts NULL argv/envp`；期间 5 次 rebase。

**遗留**：execve 后无 `do_thread`，不恒定满足 `gettid() == getpid()`（诚实标注）。

### 3.2 文件锁 [#472]（+3201 / -18，41 文件，05-12 合入）

**问题**：`sys_fcntl` 的所有 lock 命令（`F_SETLK`/`F_SETLKW`/`F_GETLK` 及 `F_OFD_*`）与 `sys_flock` 全部返回 `Ok(0)` 不实际加锁，依赖文件锁协调的软件（dpkg、sqlite、postfix、nginx）以为拿到独占锁、实际谁都能进。

**实现**：新增 `lock.rs`，维护两类互不影响的锁表（均以 `(device, inode)` 为 key）：`FCNTL_LOCKS`（POSIX 锁 owner=pid；OFD 锁 owner=open file description，`Arc::as_ptr` 作指纹、`Weak` 检测 close）与 `FLOCK_LOCKS`。范围 half-open `[start, end)`，`l_len==0` 表示到文件尾。

**评审迭代——最硬核的部分**：周睿老师对 Linux/POSIX 语义把关极严，连续 **6 轮 `CHANGES_REQUESTED`** 才 `APPROVED`，几乎每轮都在真实 Linux 上探测锚定预期：

| reviewer 指出的问题 | Linux/POSIX 预期 | 回归用例 |
|--------------------|-----------------|---------|
| 漏导入 `F_GETLK` 常量被当模式变量，匹配所有 fcntl 命令 | 非锁命令走原路径 | clippy 不可达消除 |
| POSIX 锁不随进程退出 / close / CLOEXEC 释放 | 退出与关闭任意指向同文件的 fd 时释放 | `bug-fcntl-posix-exit/close-release` |
| `F_SETLKW` 只返回 EAGAIN 未阻塞 | 冲突释放前阻塞等待 | `bug-fcntl-setlkw-blocks` |
| 部分解锁不唤醒（`len != before` 漏判） | 冲突范围释放后重新检查、唤醒 | `bug-fcntl-partial-wake` |
| flock 无 LOCK_NB 也当非阻塞 | 无 LOCK_NB 阻塞、仅 LOCK_NB 返回 EWOULDBLOCK | `bug-flock-blocks` |
| flock 升级失败丢原锁 | 转换失败语义（含 reviewer 指出测试写反） | `bug-flock-failed-upgrade` |
| 负 l_len 被拒 | 支持反向区间 | `bug-fcntl-len-negative` |
| whence 只认 SEEK_SET | 支持 SEEK_CUR/SEEK_END | `bug-fcntl-whence` |
| OFD 未校验 l_pid | 必须为 0 否则 EINVAL | `bug-fcntl-ofd-pid-einval` |
| 不校验 fd 打开模式 | F_RDLCK 需可读、F_WRLCK 需可写否则 EBADF | `bug-fcntl-fd-mode-ebadf` |
| 目录 fd 不能加锁 | Linux 允许目录 advisory lock | `bug-advisory-lock-dir` |
| O_PATH fd 被当普通 fd | 应返回 EBADF | — |

最终落地 **14 个 C 回归用例 × 4 架构**，批准时确认锁顺序一致（`WaitQueue → FCNTL/FLOCK_LOCKS`，唤醒均在锁外）。

### 3.3 共性

两个 PR 的反馈收敛到四类缺陷——**并发/阻塞唤醒、跨 syscall 状态一致性、与 Linux/POSIX 不对齐、失败路径回滚**，正是 §2.6 三个 bug-checker 的经验来源（详见 §9 四类典型缺陷）。

---

## 4. EXP3: BusyBox 应用兼容性支持

以 BusyBox 作为 Linux 应用兼容性探针，最终使测试套件达到 **320 PASS / 0 FAIL（riscv64 / aarch64 / x86_64 / loongarch64 四架构）**。但真正的价值不在数字，而在**测试质量**——不靠造假让测试变绿。

### 4.1 弱测试 vs 真测试

BusyBox 的 multi-call 二进制塞了大量 daemon、文件 round-trip、ioctl 等"硬路径"，最省事的过法是用 `-h` 触发 usage banner、用 `[ -n "$_t" ]` 弱断言糊弄——但那等于**主动把内核 bug 藏起来**（内核坏了测试也不会挂）。issue #13 里 `acpid`、`add_shell`、`crond` 早期都走了这条捷径。

### 4.2 为这个任务设计的工作流

一句话纲领：**"不允许先把测试改通过、再回头编故事。"** 必须先复现 issue 原始命令、看 StarryOS 真实行为、写**可证伪**的失败原因 claim，再据此决定改内核还是改测试：

```
复现原始失败 ──┬─ A: panic/oops ───→ 必须改内核
              ├─ B: rc≠0 + stderr ─→ 补 syscall / procfs
              ├─ C: 卡死/静默成功 ──→ strace 证明 daemon 可用 → 改测试
              └─ D: 实际跑得通 ────→ 直接加进脚本
                       │
                  可证伪 claim ──→ 改内核 / 改测试 ──→ 全量回归 ──→ 反向自检四问 ──→ 提 PR
```

**提 PR 前的反向自检四问**（结果原样进 PR 正文）：① 这次"通过"是不是绕开了真正的代码路径？② BusyBox 是否偷偷 fallback 到无害分支？③ 行为是否与 Linux/POSIX 对齐（同一段 stdout、同一个 rc）？④ 新增内核代码是否有伪装 stub / 未测旁路？硬约束：**改测试只许加强、不许削弱**（不许把 daemonize/rename/扫描循环换成 `-h`，不许把断言改成永远会过的弱条件）；test-only PR 必须登记未覆盖的欠账。

### 4.3 重点 PR（四个，恰好覆盖工作流三种结果）

| PR | applet | 路径 | 说明 |
|----|--------|------|------|
| [#722](https://github.com/rcore-os/tgoskits/pull/722) | acpid | **诚实记限制** | StarryOS 不暴露 `/proc/acpi/event`，acpid 无法进真实事件循环；用 usage-banner 验证 applet 内置，**明示限制 + 登记欠账** |
| [#751](https://github.com/rcore-os/tgoskits/pull/751) | add-shell | **加强测试** | 弱 `--help` banner（旧 #723）升级为真实 `/etc/shells` round-trip：rc=0 + `grep -qxF` 命中新行 + `.tmp` 不残留，能反向证伪 axfs-ng `O_TRUNC`/rename 回归 |
| [#741](https://github.com/rcore-os/tgoskits/pull/741) | crond | **加强测试** | 前台规避升级为真实 daemonize 端到端：父进程 rc=0、`ps` 找到 detached daemon、SIGTERM 干净退出（vfork #377 的受益者） |
| [#750](https://github.com/rcore-os/tgoskits/pull/750) | crontab | **改内核** | 复现暴露真 bug：`ax-fs-ng` 把 `O_TRUNC\|O_APPEND` 当冲突拒绝，而 busybox crontab 正用此组合；改为纯放松、对齐 Linux/POSIX/Rust std，配严格 round-trip 回归 |

### 4.4 暴露的内核缺陷

| 缺陷 | PR | 根因 |
|------|-----|------|
| `O_TRUNC \| O_APPEND` 被拒 | #750 | `ax-fs-ng` open flags 检查过严 |
| 非 ELF 脚本无法执行 | #517 | execve 缺 `/bin/sh` fallback |
| daemonize 失败 | #377 | vfork + CLONE_VM execve 未正确实现 |
| SIGSTOP 杀死进程而非挂起 | #925 | 信号处理实现错误 |
| procfs 数据缺失 / 网络工具不可用 | #452 / #668 | 多个 `/proc` 条目、socket ioctl 缺失 |

---

## 5. EXP4: eBPF 与 LKM 内核扩展机制

把一套内核可观测性与可扩展性基础设施从独立仓 `Starry-OS/StarryOS:ebpf-kmod` 迁移并重构到 tgoskits 主线，两条线并行：**eBPF 线**让用户态程序安全挂探针采事件；**LKM/kmod 线**让 `.ko` 模块运行时加载进内核。source 与 target 分叉一年以上，故这是一次**审计驱动**的迁移而非机械 cherry-pick。

### 5.1 迁移工作流与代码审计

专门写了 `WORKFLOW_EBPF_LKM_MIGRATION.md`，按四 Phase 推进：**Phase 0 锁基线**（journal 记录 source/target/上游 SHA）→ **Phase 1 审计**（diff-audit、crate-fork-audit）→ **Phase 2 集成基座**（`feat/ebpf-integration-base` 合并上游 #673 tracepoint + #805 kallsyms/kprobe，三架构 build 通过）→ **Phase 3 分 PR-A/B/C/D 实现**。审计结论：① **已有能力不重复迁**（#244/#306/#446 已并入 dev，#673/#805 已覆盖 tracepoint/kallsyms/kprobe，故只补 perf + `kbpf-basic` 真实现 + LKM + 示例 + 用户程序）；② **禁止个人 fork**（crate-fork-audit 逐条核对 source 的 5 条 `[patch.crates-io]` Godones fork，结论全不需要，tgoskits 已 vendor 并重命名为 `ax-*`，任何迁移 PR 出现 `Godones/*` patch 直接驳回）；③ **不复制旧 Makefile/`.ld`**，统一 `cargo xtask`；④ **锁 SHA，不无声跟随 force-push**。

### 5.2 eBPF 运行时

数据流：`bpf(2)` → `BpfMap`/`BpfProg`（FileLike + fd）→ `perf_event_open`（`PerfEvent` 按 `PerfTypeId` 分派 kprobe/tracepoint/uprobe）→ `OwnedEbpfVm`（rbpf 解释执行）→ ringbuf / perf output（mmap）。

| PR | 内容 | 规模 |
|----|------|------|
| [#850](https://github.com/rcore-os/tgoskits/pull/850) | 运行时迁移：`ebpf/` 子模块（`sys_bpf` 真分派、map/prog FileLike、`KernelAuxiliaryOps`）+ 全新 `perf/` 子模块（PerfEvent、ringbuf、kprobe/tracepoint 接线） | +1949 / -2054 |
| [#886](https://github.com/rcore-os/tgoskits/pull/886) | 内核侧运行时收敛：tracepoint/kprobe/perf 接线、uprobe 端到端、perf ringbuf mmap 副作用治理 | +792 / -97 |
| [#1132](https://github.com/rcore-os/tgoskits/pull/1132) | 可运行 demo：`apps/starry/ebpf/` 下 uprobe/kprobe/kretprobe/tracepoint 用户态程序 + 构建链 | +6696 |

**关键洞察**：① 跨 crate errno 边界——`kbpf-basic` 的 `axerrno` 与 tgoskits 的 `ax-errno` 是不同 crate，必须显式 `BpfError ↔ AxError` 转换；② 两处生命周期/UB 修复——VM 持有 prog 指令原本 `unsafe` 扩 slice 为 `'static`，改为 `Arc<BpfProg>` 绑定；一处 `&self` 强转 `&mut self` 会让编译器读脏寄存器，改为 `UnsafeCell<T>`；③ `mmap(perf_fd)→ringbuf` 原本因 `PerfEvent` 未覆写 `device_map` 静默丢事件 + 空 Drop 泄漏，已补齐；④ 新增 `sched:sched_switch`/`process_fork`/`process_exit` 三个 tracepoint；⑤ 三个 demo（`syscall_count` kprobe 计数、`sched_trace` tracepoint 写 ringbuf、`profile_kprobe` 按 caller PC 计数）验证能力打通。

### 5.3 LKM / kmod 加载机制

目标：让用户态把 Rust 编译的 `.ko` 在运行时加载进内核、解析符号、调用内核 API，并提供与工具链一致的构建链。工程量集中在内核加载器与模块+构建链两块。

**内核加载器（[#851](https://github.com/rcore-os/tgoskits/pull/851)，+828 / -241，已合入）**：三个 syscall `init_module`/`finit_module`/`delete_module`，每步都做成真实现——`resolve_symbol` 走 `kallsyms` 真实解析、`finit/delete` 真实现、用户态内存经 `VmBytes`/`vm_load_string` 正规拷贝、`printk` 等 C-ABI shim 经 `lwprintf-rs` 实现并正确转发 varargs、模块由 `MODULES` 注册表持有可卸载。真正吃功夫的是一连串底层链接与内存语义的打磨：驱动 `rust-lld` 以 GNU ELF driver 模式做 partial link 产出可加载重定位；按 ELF section 权限安置页、**释放 section 页前先恢复 RW 内核映射**；vmalloc 区按页对齐校验、加载前拒绝重名模块并 flush icache；适配 errno 边界与传播、保持 `ax-errno` fork 领先以修构建、适配 HAL imports 到 dev 的 `ax_runtime::hal` 布局。

**示例模块 + 构建链（`hello` / `kebpf`）**：`hello` 验证模块能加载、init/exit、解析符号；`kebpf` 通过 `starry_kernel::ebpf::transform`、`file::add_file_like` 等公开接口调用内核 API、创建 fd 对象，并作为 **`bpf(2)` 的 provider**（为此把内核 `ebpf|file|mm|perf` 从 `mod` 升为 `pub mod` 开放给 out-of-tree 模块）。配套实现了 runtime `bpf(2)` 注册（让 `kebpf.ko` 既能内置也能可加载提供 `bpf(2)`）、把 `unwrap/expect` 换成规范错误传播 + 有界 `bpf_attr` 读取；并引入 **`STARRY_KMOD` 内核构建模式**（build-std parity + loadable relocations + 传入 platform features），让模块与内核在符号解析的 hash parity 上一致；新增 `kmod-modules` 可加载模块 **QEMU smoke 测试**，并写了 `docs/kmod.md` 构建/使用指南。

这条线从"三个 syscall"出发，真正落地打通了 **ELF partial link、符号 hash parity、section 权限与缓存一致性、C-ABI varargs、build-std 构建对齐、运行时 provider 注册** 一整条贯穿内核、链接器、构建系统三层的链路；目前 loader 已合入主线，模块与端到端加载持续推进。

### 5.4 两条线对比

| 维度 | eBPF 线 | LKM / kmod 线 |
|------|---------|---------------|
| 入口 | `bpf(2)` / `perf_event_open` | `init/finit/delete_module` |
| 扩展方式 | 受限字节码 + rbpf 解释执行（沙箱） | 原生 `.ko`、符号重定位后直接执行 |
| 核心难点 | 跨 crate errno、VM 生命周期、ringbuf mmap | partial link、符号 hash parity、section 权限、build-std 对齐 |
| 安全模型 | 沙箱 | 完全信任 |

两条线本质是"在内核里安全跑外部逻辑"的两种范式：eBPF 用沙箱换安全，kmod 用符号链接换能力。与 Linux eBPF 相比（StarryOS 走解释执行、attach 类型有限，但核心数据流已打通）：

| 维度 | tgoskits | Linux eBPF |
|------|----------|-----------|
| 程序加载 | 直接读取创建 | Verifier 校验后创建 |
| attach | perf/kprobe/kretprobe/tracepoint/raw_tp/uprobe | 大量类型 |
| 执行 | rbpf 解释执行 | JIT 或解释执行 |
| 输出 | perf output + ringbuf mmap | perf buffer / ringbuf / map |

---

## 6. BigLabA: 实验基础

- **Task1**：完成 5 个基础内核实验，涵盖内核开发各方面。
- **Task2 个性化实验教程**：设计两个教程——「调度算法实验」（理解与实现不同 CPU 调度策略）与「同步互斥机制的可观测系统」（观察内核同步原语）。
- **Task3 扩展实验**：完成三个——七巧板（图形应用适配）、双人羽毛球（实时交互）、Doom（复杂图形应用的内核支持）。

---

## 7. BigLabB: tg-arceos-tutorial

设计了 5 个层次递进的练习，覆盖从应用到系统调用的完整栈：

| 练习 | 层次 | 做了什么 | 核心训练点 |
|------|------|----------|-----------|
| `exercise-printcolor` | 应用/输出层 | 串口输出打印 ANSI 彩色字符串 | `no_std` app、`axstd::println!`、ANSI escape |
| `exercise-hashmap` | 标准库适配层 | 让 `axstd::collections::HashMap` 可用 | `no_std + alloc` 下补 HashMap |
| `exercise-altalloc` | 内核内存管理层 | 实现 bump 风格内核全局分配器 | `GlobalAlloc` 背后的 byte/page allocator |
| `exercise-ramfs-rename` | 文件系统层 | ramfs 支持 `fs::rename` | VFS 路径分发、目录项重命名 |
| `exercise-sysmap` | 用户态/系统调用层 | 加载用户程序并实现文件映射 `mmap` | ELF loader、用户地址空间、syscall emulation |

---

## 8. StarryOS 架构分析

八周横跨进程、文件锁、文件系统、eBPF、LKM 多个子系统，对 StarryOS 的架构在"AI 辅助开发"视角下有一些观察。

### 8.1 架构概览

StarryOS 把 ArceOS 的模块（HAL、调度、内存、网络、文件系统）作为底座，在其上叠 Linux 兼容层（syscall 分发、进程/信号、FileLike fd 抽象）。所有跨 crate 依赖经 tgoskits vendor 并重命名为 `ax-*`，构建/测试统一走 `cargo xtask`。

### 8.2 有利于 AI 开发的架构特征

- **模块化分层**：改一个子系统不易波及全局，scope 可控，天然适合 AI 做局部、可 review 的小改动。
- **Rust 编译器护栏**：所有权/借用/类型在编译期挡住一大类内存与并发错误——EXP4 那处 `&self→&mut` 的 UB，最终正是靠把状态显式化为 `UnsafeCell<T>` 让编译器重新看见。
- **FileLike trait 抽象**：`BpfMap`/`BpfProg`/`PerfEvent` 都实现统一 fd 接口，AI 加新 fd 类型有清晰模板。
- **`cargo xtask` 确定性构建/测试**：一条命令复现 build/rootfs/qemu/test，是脚本层"无幻觉"能力的基础。

### 8.3 产生系统性摩擦的架构特征

- **单体 syscall 分发**：跨 syscall 的共享状态（如文件锁随 close/exit/exec 释放）需要在多处手动串联，AI（甚至人）极易漏——EXP2 文件锁 6 轮 review 一大半卡在这。
- **隐式跨层副作用**：如 EXP4 perf `mmap` 的 `device_map` 默认返回 `Err` 导致静默丢事件、execve 与 fd-table 锁/robust-futex 的交错——副作用不在签名里，难以静态发现。
- **"假成功" stub 策略**：历史上大量 syscall/ioctl 返回 `Ok(0)` 占位（fcntl lock、flock、bpf、kmod `resolve_symbol`），让上层看起来成功却没有真实语义——这是 EXP2/3/4 反复踩的同一个坑，也是 `misalignment-checker` 与 anti-fallback 护栏的根源。
- **crate 重复 / errno 边界**：`kbpf-basic` 的 `axerrno` 与 tgoskits 的 `ax-errno` 是不同 crate，跨界要手动转换。

### 8.4 改进方向

1. 把单体 syscall dispatch 拆成按子系统注册的表，降低跨 syscall 状态串联的遗漏面。
2. 为 fd 生命周期事件（close/exit/exec）提供统一 hook，让"资源随关闭释放"成为框架保证而非每个子系统手写。
3. 主线明确**禁止"假成功 stub"**：未实现就显式 `ENOSYS`，配合 `misalignment-checker` 在 CI 拦截。
4. 统一 errno crate，消除跨 crate 边界转换。

---

## 9. 四类典型缺陷

横向看 EXP2/3/4 的全部 review 反馈与暴露的内核 bug，可归纳为四类反复出现的模式——它们正是 EXP1 三个 bug-checker 的设计依据：

1. **边界条件导致的"假成功" / 静默失效。** 操作看起来成功（rc=0）但内核状态没变：fcntl lock / flock 返回 `Ok(0)`、bpf / kmod `resolve_symbol` 桩、perf `mmap` 失败后静默丢事件。**最危险的一类**——它能在内核完全坏掉时依然让测试变绿。
2. **新旧实现混淆。** 同一能力存在桩与真实现两套：#851 取代 #849 的 `resolve_symbol`/`finit` 桩；#850 替换 #805 的单文件 bpf stub。迁移时若不审计清楚，容易把桩当真。
3. **直觉式错误的边界处理。** "想当然"的边界判断与 Linux 不符：负 `l_len` 直接拒、`O_TRUNC|O_APPEND` 当冲突、whence 只认 `SEEK_SET`、`O_PATH` 当普通 fd、`F_OFD_*` 不校验 `l_pid`。
4. **错误路径副作用。** 失败路径没有和成功路径用同一套清理纪律：execve 在可失败阶段前就杀 sibling、flock 升级失败丢原锁、二次加载失败返回 error 却留下不一致状态。

四类缺陷对应 `concurrent-bug-checker`（1、4 的并发面）、`cross-syscall-bug-checker`（1、4 的跨 syscall 面）、`misalignment-checker`（2、3）。

---

## 10. PR 汇总

| PR # | 标题 | 类型 | 状态 |
|------|------|------|------|
| [#273](https://github.com/rcore-os/tgoskits/pull/273) | support multi-threaded execve | Feature | 已合入 |
| [#472](https://github.com/rcore-os/tgoskits/pull/472) | advisory file locks (fcntl POSIX/OFD, flock) | Feature | 已合入 |
| [#517](https://github.com/rcore-os/tgoskits/pull/517) | retry non-ELF via /bin/sh in execve | Fix | 已合入 |
| [#722](https://github.com/rcore-os/tgoskits/pull/722) | busybox_acpid via usage banner | Test | 已合入 |
| [#741](https://github.com/rcore-os/tgoskits/pull/741) | busybox_crond daemon round-trip | Test | 已合入 |
| [#750](https://github.com/rcore-os/tgoskits/pull/750) | ax-fs-ng O_TRUNC\|O_APPEND + crontab regression | Fix | 已合入 |
| [#751](https://github.com/rcore-os/tgoskits/pull/751) | busybox add-shell real /etc/shells rewrite | Test | 已合入 |
| [#993](https://github.com/rcore-os/tgoskits/pull/993) | safe-failure coverage for 7 applets | Test | 已合入 |
| [#850](https://github.com/rcore-os/tgoskits/pull/850) | port eBPF runtime (ebpf/, perf/, kprobe) | Feature | 已合入 |
| [#851](https://github.com/rcore-os/tgoskits/pull/851) | LKM loader + cargo xtask starry kmod build | Feature | 已合入 |
| [#886](https://github.com/rcore-os/tgoskits/pull/886) | eBPF kernel runtime (tracepoint / kprobe / perf) | Feature | 已合入 |
| [#1132](https://github.com/rcore-os/tgoskits/pull/1132) | runnable eBPF demos under apps/starry/ebpf | Feature | 已合入 |

（EXP3 另有 #377 / #452 / #665 / #668 等多个已合入的 BusyBox 相关 PR。）

---

## 11. 最终产出

```
BigLabA: 5 基础实验 + 2 个性化教程(调度/同步可观测) + 3 扩展实验(七巧板/羽毛球/Doom)
BigLabB: tg-arceos-tutorial 5 层递进练习 (应用→系统调用全覆盖)

EXP1 (AI 开发框架):
  ├── 四角色流水线 (Designer / Developer / Reviewer / Auditor)
  ├── 20 技能 + 16 脚本 + 3 工作区文档
  ├── 三道工程护栏 (结构化输出 / 状态记忆 / 审查者否决权)
  └── 对照实验: 弱模型(DeepSeek)+强护栏 收敛了强模型(Opus)裸跑的同类 bug

EXP2 (内核功能支持):
  ├── 多线程 execve (#273): +1732 / 21 文件, 两阶段提交
  └── 文件锁 (#472): +3201 / 41 文件, 6 轮 review, 14 用例 × 4 架构

EXP3 (应用兼容):
  ├── 320 PASS / 0 FAIL (4 架构), 反向自检工作流
  └── 暴露并修复多个内核缺陷 (ax-fs-ng / vfork / 信号 / procfs / 网络)

EXP4 (eBPF/LKM):
  ├── eBPF 运行时 (#850 +1949 / #886 +792) + 可运行 demo (#1132 +6696)
  └── LKM 加载器 (#851 +828) + hello/kebpf 模块 (partial link / 符号 parity / kmod 构建链)
```

---

## 12. 感悟与建议

### 12.1 感悟

**1. 成熟内核上的工作与重新设计截然不同。** 在一个已经成熟的内核上工作，需要考虑已有的架构和 API，对齐标准预期、核对边界条件，并确保改动 scope 不会太大、易于 review 和验证。这种"在约束中演进"的开发模式，对工程能力的要求不亚于从零构建——本报告里反复出现的并发竞态、跨 syscall 状态、逐位语义对齐，都是这种约束的体现。

**2. AI 工程是一个复杂的问题。** 同样的任务，一个裸的 SOTA 闭源模型写出的代码全是 bug、需要返工十余次；但引入总结经验、借鉴先进实践的 harness 后，弱很多的开源模型就能做得很好（EXP1 §2.6 的对照实验）。**关键不在于模型本身的能力，而在于围绕模型构建的工程护栏**——把领域反馈固化成可机械执行的对抗性流程，比换一个更大的模型更有价值。

### 12.2 建议

**1. 考虑引入 Linux 近些年的新子系统 / 特性，或复现论文。** 从设计、权衡到开发、验证，避免只是"跑通应用、支持新硬件"的纯工程实践。这样能让同学们有更多思考、学到更多东西，而不是纯粹指挥 AI。

**2. 学习软工课的模式。** 每个助教带 1-3 个 3-5 人小组做 biglab，更多的人合作做一个更大的项目/问题，既能提高深度，也能锻炼团队协作。

---

*感谢聆听，敬请指正。各实验的完整细节与 review 线索另见 `report/exp1~4/report.md` 及对应 GitHub PR。*
