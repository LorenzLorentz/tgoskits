# Workflow: 旧 StarryOS eBPF / LKM 基础设施向 tgoskits 迁移

适用范围: 把 `Starry-OS/StarryOS:ebpf-kmod` 分支上的 eBPF / kprobe / tracepoint /
kallsyms / LKM 等可观测性与可加载模块基础设施移植到 `rcore-os/tgoskits:dev`
下 `os/StarryOS/` 子目录的所有工作。

本流程在 `AGENTS.md` 与 `CLAUDE.md` 之上追加要求, 与
`WORKFLOW_LINUX_SEMANTICS.md` 并列, 不替代。同时参考
[JosephJoshua/starry-harness](https://github.com/JosephJoshua/starry-harness)
的 "deterministic tools + structured artifacts + Linux 对照" 方法论 (尤其
`start-submission` / `check-upstream` / `review-quality` 的思路)。

> 写这份文档的动因: 本次迁移的 source (`ebpf-kmod`) 与 target (`tgoskits dev`)
> 的基线已经分叉一年以上, 上游同时有两个开放 PR (#673、#805) 正在做同一件事,
> 并且涉及 6+ 个 crate (`ksym`、`kprobe`、`ktracepoint`、`kbpf-basic`、
> `kmod-loader`、`rbpf`) 与多个跨架构 (`x86_64` / `riscv64` / `aarch64` /
> `loongarch64`) 的内核构建流程改造。任意一人独立推进都不可行, 必须
> **先写清协作 contract, 再分头施工**。

---

## 0. 触发判定

如果你的 PR 满足以下任一条, 按本文档走:

- 修改 `os/StarryOS/kernel/src/{ebpf,kprobe,tracepoint,kallsyms,perf,kmod}*`
- 修改 `os/StarryOS/starryos/build.rs` 中与 kallsyms / 链接脚本相关的部分
- 在 `os/StarryOS/Cargo.toml` 中新增 `ksym` / `kprobe` / `ktracepoint` /
  `kbpf-basic` / `kmod-loader` / `rbpf` 依赖
- 在 `tgoskits` 中新增 LKM (`.ko`) 构建产物或链接脚本
- 移植任何 `Starry-OS/StarryOS:ebpf-kmod` 上的 `kernel/`、`modules/`、
  `user/musl/` 的内容
- 接入 `Godones/{axcpu,arceos,page_table_multiarch}` 任意 fork 分支

否则按常规 `CONTRIBUTING.md` / `WORKFLOW_LINUX_SEMANTICS.md` 走即可。

---

## 1. 上下文与已知事实

### 1.1 上游迁移计划 (固定清单)

| # | 项目 | 状态 | 关联 PR |
|---|---|---|---|
| 1 | 扩展 axhal 对 break/debug 异常的处理 | ✅ 已并入 dev | rcore-os/tgoskits#244 |
| 2 | `proc/pid/maps` 支持 | ✅ 已并入 dev | rcore-os/tgoskits#306 |
| 3 | dynamic debug 支持 | ✅ 已并入 dev | rcore-os/tgoskits#446 |
| 4 | tracepoint 支持 | 🔄 开放 PR | rcore-os/tgoskits#673 (`feat/tp`, Godones) |
| 5 | 内核符号表 (`ksym` / `kallsyms`) | 🔄 开放 PR (覆盖) | rcore-os/tgoskits#805 (`feat/ebpf-observability`, CN-TangLin) |
| 6 | kprobe 支持 | 🔄 开放 PR (覆盖) | rcore-os/tgoskits#805 (同上) |
| 7 | eBPF 内核侧 (`kbpf-basic` + perf 文件) | ❌ stub 在 #805 | — (本流程 PR-A) |
| 8 | eBPF 用户程序 (`kret` / `mytrace` / `rawtp` / `upb` / `syscall_ebpf` …) | ❌ | — (本流程 PR-D) |
| 9 | LKM 支持 (`kmod-loader` + 内核构建流程) | ❌ | — (本流程 PR-B) |
| 10 | kmod 示例 (`hello` / `kebpf`) | ❌ | — (本流程 PR-C) |

### 1.2 Source / Target 基线

| | 路径 | 当前 HEAD | 备注 |
|---|---|---|---|
| Source | `~/StarryOS` (branch `ebpf-kmod`) | `3fed46e` | 基于旧 StarryOS (PR #78 时代), 含 3 个迁移 commit (`6278553` user musl, `1488cf3` 内核 + LKM, `3fed46e` deps 微调) |
| Target | `~/tgoskits` (`rcore/dev`) | `6ff5a24bf` | 当前主仓 dev, 已包含 #244 / #306 / #446 |
| Stack-on | `rcore/feat/tp` (#673) | TBD | 必须 fetch 后锁 SHA |
| Stack-on | `rcore/feat/ebpf-observability` (#805) | TBD | 必须 fetch 后锁 SHA |

锁 SHA 的方式: 每个 PR 开工前在 PR body 第一行写明
`Stacked on rcore-os/tgoskits@<dev-sha>, feat/tp@<sha>, feat/ebpf-observability@<sha>`,
review 期间 rebase 也按记录的 SHA 推进, 不无声跟随上游 force-push。

### 1.3 命名与目录映射

| `Starry-OS/StarryOS:ebpf-kmod` 中的路径 | `rcore-os/tgoskits:dev` 中的目标位置 |
|---|---|
| `kernel/src/bpf/` | `os/StarryOS/kernel/src/ebpf/`  (注意是 `ebpf`, #805 已建立) |
| `kernel/src/kprobe/` | `os/StarryOS/kernel/src/kprobe.rs` 或 `kprobe/` (#805 已建立 `.rs` 形式) |
| `kernel/src/tracepoint/` | `os/StarryOS/kernel/src/tracepoint/`  (#673 已建立) |
| `kernel/src/perf/` | `os/StarryOS/kernel/src/perf/`  (本流程 PR-A 新增) |
| `kernel/src/kmod/` | `os/StarryOS/kernel/src/kmod/`  (本流程 PR-B 新增) |
| `kernel/src/syscall/perf.rs` / `kmod/mod.rs` | `os/StarryOS/kernel/src/syscall/` 对应位置, 按 dev 当前的 dispatch 拆分方式适配 |
| `starryos/kallsym.ld` | `os/StarryOS/starryos/kallsym.ld` 或合并进 `ext_linker.ld` (按 #805 与 #673 落点决定) |
| `starryos/build.rs` 中 kallsyms 生成逻辑 | `os/StarryOS/starryos/build.rs` (#805 已写第一版, 需协调) |
| `modules/hello` / `modules/kebpf` | `os/StarryOS/modules/` 或独立 workspace (PR-B 时决定, 见 §5.3) |
| `modules/kmod.mk` / `kmod-linker.ld` | 重写为 `cargo xtask` 子命令, **不引入 Makefile** (见 §5.3) |
| `user/musl/{kret,mytrace,rawtp,upb,upb2,syscall_ebpf,async_test}` | `os/StarryOS/user/` 或 `test-suit/starryos/ebpf/` (PR-D 时确定, 见 §5.4) |

---

## 2. Roadmap、Phases、Tasks

本节是协作 contract: **谁要做什么、依赖什么、产出什么**。Task ID 引用本仓
Claude Code task list (`TaskList`), 同时跨人协作时也用同样的编号在 PR / issue
里互引。

### 2.1 依赖图

```
Phase 0 ── Task #1 抓取 rcore/dev + #673 + #805 分支
                │
Phase 1 ── Task #2 差异审计 (源 vs 目标 + 在途 PR 覆盖范围)
        │  Task #3 上游 crate fork 审计 (Godones forks)
                │
Phase 2 ── Task #4 integration base (合 dev + #673 + #805, 三架构 build 过)
                │
Phase 3 ┬─ Task #5  PR-A: kbpf-basic 真正实现 + perf 文件 ─┐
        │                                                  └─→ Task #8 PR-D: 用户态 eBPF 程序
        └─ Task #6  PR-B: LKM (kmod-loader) ──────────────┐
                                                          └─→ Task #7 PR-C: kmod 示例
```

### 2.2 Task 卡片

> 每个 Task 在 `TaskList` 中均有对应条目。新人领任务的动作: (a) `TaskList`
> 找未 claim 的 task, (b) 在 PR description 写 `Task: #N`, (c) 把 task
> owner 改为自己, (d) 完成后 mark completed。

#### Task #1 — Phase 0 抓取分支 (任何人)

- 在本地 `~/tgoskits` 执行:
  ```bash
  git fetch rcore dev
  git fetch rcore pull/673/head:pr-673-tp
  git fetch rcore pull/805/head:pr-805-ebpf-observability
  ```
- 产出: 三个本地分支的 SHA, 记录到工作日志 (见 §8)。
- 完成判据: `git rev-parse rcore/dev pr-673-tp pr-805-ebpf-observability` 都成功。

#### Task #2 — Phase 1 差异审计 (1 人)

- 把 `Starry-OS/StarryOS:ebpf-kmod` 的 3 个 commit 文件清单逐行对照
  `tgoskits/os/StarryOS` 当前状态 + #673 + #805 的文件清单, 输出一份
  **剩余清单表**:

  | 源文件 | 源行数 | 目标位置 | 是否被 #673/#805 覆盖 | 剩余需移植内容 | 归属 PR |
  |---|---|---|---|---|---|

- 产出: `os/StarryOS/docs/ebpf-migration/diff-audit.md` (与本工作流文档同目录;
  仓根 `docs/` 是 docusaurus 站点根, 不放过程文档)。
- 完成判据: 表内 "剩余需移植内容" 列总和 = ebpf-kmod 全部 6400+ 行的内容
  指针 (允许 "未来再迁" 的显式延后行)。

#### Task #3 — Phase 1 上游 crate fork 审计 (1 人, 可与 #2 并行)

- 对 `Godones/axcpu` (branch `0.3.0.preview.9`), `Godones/arceos` (branch
  `dev`, 关注 `axhal`、`axalloc`), `Godones/page_table_multiarch` (branch
  `dev`) 逐一做:
  ```bash
  git diff arceos-org/<repo>/<latest>..Godones/<repo>/<branch> -- <relevant paths>
  ```
- 把每一处实质改动分类为以下之一:
  - **A. 已在 tgoskits 上游存在**: 不需要做。
  - **B. 需要 PR 到 arceos-org**: 创建子任务, 在 `Task` 列表里加新条目,
    blockBy 当前迁移 task。
  - **C. tgoskits 本地适配可吸收**: 直接在迁移 PR 内修改 tgoskits 自带的
    `components/`、`drivers/`、`platform/` 等同名/同义代码。
  - **D. 不需要 (旧实现已被替代)**: 显式标注理由。
- 产出: `os/StarryOS/docs/ebpf-migration/crate-fork-audit.md`。
- **硬约束**: 任何迁移 PR 都不得在 `os/StarryOS/Cargo.toml` 引入
  `[patch.crates-io]` 指向个人 fork。出现任何 B 类条目, 在子 PR 合入
  arceos-org 之前, 主迁移 PR 等待 (见 §5.1)。

#### Task #4 — Phase 2 构建 integration base (1 人)

- 创建本地分支 `feat/ebpf-integration-base` 自 `rcore/dev`, 依次:
  ```bash
  git merge --no-ff pr-673-tp
  git merge --no-ff pr-805-ebpf-observability
  ```
  解冲突 (#673 与 #805 都改 `kernel/src/lib.rs`、`syscall/mod.rs`,
  必有冲突, 解决方式以两侧语义都保留为准)。
- 跑 §6 的验证清单。
- 产出: 本地分支 `feat/ebpf-integration-base`, 推到 `origin` 但
  **不开 PR** (只是 stacking parent), 在 §8 工作日志记录其 SHA。
- 完成判据: 三架构 `cargo xtask starry build` 通过, clippy 通过, fmt 干净。

#### Task #5 — Phase 3 PR-A: kbpf-basic + perf 文件 (1 人)

- 从 `feat/ebpf-integration-base` 切 `feat/starry-ebpf-runtime`。
- 移植 (映射见 §1.3): `kernel/src/perf/{bpf,kprobe,tracepoint,raw_tracepoint,uprobe,mod}.rs`,
  `kernel/src/bpf/{map,prog,tansform}.rs` (注意原仓库拼写为 `tansform`,
  迁移时改成 `transform` 并在 PR body 说明)。
- 接入 `kbpf-basic` (实现 `KernelAuxiliaryOps` + `PerCpuVariantsOps`)。
- 把 #805 中 `sys_bpf` / `sys_perf_event_open` 的 stub 换成真路由。
- 与 #673 的 tracepoint 对接 `BPF_RAW_TRACEPOINT_OPEN`。
- 完成判据: 三架构 build + clippy 11 features, 至少一个用户态测试程序
  (PR-D 中的最小 case) 能成功 `bpf(BPF_PROG_LOAD)`。

#### Task #6 — Phase 3 PR-B: LKM (kmod-loader) (1 人, 与 #5 并行)

- 从 `feat/ebpf-integration-base` 切 `feat/starry-lkm`。
- 移植: `kernel/src/kmod/` (`mod.rs` + `shim/{block,kprint,mq,xarray,mod}.rs`),
  `kernel/src/syscall/kmod/`, `starryos/kallsym.ld` 中 LKM 段相关变更。
- 接入 `kmod-loader` crate。
- **构建系统**: 不引入 `make/`、`Makefile`、`modules/kmod.mk`。改成
  `cargo xtask starry kmod build --module <name>` 子命令 (在
  `scripts/axbuild/` 下加), 输出 `.ko` 到 `target/<arch>/kmod/`。
- 完成判据: 三架构 build + clippy, `init_module` / `delete_module` /
  `finit_module` syscall 在最小测试程序下可加载/卸载一个空模块。

#### Task #7 — Phase 3 PR-C: kmod 示例 (1 人, 等 #6)

- 从 `feat/ebpf-integration-base` (或 #6 已合入后的 dev) 切
  `feat/starry-kmod-examples`。
- 移植 `modules/hello` (打印型示例) 与 `modules/kebpf` (eBPF map/prog 加载示例)。
- 调整 `Cargo.toml` 让模块 crate 作为 workspace member 或独立子工作区
  (Task #6 时已定标准, 这里只跟随)。
- 完成判据: `cargo xtask starry kmod build --module hello` 产出 `.ko`,
  qemu 内 `insmod /lib/modules/hello.ko` 能在 dmesg 看到 hello。

#### Task #8 — Phase 3 PR-D: 用户态 eBPF 程序 (1 人, 等 #5)

- 从 `feat/ebpf-integration-base` (或 #5 已合入后的 dev) 切
  `feat/starry-ebpf-userspace`。
- 决定用户态测试程序的 canonical 位置 (`os/StarryOS/user/` 还是
  `test-suit/starryos/ebpf/`), 在 PR body 引用 §1.3 表更新。
- 移植 `user/musl/{kret,mytrace,rawtp,upb,upb2,syscall_ebpf,async_test}`。
- 适配 `cargo xtask` 跨编 (`riscv64`/`aarch64`/`x86_64` 三 musl 工具链)。
- 完成判据: 至少 `kret` 与 `rawtp` 两个程序在 qemu 内可跑通,
  trace_pipe 有期望输出。

### 2.3 关键非功能任务 (任何人)

- **`scripts/test/clippy_crates.csv` 更新**: 每新增 crate 必须同步加入
  (`AGENTS.md` 显式要求)。在每个迁移 PR 内顺手完成。
- **`docs/starry-reports/ebpf-migration/journal.md` 维护**: 见 §8。
- **rcore-os/{arceos,axcpu,page_table_multiarch} 子 PR** (如 Task #3 发现
  B 类): 谁审出谁负责开, 阻塞当前迁移 task, 在 `TaskList` 用
  `addBlockedBy` 显式标注。

---

## 3. Stacking 策略

### 3.1 Base 选择

- **不依赖 #673 / #805 的工作**: 直接基于 `rcore/dev`。
- **依赖 #673** (tracepoint 接口) 的工作: 基于 `pr-673-tp` 本地分支。
- **依赖 #805** (kallsyms / kprobe / bpf 框架) 的工作: 基于
  `pr-805-ebpf-observability`。
- **同时依赖两者**: 基于 `feat/ebpf-integration-base` (Task #4 产物)。

### 3.2 上游 PR 更新时的应对

- #673 / #805 force-push 后:
  1. 不立即 rebase。
  2. 在 §8 工作日志记录新 SHA。
  3. 评估改动是否影响 stacking PR 接口; 若无影响, 延后到本 PR review 间隙再 rebase。
  4. 若有破坏性 API 变更, 优先和该 PR author 沟通, 不要私下 patch 绕开。

### 3.3 #673 / #805 合入 dev 后

- 立即 rebase 本 PR 到新的 `rcore/dev`, 删除 stacking base 引用,
  更新 PR body 中 "Stacked on" 行。

### 3.4 三个 PR 的归属冲突

如果 PR-A (kbpf-basic) 需要修改 #805 已经写的 `ebpf.rs` 中的 stub:

- **不要 force-push 别人的 PR 分支**。
- 改动作为 PR-A 的一部分提交; 在 PR-A description 显式标注 "supersedes
  the stub introduced in #805 / 与 #805 协作完成"。
- 主动在 #805 评论里 @ 作者周知。

---

## 4. 上游 crate 处理规范

### 4.1 [patch.crates-io] 红线

`os/StarryOS/Cargo.toml` 与根 `Cargo.toml` 的 `[patch.crates-io]` **不得**
新增以下任何形式的条目:

```toml
# ❌ 禁止
axcpu = { git = "https://github.com/<personal>/axcpu.git", branch = "..." }
axhal = { git = "https://github.com/<personal>/arceos.git", branch = "..." }
```

如果 Task #3 审计发现 B 类条目 (需要 arceos-org 上游变更), 处理流程:

1. 在对应 arceos-org 仓库开 PR (作者 = 发现者), 在 PR title 用
   `feat(<crate>): <change> for StarryOS eBPF/LKM support`。
2. 在 `TaskList` 加一条 "上游 PR <link>" 子任务, 用 `addBlockedBy`
   标在依赖它的迁移 task 上。
3. 待上游 merge + 发版后, 在迁移 PR 内更新 crate 版本号。
4. 如果时间紧急, **可** 临时用 git dep, 但必须:
   - 指向 `rcore-os` 组织下的分支, 不是个人 fork
   - 在 PR body 用 ⚠️ 标注 "blocked on upstream release of <crate>"
   - 该 PR 不得 merge 直到改回 crates.io 版本

### 4.2 ksym / kprobe / ktracepoint / kbpf-basic / kmod-loader

这五个 crate 已经发到 crates.io (源仓: Godones), 直接用版本号引用,
在 `workspace.dependencies` 声明 (`ebpf-kmod` 中的写法已是规范):

```toml
[workspace.dependencies]
ksym = "0.5"
kprobe = "0.5"
ktracepoint = "0.5"
kbpf-basic = "0.5"
kmod-loader = "0.2"
```

如果 crates.io 版本与 ebpf-kmod 用的版本不一致, 优先用 crates.io,
不一致带来的 API 差异在迁移 PR 内适配, 不回退 crates.io 版本。

### 4.3 `rbpf`

ebpf-kmod 用 `rbpf = "0.4"` (qmonnet/rbpf 上游), 直接照搬。

---

## 5. 移植步骤模板 (per-PR)

每个迁移 PR 都按这个模板执行。Task #5–#8 都引用本节。

### 5.1 准备

```bash
# 切分支
cd ~/tgoskits
git fetch rcore && git fetch origin
git checkout -b <new-branch> <stacking-base>

# 起手把 Source commit 当成可读引用 (不 cherry-pick)
cd ~/StarryOS && git show <commit> --stat > /tmp/source-files.txt
```

### 5.2 移植

- **逐文件**, 不要 `cp -r`。每个源文件: 读、理解、改写到目标布局、
  调用点适配 (`use` 路径、`feature` 门、错误类型)。
- 文件级 commit, commit message 用 Conventional Commits:
  `feat(starry-kernel): port <subsystem> from ebpf-kmod` /
  `chore(starry-kernel): rename bpf::tansform → bpf::transform`。
- 避免一个 commit 跨越多个子系统 (kprobe + bpf 混合改不要塞一个 commit)。

### 5.3 LKM 构建集成 (Task #6 专属)

- 不要把 `modules/kmod.mk` 直接搬进来。
- 新增 `scripts/axbuild/src/cmd/kmod.rs`, 暴露:
  ```
  cargo xtask starry kmod build --arch <arch> --module <name>
  cargo xtask starry kmod build --arch <arch> --all
  ```
- 内部仍可调用 `rustc -C panic=abort -C relocation-model=static …`
  实现 (`ebpf-kmod` Makefile 中已有完整命令行, 翻译过来即可),
  但入口必须是 xtask。

### 5.4 用户态程序位置 (Task #8 专属)

- Task #8 开工前在 `TaskList` 加一条 "决定 user-space eBPF 测试程序位置"
  的轻量 task, 给出二选一的对照 (`os/StarryOS/user/` vs
  `test-suit/starryos/ebpf/`) + 评估表, 推 PR / issue 让 reviewer 确认。
- 确认后再开 PR-D 主体。

---

## 6. 验证清单 (pre-push)

每个迁移 PR push 前必须全部过, 在 PR body 的 "## 验证" 小节贴最新输出。

### 6.1 格式

```bash
cd ~/tgoskits
cargo fmt --all -- --check
git diff --check
```

### 6.2 clippy (按 AGENTS.md, 受影响 crate 必须过)

```bash
cargo xtask clippy --since <merge-base>
cargo xtask clippy --package starry-kernel    # 11 个 feature 全过, 参考 #805
```

如果新增 crate, 同步:

```bash
# 把新 crate 加到 scripts/test/clippy_crates.csv
$EDITOR scripts/test/clippy_crates.csv
```

### 6.3 四架构 cross-build

```bash
for arch in aarch64 riscv64 x86_64 loongarch64; do
    cargo xtask starry build --arch $arch
done
```

`loongarch64` 是死结时显式标注 (`ebpf-kmod` 的 `Cargo.toml` 已专门
为 loongarch64 关 LTO, 迁移时观察是否仍需要)。

### 6.4 sync-lint

```bash
cargo xtask sync-lint --since <merge-base>
```

eBPF / kprobe / tracepoint 大量使用原子操作和 per-CPU 数据, 这条容易踩。

### 6.5 qemu 烟测 (PR-A / PR-B / PR-C / PR-D 必跑)

```bash
cargo xtask starry rootfs --arch x86_64
cargo xtask starry qemu --arch x86_64
# 在 qemu 内跑本 PR 引入的最小测试 (PR-D: ./kret; PR-C: insmod hello.ko)
```

### 6.6 PR body 与代码一致性 (照搬 WORKFLOW_LINUX_SEMANTICS §4.5)

最后一次 push 后**重读 body 全文**, 逐条对照 commit:

- 每个声称已实现的行为 → 找到对应代码位置
- 每个声称的验证结果 → 重新跑一遍贴最新输出
- 早期 TODO / 已知问题段落 → 删除或更新

---

## 7. PR description 模板

按 `AGENTS.md`: PR title 英文 Conventional Commits, body 中文。
基础结构 (PR-A 为例):

```markdown
type(scope): content
```

```markdown
Stacked on: rcore-os/tgoskits@<dev-sha>, #805@<sha>
Task: #5
Migration plan: os/StarryOS/docs/WORKFLOW_EBPF_LKM_MIGRATION.md

## 背景

承接 `ebpf-kmod` 迁移计划 (#244 / #306 / #446 / #673 / #805 之后),
本 PR 提供 eBPF 运行时的真正实现, 替换 #805 中 `sys_bpf` 与
`sys_perf_event_open` 的 stub。

## 变更内容

### 1. perf 文件
- ...

### 2. kbpf-basic 接入
- 实现 `KernelAuxiliaryOps` (位置: `os/StarryOS/kernel/src/ebpf/aux.rs`)
- 实现 `PerCpuVariantsOps` (位置: ...)

### 3. syscall 接线
- ...

## 设计决策

1. **为什么用 kbpf-basic 而非自写 verifier**: ...
2. **perf 文件层级**: 选择 procfs / debugfs 哪一种、为什么、Linux 对照
3. **与 #673 的 raw tracepoint attach 协议**: ...

## 上游 crate 依赖

- `kbpf-basic = "0.5"` (crates.io, 无 patch)
- 若 §4 中有 B 类条目, 列在此处

## 验证

- [x] `cargo fmt --all -- --check`
- [x] `cargo xtask clippy --package starry-kernel` (11 features)
- [x] `cargo xtask sync-lint --since <merge-base>`
- [x] `cargo xtask starry build --arch x86_64`
- [x] `cargo xtask starry build --arch riscv64`
- [x] `cargo xtask starry build --arch aarch64`
- [x] `cargo xtask starry build --arch loongarch64`
- [x] qemu x86_64 内运行 `kret` 用户程序, 见
      `os/StarryOS/docs/ebpf-migration/pr-A-qemu.log`

## 后续工作

- [ ] PR-D 接入更多用户态测试
- [ ] eBPF verifier 完善 (本 PR 暂用 kbpf-basic 默认)
- [ ] ...
```

---

## 8. 协作通信

### 8.1 工作日志

所有 reviewer 共享一份 `os/StarryOS/docs/ebpf-migration/journal.md`,
追加格式 (与 starry-harness `journal-entry.sh` 兼容):

```markdown
## 2026-05-21 — Task #4 integration base 建立
- author: <name>
- rcore/dev SHA: 6ff5a24bf
- pr-673-tp SHA: <sha>
- pr-805-ebpf-observability SHA: <sha>
- 冲突: kernel/src/lib.rs (两 PR 都加 mod 声明) — 解决: 保留并集, 排序
- 验证: 三架构 build 通过
- 推送: origin/feat/ebpf-integration-base @ <sha>
```

### 8.2 TaskList 用法

- 领任务: `TaskUpdate --owner <name> --status in_progress`
- 完成: `TaskUpdate --status completed`
- 阻塞: `TaskUpdate --addBlockedBy <other-task-id>` + 在 task 描述里
  写阻塞原因
- 新增子任务 (例如 Task #3 审出来需要上游 PR): `TaskCreate` +
  blockBy 主任务

### 8.3 跨 PR 沟通

- 在 #673 / #805 下评论时, 用中文 (与作者一致), 引用本工作流文档路径,
  解释依赖关系。
- 不在他人 PR 下做实质功能讨论 (那是他们的设计空间), 仅同步集成进度
  和接口确认。

---

## 9. 反模式黑名单 (一眼自检)

迁移 PR 出现下列任何一条 = 偏离本文档, reviewer 直接驳回:

- ❌ 在 `[patch.crates-io]` 引入个人 fork (`Godones/*` 等) — 见 §4.1
- ❌ 把 `Starry-OS/StarryOS` 的 `Makefile` / `make/` / `modules/kmod.mk`
  直接复制进来 — 必须走 `cargo xtask` (§5.3)
- ❌ 一个 PR 同时包含 PR-A 和 PR-B 的改动 — 必须按 §2.2 拆分
- ❌ `git cherry-pick 1488cf3` 直接合 — 基线差异太大, 必须按 §5.2 逐文件
- ❌ Force-push #673 / #805 的分支 — 见 §3.4
- ❌ PR body 缺 "Stacked on:" 行 — 见 §1.2
- ❌ PR body 用英文 / PR title 用中文 — 反 `AGENTS.md`
- ❌ "已经能 build" 当成验证结论 — 必须按 §6 跑全部 4 项
- ❌ 新加 crate 不更新 `scripts/test/clippy_crates.csv` — 反 `AGENTS.md`
- ❌ 在 Task #3 完成前就开始动 `Cargo.toml` 的依赖 — 容易引入个人 fork
- ❌ 不在 §8 journal 留记录就把 integration base 推到 origin

---

## 10. 案例对照

| 维度 | 本流程目标 | 反例 (假想) |
|---|---|---|
| 起点 | 锁 SHA 的 `rcore/dev` + 锁 SHA 的 #673/#805 | "最新 dev" 不锁 SHA, 一周后无法复现 |
| 移植粒度 | 按 §2.2 拆 4 个 PR | 一个 PR 6400 行, reviewer 弃读 |
| 依赖管理 | crates.io 版本 + 必要时 arceos-org 上游 PR | `[patch.crates-io]` 一行了之 |
| 构建系统 | 全部走 `cargo xtask` | 引入并行的 `Makefile` |
| 验证 | 4 架构 + clippy + sync-lint + qemu | 单架构 build 通过 |
| 协作 | TaskList + journal + 跨 PR 评论 | 各自闭门写, merge 时撞车 |

---

## 附录 A — 快速命令清单

```bash
# 抓上游
git -C ~/tgoskits fetch rcore dev
git -C ~/tgoskits fetch rcore pull/673/head:pr-673-tp
git -C ~/tgoskits fetch rcore pull/805/head:pr-805-ebpf-observability

# 看源
git -C ~/StarryOS log --oneline ebpf-kmod -5

# integration base
cd ~/tgoskits
git checkout -b feat/ebpf-integration-base rcore/dev
git merge --no-ff pr-673-tp
git merge --no-ff pr-805-ebpf-observability

# 验证
cargo fmt --all -- --check
cargo xtask clippy --package starry-kernel
for arch in aarch64 riscv64 x86_64 loongarch64; do
    cargo xtask starry build --arch $arch
done
cargo xtask sync-lint --since rcore/dev
cargo xtask starry rootfs --arch x86_64
cargo xtask starry qemu --arch x86_64

# 推送 + 开 PR
git push -u origin <branch>
gh pr create -R rcore-os/tgoskits -B dev -H LorenzLorentz:<branch> \
    -t "type(scope): content" \
    -F /tmp/pr-body.md
```

## 附录 B — 参考链接

- `WORKFLOW_LINUX_SEMANTICS.md` (同目录) — Linux 语义对齐 PR 流程
- `AGENTS.md` (仓根) — PR 标题/描述/clippy/工具链规范
- `CLAUDE.md` (仓根) — 架构与构建命令
- [JosephJoshua/starry-harness](https://github.com/JosephJoshua/starry-harness) — 方法论与脚本灵感
- [rcore-os/tgoskits PR #244](https://github.com/rcore-os/tgoskits/pull/244) — break/debug 异常
- [rcore-os/tgoskits PR #306](https://github.com/rcore-os/tgoskits/pull/306) — /proc/pid/maps
- [rcore-os/tgoskits PR #446](https://github.com/rcore-os/tgoskits/pull/446) — dynamic debug
- [rcore-os/tgoskits PR #673](https://github.com/rcore-os/tgoskits/pull/673) — tracepoint
- [rcore-os/tgoskits PR #805](https://github.com/rcore-os/tgoskits/pull/805) — kallsyms + kprobe + bpf stub
- [Starry-OS/StarryOS:ebpf-kmod](https://github.com/Starry-OS/StarryOS/tree/ebpf-kmod) — source
- crates: [ksym](https://crates.io/crates/ksym) · [kprobe](https://crates.io/crates/kprobe) · [ktracepoint](https://crates.io/crates/ktracepoint) · [kbpf-basic](https://crates.io/crates/kbpf-basic) · [kmod-loader](https://crates.io/crates/kmod-loader)
