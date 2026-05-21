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
