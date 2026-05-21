# Crate Fork Audit: ebpf-kmod 的 [patch.crates-io] 是否需要在 tgoskits 重现

Task #3 产出。审计 `Starry-OS/StarryOS:ebpf-kmod` 在 `Cargo.toml` 中
声明的 5 条 `[patch.crates-io]` 是否需要被移植到 tgoskits。

## 一句话结论

**全都不需要。** tgoskits 把 `axcpu` / `axhal` / `axalloc` /
`page_table_multiarch` / `page_table_entry` 全部 vendor 到仓内, 并且
**重命名了 package**, 不再依赖 crates.io 上的同名 crate。
ebpf-kmod 通过 `[patch.crates-io]` 引入的 Godones fork 在 tgoskits
的依赖图里完全没有触达点。

工作流文档 §4.1 的硬约束 (禁止个人 fork patch) 因此**没有任何
正当的例外**, 任何迁移 PR 出现 `[patch.crates-io] axcpu = ...` 直接驳回。

---

## 1. ebpf-kmod 的原始声明

```toml
# Starry-OS/StarryOS:ebpf-kmod  Cargo.toml (root)
[patch.crates-io]
axcpu                = { git = "https://github.com/Godones/axcpu.git", branch = "0.3.0.preview.9" }
axhal                = { git = "https://github.com/Godones/arceos.git", branch = "dev" }
axalloc              = { git = "https://github.com/Godones/arceos.git", branch = "dev" }
page_table_multiarch = { git = "https://github.com/Godones/page_table_multiarch.git", branch = "dev" }
page_table_entry     = { git = "https://github.com/Godones/page_table_multiarch.git", branch = "dev" }
```

锁定 SHA (审计时间 2026-05-21):

| Fork branch | HEAD |
|---|---|
| `Godones/axcpu:0.3.0.preview.9` | `e052b894ce0b611081a5b8bba1d74a136602aa53` |
| `Godones/arceos:dev` | `c94a740cc17d0d03abf323e347fdcb7743487864` |
| `Godones/page_table_multiarch:dev` | `300328f5ffcb97f45f184a6c620d765f834a7cfe` |

`Godones/axcpu:0.3.0.preview.9` 相对 `arceos-org/axcpu:main` 领先 **44 个 commit**
(从 `feat: improve context with more api` 到 `feat: improve backtrace support`,
多为 user_copy、exception kind、unaligned access、breakpoint ReturnReason、
exception table、POST_TRAP/pre_trap callback 等)。

---

## 2. tgoskits 对这五个 crate 的处理

| crates.io 名 | tgoskits 实际位置 | tgoskits 内 package 名 | repository 字段 |
|---|---|---|---|
| `axcpu` | `components/axcpu/` | `ax-cpu` v0.6.3 | `rcore-os/tgoskits` |
| `axhal` | `os/arceos/modules/axhal/` | `ax-hal` v0.5.14 | `rcore-os/tgoskits` |
| `axalloc` | `os/arceos/modules/axalloc/` | `ax-alloc` v0.7.1 | `rcore-os/tgoskits` |
| `page_table_multiarch` | `components/page_table_multiarch/page_table_multiarch/` | `ax-page-table-multiarch` v0.8.9 | `rcore-os/tgoskits` |
| `page_table_entry` | `components/page_table_multiarch/page_table_entry/` (同上 workspace) | `ax-page-table-entry` | `rcore-os/tgoskits` |

**关键事实**: package 名都加了 `ax-` 前缀。这意味着:

- tgoskits 内的 `Cargo.toml` 用 `ax-cpu = "0.6.3"` (或 `ax-cpu.workspace = true`)
  引用, **不存在** `axcpu = "..."` 的依赖。
- crates.io 上的 `axcpu` (原 arceos-org 项目, Godones fork 的基准) 在
  tgoskits 的依赖闭包里**找不到任何引用点**。
- `[patch.crates-io] axcpu = ...` 因此是 dead patch, cargo 不会用它。

`starry-kernel` (`os/StarryOS/kernel/Cargo.toml`) 里所有相关依赖都按
tgoskits 的命名:

```toml
ax-hal.workspace = true
ax-alloc.workspace = true
# 没有 axcpu / axhal / axalloc / page_table_* 字面量
```

---

## 3. ebpf-kmod 实际使用的外部 API 清单

通过 `grep -rE "use (axcpu|axhal|axalloc|page_table)"` 扫
`kernel/src/{kprobe,perf,bpf,uprobe,kmod,tracepoint,exception,lock_api}/`,
ebpf-kmod 真正用到的外部 API 只有以下少数项:

| API | 位置 | tgoskits 提供方 | 是否已可用 |
|---|---|---|---|
| `axhal::context::TrapFrame` | kprobe / uprobe / exception | `ax-cpu::TrapFrame` (`ax-hal` re-export 同名) | ✅ 已可用 (PR #805 已验证) |
| `axhal::paging::PageSize` | perf/bpf, bpf/map | `ax-hal::paging::PageSize` | ✅ |
| `axhal::paging::MappingFlags` | tracepoint | `ax-hal::paging::MappingFlags` | ✅ |
| `axhal::trap::BREAK_HANDLER` (旧静态) | exception | `#[ax_hal::trap::breakpoint_handler]` 属性宏 (PR #244 引入新形式) | ✅ 新接口, PR #805 已用 |
| `axhal::trap::DEBUG_HANDLER` (旧静态) | exception | `#[ax_hal::trap::debug_handler]` 属性宏 (PR #244 引入新形式) | ✅ 新接口, PR #805 已用 |
| `axhal::mem::phys_to_virt` | kmod | `ax-hal::mem::phys_to_virt` | ✅ |
| `axhal::percpu::this_cpu_id` | tracepoint | `ax-hal::percpu::this_cpu_id` (或 axplat) | ✅ |
| `axhal::time::monotonic_time_nanos` | tracepoint | `ax-hal::time::monotonic_time_nanos` | ✅ |
| `axalloc::UsageKind` | kmod | `ax-alloc::UsageKind` | ✅ |

**没有** 使用 `page_table_multiarch::*` 或 `page_table_entry::*` 的直接 import
(只通过 `axhal::paging` 间接访问)。

**结论**: ebpf-kmod 用到的所有 axhal/axalloc/axcpu API 在 tgoskits 已经
存在, 且接口语义相同 (或更先进, 如 PR #244 把 break/debug handler 从
全局静态函数指针改成属性宏注册)。

---

## 4. Godones fork 中那 44 个 commit 都在做什么? 是否 tgoskits 缺?

按 commit 主题归类:

| 主题 | commit 数 | tgoskits 是否需要? |
|---|---|---|
| POST_TRAP / pre_trap callback | 2 | ❌ 不需要 — ebpf-kmod 的 kprobe 通过 #244 风格的 breakpoint_handler 属性宏即可达成同样目的, 不依赖 POST_TRAP |
| backtrace 改进 | 3 | ❌ tgoskits 有独立 `axbacktrace`, 不沿用 axcpu 内置 |
| user_copy & exception table | 4 | 🟡 部分有用 — tgoskits 已有 `starry_vm::vm_read_slice` 之类替代, kprobe 当前已用; 暂时不必移植 axcpu 内的 user_copy |
| loongarch64 unaligned access | 3 | 🟡 与 eBPF 无关, 单独议题; ebpf-kmod 顺带; 不进本迁移 |
| breakpoint / debug ReturnReason | 1 | ✅ 等价能力已在 tgoskits via PR #244 (属性宏) |
| user space 重设计 (`feat: redesign user space`) | 1 | ❌ tgoskits 已有自己的 user space 设计, 不引入 |
| 各架构 trap.S 改写 / TLB / 4-level fix | ~20 | ❌ 多为 fork 内部维护, tgoskits 各架构已独立维护 |
| chore / refactor / ci | ~10 | ❌ 与功能无关 |

没有任何一项是 "tgoskits 缺、必须 PR 到 arceos-org" 的硬阻塞。

---

## 5. 工作流文档 §4 的 ABCD 分类对照

按 §4 给定的 4 类:

- **A. 已在 tgoskits 上游存在**: 全部 9 个 API 都在 tgoskits 内, ✅
- **B. 需要 PR 到 arceos-org**: 0 个
- **C. tgoskits 本地适配可吸收**: 0 个 (没有需要适配的)
- **D. 不需要 (旧实现已被替代)**: 5 条 patch 全部归入 D 类

5 条 patch 全部 D 类, 无需任何上游 PR, 无需任何 tgoskits 本地修改。

---

## 6. 写进迁移 PR 的实操指引

每个迁移 PR (PR-A / PR-B / PR-C / PR-D) 操作时:

1. **不要** 在根 `Cargo.toml` 或 `os/StarryOS/Cargo.toml` 加任何
   `[patch.crates-io]` 指向 `Godones/*`。
2. 移植 ebpf-kmod 文件时, 把原本 `use axhal::context::TrapFrame` 之类
   import 改为 `use ax_hal::context::TrapFrame` (注意 crate 名带连字符的
   情况下, `use` 里改为下划线: `use ax_hal::...`)。
3. ebpf-kmod 自带的 `Cargo.lock` (+475 行) 不直接拷贝, 让 cargo 在
   tgoskits workspace 内重新解析。
4. 如果 cargo 在解析时报 `axcpu`/`axhal` 不存在, 说明误用了 crates.io 名,
   按上文映射改成 `ax-*`。
5. ebpf-kmod 的 `Cargo.toml` 中 `[workspace.dependencies]` 部分对 `ksym` /
   `kprobe` / `ktracepoint` / `kbpf-basic` / `kmod-loader` / `rbpf` 的
   声明可以直接照搬 (这些是 crates.io 真正发布的 crate, 不属于 fork 范畴)。
6. 如果 ebpf-kmod 的某个外部 crate (`kbpf-basic` 等) 内部 *再* `[patch]`
   了 `axhal`/`axcpu`, 那是该 crate 的事, 与 tgoskits 无关 (该 crate
   在 tgoskits 里走自己的 git/crates.io 拉取闭包, 不会传染到 tgoskits 的
   依赖解析)。

---

## 7. 风险与剩余 unknowns

- `kbpf-basic = "0.5"` 自身 (crates.io 上发布的版本) 是否对 axhal/axcpu
  有源码级假设 (例如直接 `extern crate axhal`)? **需要在 PR-A 第一次
  `cargo build` 时验证**。若 crates.io 版本的 `kbpf-basic` 在
  `Cargo.toml` 内显式 `axhal = "..."`, 因为 tgoskits 没有 `axhal` 而
  只有 `ax-hal`, cargo 会到 crates.io 拉真正的 `axhal`, 此时可能引入
  不一致。**预防措施**: 第一次 build 后跑 `cargo tree -i axhal`, 如有命中
  在 PR-A 内通过添加 cargo `[patch.crates-io] axhal = { path = "os/arceos/modules/axhal" }`
  把它指回本地 vendor (这种 path 类 patch 不违反 §4.1, 因为指向的是
  tgoskits 自己的目录, 不是个人 fork)。
- 如果上述 path patch 还需要修改 `ax-hal` 的 `package` 字段 (从 `ax-hal`
  改回 `axhal`) 才能匹配 crates.io 名字, 那不是 PR-A 能解决的, 必须
  开一个独立的 PR 改名 (但更可能的是用 `package = "ax-hal"` syntax
  在 `[patch.crates-io]` 里就够了, 见 cargo docs)。
- 这一切都是"可能", 实际 PR-A 第一次 build 出现时再处理。在 build 不
  出问题之前不预先动手 (§9 反模式: "在 Task #3 完成前就动 Cargo.toml")。

---

## 8. 工作流文档需要更新的点

无。文档 §4 的写法 (默认禁止 + ABCD 分流 + B 类时开上游 PR) 在本审计
下完全适用, 只是本次审计结论恰好是 "全部 D 类"。

如果未来 PR-A 实际 build 时触发上文 §7 的 unknown, 在 journal.md 追加
新 entry, 并视情况新建 task。
