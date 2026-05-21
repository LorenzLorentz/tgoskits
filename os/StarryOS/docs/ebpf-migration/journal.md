# eBPF / LKM 迁移工作日志

按时间倒序追加 (最新在最上)。格式见
`os/StarryOS/docs/WORKFLOW_EBPF_LKM_MIGRATION.md` §8.1。

---

## 2026-05-21 — Task #1 抓取上游分支

- author: claude (代 LorenzLorentz)
- 操作:
  ```bash
  git fetch rcore dev
  git fetch rcore pull/673/head:pr-673-tp
  git fetch rcore pull/805/head:pr-805-ebpf-observability
  ```
- 锁定 SHA:
  - `rcore/dev` = `a2ea1e271839044712f7400355f934db6352f896`
    ("Enhance CI workflow output and optimize test configurations (#832)")
  - `pr-673-tp` = `91a2499a6a7e81a09a6acc7837cc78cc23f2ee40`
    ("Merge branch 'dev' into feat/tp", 6 commits ahead of dev)
  - `pr-805-ebpf-observability` = `1d7d51b1e2e50841c7c14c0e76475faf1a8345d8`
    ("feat(starry-kernel): implement TrapFrame<->PtRegs conversion and eBPF
    map/prog framework", 2 commits ahead of dev)
- 备注: PR #673 最近一次 merge dev 是在自身 head, 说明 author 已主动跟上
  上游, integration base 的冲突面应该可控。PR #805 没 merge 过 dev,
  base 在 `9a5e44b79` (旧于 dev HEAD 约 30 commit), Task #4 时需要先
  rebase 或合 dev 再 merge。
- Task 状态: #1 completed.

---

## 2026-05-21 — Task #2 差异审计完成

- author: claude
- 产出: `os/StarryOS/docs/ebpf-migration/diff-audit.md`
- 关键发现:
  - 已并入 dev: PR #244 / #306 / #446
  - 已在在途 PR 内: tracepoint (#673), kallsyms/kprobe/bpf-stub (#805)
  - 真正剩余移植 ≈ 3000 行内核代码 (PR-A/B 主体), 加 user/musl ~6000 行
    (PR-D, 含大量 Cargo.lock)
  - 决策点 6 项, 包括: 不移植 `kallsym.ld` (PR #805 用 nm-at-build-time
    路线), 不移植 `kprobe/arch/*.rs` (用 `kprobe` crate), 修正
    `tansform.rs` → `transform.rs` 拼写
- Task 状态: #2 completed.

---

## 2026-05-21 — Task #3 crate fork 审计完成

- author: claude
- 产出: `os/StarryOS/docs/ebpf-migration/crate-fork-audit.md`
- 关键发现: **ebpf-kmod 的 5 条 `[patch.crates-io]` 在 tgoskits 全部
  归为 D 类 (不需要)**。原因: tgoskits 把 `axcpu`/`axhal`/`axalloc`/
  `page_table_multiarch`/`page_table_entry` 全部 vendor 到仓内, 且
  package 已重命名为 `ax-*` (`ax-cpu` v0.6.3 / `ax-hal` v0.5.14 等),
  crates.io 上的同名 crate 在 tgoskits 依赖闭包没有任何引用点。
  Godones fork 的 44 个 commit 中, 没有任何一项是 "tgoskits 缺、必须
  上游 PR" 的硬阻塞 (POST_TRAP / pre_trap / user_copy / loongarch
  unaligned 等都和本迁移无关或已有等价替代)。
- 后续: PR-A 第一次 `cargo build` 时跑 `cargo tree -i axhal`, 验证
  crates.io 上的 `kbpf-basic` 是否引入对原始名字的传染性依赖; 若是,
  追加一条 path 形式的 `[patch.crates-io]` 指回本仓 vendor。
- Task 状态: #3 completed.

---

## 2026-05-21 — Task #4 integration base 建立

- author: claude
- 分支: `feat/ebpf-integration-base`, HEAD `8ad9a0d09` (合并后); 后追加
  docs commit, 最终 HEAD = 见 `git rev-parse feat/ebpf-integration-base`
- 步骤:
  1. `git checkout -b feat/ebpf-integration-base rcore/dev` (起点
     `a2ea1e271`)
  2. `git merge --no-ff pr-673-tp` — 干净, 自动合并 Cargo.lock 与
     `syscall/fs/fd_ops.rs`, 无冲突, commit `83dcc33a9`
  3. `git merge --no-ff pr-805-ebpf-observability` — 2 个冲突:
     - `kernel/src/entry.rs`: 两 PR 都在 `init` 中 init_*; 解决方式
       保留双方 (kallsyms_init + tracepoint_init), kallsyms 在前
     - `Cargo.lock`: starry-kernel 依赖列表合并 ktracepoint(673) +
       kprobe(805)
     commit `8ad9a0d09`
  4. 追加 docs commit (workflow + Phase 1 audits + journal)
- 验证:
  - `cargo fmt --all -- --check` ✅ (无输出)
  - `cargo xtask starry build --arch x86_64` ✅ (1m 11s, 静态 ELF + bin)
  - `cargo xtask starry build --arch aarch64` ✅ (~1m)
  - `cargo xtask starry build --arch riscv64` ✅ (1m 05s)
  - loongarch64 未跑 (Task #4 范围内三架构, loongarch64 留给具体 PR)
  - `cargo xtask clippy --package starry-kernel` ❌ 11/11 fail, 但
    **rcore/dev pristine 也同样 fail** (Mac aarch64 host 上的 ax-percpu
    内联汇编与 const trait 报错), 不是 merge 引入。本机 host 上的
    clippy 暂不可用, 后续依赖 CI 上的 Linux runner。
- Task 状态: #4 completed。

---

## 2026-05-21 — 分支整理与推送 (Push to origin)

- author: claude
- 用户决策: docs 只本地用, 不上游 → 把 docs commits 从 integration base
  剥离, 留作独立 `docs/ebpf-migration-local` 分支。
- 操作:
  1. `git branch docs/ebpf-migration-local` (在 `f9ebd62ca` 处保留 docs)
  2. `git reset --hard 8ad9a0d09` (integration base 回到 merges-only)
  3. `git push origin feat/ebpf-integration-base:feat/ebpf-integration-base`
  4. `git push -u origin docs/ebpf-migration-local`
- 远端结果:
  - `origin/feat/ebpf-integration-base` @ `8ad9a0d09` — 干净 stacking parent
    (仅含 #673 + #805 merge), 是 PR-A/B 的 base
  - `origin/docs/ebpf-migration-local` @ `f9ebd62ca` (+ 本 commit) — 含
    workflow + audits + journal 的本地协作分支, 不会进 upstream
  - tracking 已修正为各自的 origin 分支 (之前 `feat/ebpf-integration-base`
    误 track 到 `rcore/dev`, 触发 "push to dev rejected" — 已修正,
    没有任何 force-push 到 dev 发生)
- 下一步: 用户审阅 Phase 0-2 产出后再决定启动 Task #5/#6。

---

## 2026-05-21 — Task #5 PR-A: eBPF 运行时移植落地

- author: claude (代 LorenzLorentz)
- 分支: `feat/starry-ebpf-runtime` @ `314ee11e5` (起点 `8ad9a0d09`,
  从 `feat/ebpf-integration-base` 切出)
- 单 commit, +1873/-239, 18 文件:
  - `os/StarryOS/kernel/src/ebpf.rs` 删除, 改为 `ebpf/` 目录
    (mod / map / prog / transform), 替换 #805 的 stub.
  - 新增 `os/StarryOS/kernel/src/perf/` (mod / bpf / kprobe /
    tracepoint / raw_tracepoint / uprobe), 共 6 文件.
  - 新增 `os/StarryOS/kernel/src/lock_api.rs` (KSpinNoPreempt<T>
    包装, 满足 `lock_api::RawMutex`, 供 kprobe::Uprobe 等需要 lock_api
    风格类型参数的 API 使用).
  - `kprobe.rs` 增量: 暴露 KernelKprobe / KernelKretprobe /
    KprobeAuxiliary 别名 + register_/unregister_kprobe/kretprobe.
  - `tracepoint/mod.rs` 增量: lookup_ext_tracepoint /
    find_ext_tracepoint_by_name 公开函数.
  - `entry.rs`: 追加 ebpf::init_ebpf() 与 perf::perf_event_init().
  - `Cargo.toml`: 启用 kbpf-basic = "0.5", 新增 rbpf 0.4.
- 关键 API 适配:
  - 全部 `axhal/axalloc/axmm/axkspin/...` → `ax_hal/ax_alloc/ax_mm/
    ax_kspin/...` (crate-fork-audit §6).
  - 拼写: 源 `bpf/tansform.rs` → `ebpf/transform.rs`, commit 说明.
  - ktracepoint 0.5 → 0.6: 旧 `TracePoint::register_event_callback
    (id, callback)` 替换为 `ExtTracePoint::register
    (TraceCallbackType::Event(Arc<TraceEventFunc>))`. raw tp 同理.
  - kallsyms 查询走 #805 的 `crate::kallsyms::lookup_name`, 不再
    依赖源仓库的 `pseudofs::KALLSYMS`.
  - BpfError ↔ AxError 显式映射 (kbpf-basic 的 axerrno 与 tgoskits
    的 ax-errno 是两个不同的 crate, 即使语义同源).
- 范围裁剪 (留给后续 PR):
  - PERF_TYPE_UPROBE 暂返回 Unsupported — 依赖 ProcessData 的
    uprobe_manager / uprobe_point_list 字段与 AddrSpace::memoryset
    accessor, 两者均未在 #805 引入. (待 PR-A-followup 或 PR-B
    顺手扩 task/ProcessData.)
  - BpfPerfEvent::do_mmap 路径不接 FileLike (tgoskits 的 FileLike
    无源仓库的 custom_mmap hook), ringbuf mmap 后续 PR 处理.
  - eBPF procfs 节点 (bpf_stats_enabled 等) 未涉及.
- ⚠️ **上游阻塞**: kbpf-basic 0.5.5 传递依赖 `printf-compat = "0.3"`,
  printf-compat 0.3.1 在 nightly-2026-04-27 不能编译
  (`core::ffi::VaList::arg` API 已重命名/移除). printf-compat 0.4.0
  已在 crates.io 但 semver 不兼容, 必须等 Godones/ext_ktrace 把
  kbpf-basic 的依赖升到 0.4. PR-A merge 等这件事.
  - 临时验证选项: vendor printf-compat 0.4 到 components/ 并 path-patch
    (workflow §4.1 允许, 因为不是个人 fork). 用户选择不做, 直接提交
    代码 + 在 PR body 标 blocked.
- 验证状态:
  - `cargo fmt --package starry-kernel -- --check` ✅
  - `cargo check --package starry-kernel --target x86_64-unknown-none`
    ❌ blocked on printf-compat 0.3 编译失败 (上游问题, 与本 PR 改动
    无关).
  - 三架构 build / clippy 11 features / sync-lint / qemu 烟测均
    blocked on 同一问题.
- 下一步:
  1. 在 ext_ktrace 仓 (kbpf-basic / printf-compat 0.4) 推 patch.
  2. 待上游发版后回本 PR 重跑 §6 全套验证.
  3. 可与 Task #6 (PR-B LKM) 并行启动, base 同样是
     `feat/ebpf-integration-base` (不依赖 PR-A 改动).
- Task 状态: #5 in_progress (待上游解锁后完成验证); PR-B 仍 pending,
  可领。

---

## 2026-05-21 — Task #6 PR-B: LKM (kmod-loader) 移植落地

- author: claude (代 LorenzLorentz)
- 分支: `feat/starry-lkm` @ `2a115432e` (起点 `8ad9a0d09`, 与 PR-A
  并行从 `feat/ebpf-integration-base` 切出, 不依赖 PR-A 改动)
- 单 commit, 11 文件:
  - 新增 `os/StarryOS/kernel/src/kmod/` (mod.rs + kprint.rs).
  - 新增 `os/StarryOS/kernel/src/syscall/kmod.rs` (init_module /
    finit_module / delete_module).
  - `syscall/mod.rs`: 注册三条 syscall 分派.
  - `lib.rs`: 新增 `mod kmod;`.
  - `entry.rs`: 追加 `crate::kmod::init_kmod()`.
  - `Cargo.toml`: 启用 kmod (kmod-tools) / kmod-loader / lwprintf-rs.
  - 新增 `os/StarryOS/scripts/kmod-linker.ld` (从源仓
    modules/kmod-linker.ld 字节级移植).
  - 新增 `scripts/axbuild/src/starry/kmod.rs` + 注入到 starry/mod.rs
    Command 枚举: 暴露 `cargo xtask starry kmod build --arch <arch>
    [--module <path> | --all]` 子命令.
- 关键决策:
  - **不引入 Makefile / make/ / modules/kmod.mk** (workflow §5.3
    硬要求). 取而代之是 `cargo xtask starry kmod build`, 内部串
    `cargo build` + `rust-lld -r -T kmod-linker.ld --whole-archive`.
  - shim/{block,mq,xarray}.rs **不移植**: 源仓 kmod/mod.rs 用
    `// mod shim;` 显式注释禁用, block/mq 是 null_blk 的 WIP, 与
    PR-B 完成判据无关. 把这件事留给 PR-C 或 PR-B-followup, 配合
    null_blk 示例再补.
  - KALLSYMS 查询走 #805 `crate::kallsyms::lookup_name`, 不走源仓
    `pseudofs::KALLSYMS`. 与 PR-A 一致.
  - syscall 入口扁平为 `syscall/kmod.rs` 而非 `syscall/kmod/mod.rs`,
    保持 tgoskits 小子系统的现有命名风格 (cf. `syscall/signal.rs`).
- 验证状态:
  - `cargo fmt --all -- --check` ✅
  - `cargo check -p axbuild` ✅ (xtask 编译通过)
  - `cargo check -p starry-kernel --target x86_64-unknown-none`
    ❌ blocked: lwprintf-rs 0.3.3 build.rs 调 `gcc -print-sysroot`,
    本地 Mac 没装 cross-toolchain. CI runner 上的 Linux 镜像
    (ghcr.io/rcore-os/tgoskits-container) 应当有, 等 CI 验证.
  - 同样依赖 nightly `#![feature(c_variadic)]`, 若 nightly-2026-04-27
    上 bindgen 生成的 VaList 用法与新 API 不兼容, 还需追加 patch
    (与 PR-A 的 printf-compat 同类问题).
- ⚠️ **Blocked**: 与 PR-A 类似但是更轻 — 不是 semver 不兼容, 而是
  build 环境差异 (gcc cross-compile). 代码本身可独立 review, 等
  CI Linux runner 跑出更精确的状态.
- 不在本 PR 范围:
  - shim/{block,mq,xarray}.rs (源仓也禁用).
  - modules/{hello,kebpf} 示例 (Task #7 / PR-C).
  - qemu 内 insmod hello.ko 烟测 (依赖 PR-C 提供 hello 模块).
- 下一步:
  1. CI Linux 上跑 `cargo xtask starry build --arch x86_64` 验证
     kernel 含 kmod 子系统能正常编译.
  2. 与 PR-C (kmod 示例) 同步推, 形成 "kmod 内核侧 + 示例" 完整链.
- Task 状态: #6 in_progress (待 CI 跑出验证), 可与 PR-A 并行
  review.

---
