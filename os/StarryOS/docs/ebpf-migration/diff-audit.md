# Diff Audit: ebpf-kmod → tgoskits

Task #2 产出。审计 `Starry-OS/StarryOS:ebpf-kmod` 三个 commit 与
`rcore-os/tgoskits` (dev + 在途 PR #673 + #805) 的覆盖关系。

## 锁定 SHA (与 journal.md 同步)

| Source / Target | SHA | 备注 |
|---|---|---|
| Source `ebpf-kmod` HEAD | `3fed46e` | 含 `6278553` (user/musl), `1488cf3` (内核+LKM), `3fed46e` (deps 微调) |
| Target `rcore/dev` | `a2ea1e271` | "Enhance CI workflow output …" (#832) |
| Target `pr-673-tp` | `91a2499a6` | tracepoint, 6 commit ahead, 已 merge 过 dev |
| Target `pr-805-ebpf-observability` | `1d7d51b1e` | kallsyms+kprobe+bpf stub, 2 commit ahead |

## Source 文件清单总览

`1488cf3` 共 76 文件 / +6421 行 / -165 行 (含 Makefile / deps / kernel)。
`6278553` 共 7 个 user 程序 (`async_test` / `kret` / `mytrace` / `rawtp` /
`syscall_ebpf` / `upb` / `upb2`), 每个含 `*-common` / `*-ebpf` / `*-userspace`
三 crate 三件套。
`3fed46e` 只动 3 文件 (Cargo.lock/toml + make/cargo.mk), 删 LTO 强制项。

## 覆盖矩阵

图例: ✅ 已覆盖 / 🔄 #PR 覆盖部分 / ❌ 未覆盖 / ⛔ 不移植 (有理由) /
🔧 需要适配 (源在 tgoskits 已有但语义/路径不同)

### A. 内核根级修改

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `.gitignore` | +1 | `.gitignore` | 🔧 | 各 PR 顺手 | `*.ko` 等需要时再加 |
| `.vscode/settings.json` | +2 | n/a | ⛔ | — | 个人 IDE 配置, 不入库 |
| `Cargo.lock` | +475 | `Cargo.lock` | 🔧 | 各 PR 自动 | cargo 重新解析 |
| `Cargo.toml` (root) | +24 | `Cargo.toml` | 🔧 | PR-A/B/C | 仅加 `workspace.dependencies` 行, **不加 [patch.crates-io]** |
| `Makefile` | +47 | n/a | ⛔ | — | tgoskits 走 cargo xtask, §5.3 |
| `TODO.md` | +4 | n/a | ⛔ | — | 老仓 TODO, 已过时 |
| `docs/eBPF for Starry.md` | +232 | `os/StarryOS/docs/ebpf-howto.md` | ❌ | PR-A | 改写为用户使用文档 |
| `docs/null_blk.md` | +128 | `os/StarryOS/docs/lkm-null-blk.md` | ❌ | PR-B | 改写为示例文档 |
| `make/Makefile` | +25 | n/a | ⛔ | — | 同 Makefile |
| `make/build.mk` | +27 | `scripts/axbuild/` 内吸收 | ⛔ | — | 翻译进 xtask |
| `make/cargo.mk` | +14 | `scripts/axbuild/` 内吸收 | ⛔ | — | 同上 |

### B. kernel/src/bpf/ (eBPF map + prog + transform)

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `kernel/src/bpf/map.rs` | +142 | `os/StarryOS/kernel/src/ebpf/` (新建子模块) | ❌ | PR-A | #805 仅有 `ebpf.rs` 单文件 stub, 不含 map 实现 |
| `kernel/src/bpf/mod.rs` | +17 | 合并进 `ebpf.rs` 或拆 `ebpf/mod.rs` | ❌ | PR-A | 注意目录命名: #805 用 `ebpf.rs`, 改成 `ebpf/` |
| `kernel/src/bpf/prog/mod.rs` | +66 | `os/StarryOS/kernel/src/ebpf/prog.rs` | ❌ | PR-A | — |
| `kernel/src/bpf/tansform.rs` | +242 | `os/StarryOS/kernel/src/ebpf/transform.rs` | ❌ | PR-A | **拼写修正** tansform→transform, 在 commit message 说明 |

### C. kernel/src/perf/ (整个子模块全新)

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `kernel/src/perf/bpf.rs` | +164 | `os/StarryOS/kernel/src/perf/bpf.rs` | ❌ | PR-A | 实现 bpf perf event |
| `kernel/src/perf/kprobe.rs` | +238 | `os/StarryOS/kernel/src/perf/kprobe.rs` | ❌ | PR-A | 复用 #805 已加的 `kprobe.rs` (是 `kprobe::*` crate 适配层) |
| `kernel/src/perf/mod.rs` | +205 | `os/StarryOS/kernel/src/perf/mod.rs` | ❌ | PR-A | perf_event_open dispatch |
| `kernel/src/perf/raw_tracepoint.rs` | +112 | `os/StarryOS/kernel/src/perf/raw_tracepoint.rs` | ❌ | PR-A | 接 #673 的 tracepoint 框架 |
| `kernel/src/perf/tracepoint.rs` | +155 | `os/StarryOS/kernel/src/perf/tracepoint.rs` | ❌ | PR-A | 同上 |
| `kernel/src/perf/uprobe.rs` | +96 | `os/StarryOS/kernel/src/perf/uprobe.rs` | ❌ | PR-A | — |
| `kernel/src/syscall/perf.rs` | +23 | 改写 `ebpf.rs` 中 `sys_perf_event_open` | ❌ | PR-A | #805 留了 stub, PR-A 接通到 perf 子模块 |

### D. kernel/src/kprobe/ (源是多文件, tgoskits 已被 #805 合成单文件)

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `kernel/src/kprobe/arch/{aarch64,riscv,x86_64,loongarch64,mod}.rs` | +208 (合计) | n/a | 🔄 | — | #805 直接用 `kprobe = "0.5"` crate, 架构差异封装在 crate 内, 无需移植 |
| `kernel/src/kprobe/kprobe_test.rs` | +102 | `os/StarryOS/kernel/tests/kprobe.rs` 或 PR-A test | ❌ | PR-A | 移植为集成测试 |
| `kernel/src/kprobe/mod.rs` | +72 | `os/StarryOS/kernel/src/kprobe.rs` | ✅ | #805 | #805 已实现 + 加了 TrapFrame↔PtRegs 转换 |
| `kernel/src/kprobe/probe_aux.rs` | +215 | `os/StarryOS/kernel/src/kprobe.rs` 内联 | ✅ | #805 | KernelKprobeOps 已在 #805 |

### E. kernel/src/tracepoint/ (在 #673 中实现, 部分对得上)

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `kernel/src/tracepoint/event.rs` | +72 | n/a | ❌ | PR-A | #673 没有 event.rs, 但 raw tracepoint 需要; PR-A 时按需补 |
| `kernel/src/tracepoint/mod.rs` | +235 | `os/StarryOS/kernel/src/tracepoint/mod.rs` | ✅ | #673 | 接口形式不同, raw tp open 在 PR-A 接 |
| `kernel/src/tracepoint/trace.rs` | +76 | `os/StarryOS/kernel/src/tracepoint/trace.rs` | ✅ | #673 | — |
| `kernel/src/tracepoint/trace_pipe.rs` | +81 | `os/StarryOS/kernel/src/tracepoint/trace_pipe.rs` | ✅ | #673 | — |

### F. kernel/src/kmod/ (LKM 主体)

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `kernel/src/kmod/mod.rs` | +184 | `os/StarryOS/kernel/src/kmod/mod.rs` | ❌ | PR-B | 接 `kmod-loader = "0.2"` |
| `kernel/src/kmod/shim/block.rs` | +149 | `os/StarryOS/kernel/src/kmod/shim/block.rs` | ❌ | PR-B | — |
| `kernel/src/kmod/shim/kprint.rs` | +103 | `os/StarryOS/kernel/src/kmod/shim/kprint.rs` | ❌ | PR-B | — |
| `kernel/src/kmod/shim/mod.rs` | +242 | `os/StarryOS/kernel/src/kmod/shim/mod.rs` | ❌ | PR-B | — |
| `kernel/src/kmod/shim/mq.rs` | +961 | `os/StarryOS/kernel/src/kmod/shim/mq.rs` | ❌ | PR-B | 最大单文件, 注意 review 难度 |
| `kernel/src/kmod/shim/xarray.rs` | +12 | `os/StarryOS/kernel/src/kmod/shim/xarray.rs` | ❌ | PR-B | — |
| `kernel/src/syscall/kmod/mod.rs` | +69 | `os/StarryOS/kernel/src/syscall/kmod.rs` | ❌ | PR-B | init_module / finit_module / delete_module |

### G. kernel/src/uprobe/

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `kernel/src/uprobe/mod.rs` | +55 | `os/StarryOS/kernel/src/uprobe.rs` | ❌ | PR-A | 作为 perf/uprobe.rs 的依赖, 一起进 PR-A |

### H. kernel 其他散点

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `kernel/src/lock_api.rs` | +58 | `os/StarryOS/kernel/src/lock_api.rs` 或合并到现有 spinlock | ❌ | PR-A 或基础设施 | `KernelRawMutex` 已部分在 #805 (`lib.rs` 的 import 处), 可能需要单独抽出 |
| `kernel/src/exception.rs` | +59 | n/a (#244 已合) | ✅ | #244 | break/debug 异常已在 axhal 上游 |
| `kernel/src/file/mod.rs` | +19 | `os/StarryOS/kernel/src/file/mod.rs` | ❌ | PR-A | 添加 bpf / perf fd 类型, 与 tgoskits 现有 file enum 适配 |
| `kernel/src/mm/aspace/backend/cow.rs` | +10 | `os/StarryOS/kernel/src/mm/aspace/backend/cow.rs` | 🔧 | PR-A | kprobe 触发 COW 时的 hook, 需对照现有 backend 接口 |
| `kernel/src/mm/aspace/backend/file.rs` | +9 | 同 | 🔧 | PR-A | 同上 |
| `kernel/src/mm/aspace/backend/mod.rs` | +67/-? | 同 | 🔧 | PR-A | tgoskits 已新增 `linear.rs` `shared.rs` backend, 需重新对接 |
| `kernel/src/mm/aspace/mod.rs` | +10 | 同 | 🔧 | PR-A | — |
| `kernel/src/syscall/mm/mmap.rs` | +17/-? | `os/StarryOS/kernel/src/syscall/mm/mmap.rs` | 🔧 | PR-A | 与 #486 (brk semantics) 后的实现对接 |
| `kernel/src/syscall/fs/ctl.rs` | +33 | 已部分在 #673 | 🔄 | PR-A | #673 加了 32 行 (debugfs ioctl), eBPF 还需补 bpf fd 的 ioctl |
| `kernel/src/syscall/fs/fd_ops.rs` | +33 | 已部分在 #673 | 🔄 | PR-A | 同上 |
| `kernel/src/syscall/task/clone.rs` | +27 | `os/StarryOS/kernel/src/syscall/task/clone.rs` | 🔧 | PR-A | kretprobe stack 在 clone 时的复制; #805 已加 `kretprobe_stack` 字段, 但 clone 路径未对接 |
| `kernel/src/syscall/mod.rs` | +70/-? | 已部分在 #805 | 🔄 | PR-A/B | bpf/perf_event_open 已在 #805; init_module/finit_module/delete_module 由 PR-B 加 |
| `kernel/src/task/mod.rs` | +16 | 已部分在 #805 (`kretprobe_stack`) | ✅ | #805 | — |
| `kernel/src/task/user.rs` | +26 | `os/StarryOS/kernel/src/task/user.rs` | 🔧 | PR-A | kprobe 进入用户态前的 hook |
| `kernel/src/entry.rs` | +17 | 已部分在 #805 | 🔄 | PR-A/B | #805 加了 kallsyms / kprobe init; PR-A 加 perf/bpf init, PR-B 加 kmod init |
| `kernel/src/lib.rs` | +19 | 已部分在 #805 (`kallsyms`/`kprobe`/`ebpf` mod) + #673 (`tracepoint`) | 🔄 | PR-A/B | 还需加 `perf`、`uprobe`、`kmod` mod |

### I. pseudofs (与 #673 部分重叠)

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `kernel/src/pseudofs/debug.rs` | +138 | `os/StarryOS/kernel/src/pseudofs/debug.rs` | 🔄 | #446 + #673 | 已合 #446 dyn_debug + #673 加了 18 行 debugfs 框架 |
| `kernel/src/pseudofs/dir.rs` | +45 | `os/StarryOS/kernel/src/pseudofs/dir.rs` | 🔄 | #673 | dynamic dir 已加, eBPF 需要的 ProgFs 等仍需 PR-A 补 |
| `kernel/src/pseudofs/mod.rs` | +62 | `os/StarryOS/kernel/src/pseudofs/mod.rs` | 🔄 | #673 | — |
| `kernel/src/pseudofs/proc.rs` | +91/-? | 已部分在 #805 (`/proc/kallsyms`) + #306 (`/proc/pid/maps`) | 🔄 | PR-A | bpf prog 列表 `/proc/sys/kernel/bpf_stats_enabled` 等仍需补 |
| `kernel/src/pseudofs/sys.rs` | +70 | `os/StarryOS/kernel/src/pseudofs/sysfs.rs` | 🔧 | PR-A | tgoskits 已重命名 `sys.rs` → `sysfs.rs`, 路径调整 |

### J. starryos crate (内核可执行)

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `starryos/Cargo.toml` | +3 | `os/StarryOS/starryos/Cargo.toml` | 🔧 | PR-A/B | 按需加 feature flag |
| `starryos/build.rs` | +12 | 已在 #805 (+82 行) | ✅ | #805 | #805 用 `nm` 直接抽符号, 比源更简洁; **不引入 kallsym.ld** (见 K) |
| `starryos/src/main.rs` | +5/-? | 已在 #805 (+5 行 kallsyms 初始化) | ✅ | #805 | PR-A 不必再动 |

### K. 链接脚本

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `starryos/kallsym.ld` | +22 | n/a | ⛔ | — | #805 的 build.rs 走 `nm` 路线, 不预留 8M 段, 不需移植此文件 |
| `modules/kmod-linker.ld` | +47 | `os/StarryOS/modules/kmod-linker.ld` 或 `os/StarryOS/scripts/kmod-linker.ld` | ❌ | PR-B | LKM 需要的链接脚本, 不属于 starryos 主 binary, 由 xtask kmod 子命令引用 |

### L. modules/ (LKM 示例)

| 源文件 | 源行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `modules/hello/Cargo.toml` | +21 | `os/StarryOS/modules/hello/Cargo.toml` | ❌ | PR-C | — |
| `modules/hello/src/lib.rs` | +43 | `os/StarryOS/modules/hello/src/lib.rs` | ❌ | PR-C | — |
| `modules/kebpf/Cargo.toml` | +40 | `os/StarryOS/modules/kebpf/Cargo.toml` | ❌ | PR-C | — |
| `modules/kebpf/src/lib.rs` | +114 | `os/StarryOS/modules/kebpf/src/lib.rs` | ❌ | PR-C | — |
| `modules/kebpf/src/map.rs` | +28 | `os/StarryOS/modules/kebpf/src/map.rs` | ❌ | PR-C | — |
| `modules/kebpf/src/prog/mod.rs` | +30 | `os/StarryOS/modules/kebpf/src/prog/mod.rs` | ❌ | PR-C | — |
| `modules/kmod.mk` | +29 | n/a | ⛔ | — | 翻译进 xtask kmod 子命令 |

### M. user/musl/ (eBPF 用户程序, 7 个独立 crate)

| 源目录 | 大致行数 | tgoskits 对应 | 状态 | 归属 PR | 备注 |
|---|---|---|---|---|---|
| `user/musl/async_test/` | ~100 | TBD (`os/StarryOS/user/musl/` ?) | ❌ | PR-D | 最简单的 epoll 测试 |
| `user/musl/kret/` | ~1100 | TBD | ❌ | PR-D | kretprobe 综合测试, 三件套 (-common/-ebpf/-userspace) |
| `user/musl/mytrace/` | ~1100 | TBD | ❌ | PR-D | 同上, 自定义 tracepoint |
| `user/musl/rawtp/` | ~1100 | TBD | ❌ | PR-D | raw tracepoint |
| `user/musl/syscall_ebpf/` | ~1100 | TBD | ❌ | PR-D | syscall tracing |
| `user/musl/upb/` | ~1100 | TBD | ❌ | PR-D | uprobe |
| `user/musl/upb2/` | ~1100 | TBD | ❌ | PR-D | uprobe 进阶 |
| `user/musl/Makefile` | +42 | n/a | ⛔ | — | 翻译进 xtask |
| `user/musl/.cargo/config.toml` | +13 | `os/StarryOS/user/musl/.cargo/config.toml` 或 workspace 级 | ❌ | PR-D | cross 编译 target 配置 |

> **注**: Task #8 (PR-D) 开工前要先决定 `user/musl/` 的 canonical 位置
> (见 workflow §5.4)。本表暂用 `TBD` 占位。

---

## 剩余移植量估算 (净新增, 排除 ⛔ / ✅ / 大部分 🔄)

| 归属 PR | 净新增文件 | 净新增行数 (粗估) |
|---|---|---|
| PR-A (kbpf-basic + perf + uprobe + ebpf module 化) | ~16 | ~1900 |
| PR-B (kmod 内核侧 + syscall + 链接脚本 + xtask) | ~7 + xtask | ~1750 |
| PR-C (modules/hello + modules/kebpf) | 6 | ~280 |
| PR-D (user/musl 7 个 crate + 跨编 toolchain 配置) | ~50+ | ~6000+ (含 Cargo.lock) |

(总计 ≈ 9900 行, 其中 ~70% 是 PR-D 的 Cargo.lock; 实际审查代码 ≈ 3000 行。)

## 关键决策点

1. **`kernel/src/bpf/tansform.rs` 改名**: 移植时改为 `transform.rs`,
   commit message 显式说明拼写修正。
2. **`kernel/src/kprobe/arch/*.rs` 不移植**: #805 使用 `kprobe = "0.5"`
   crate, 架构差异由 crate 内部处理。源中的 arch 文件是早期未抽出 crate
   时的产物。
3. **`starryos/kallsym.ld` 不移植**: #805 改用 build.rs `nm` 抽符号方式,
   不预留 8M 段, 设计上优于源仓做法。
4. **`kernel/src/perf/kprobe.rs` 与 #805 `kprobe.rs` 关系**:
   #805 的 `kprobe.rs` 是底层 (KernelKprobeOps + 全局 KPROBE_MANAGER),
   源中的 `perf/kprobe.rs` 是上层 (perf event 的 kprobe 绑定),
   PR-A 直接基于 #805 的底层调用即可。
5. **`mm/aspace/backend/` 改动**: 源仓基于旧 backend 接口, tgoskits
   已经引入 `linear.rs` `shared.rs` 新 backend, PR-A 需要按新接口
   重写 kprobe 的 COW hook。
6. **`pseudofs/sys.rs` → `pseudofs/sysfs.rs`**: tgoskits 已重命名,
   迁移路径要跟进。

## 风险与未决

- **PR-A 接 #673 的 raw tracepoint 接口**: 当前 #673 没有 `event.rs`
  对应物, PR-A 在添加 `perf/raw_tracepoint.rs` 时要么 (a) 在 PR-A
  内顺手补 `tracepoint/event.rs` 并通知 #673 author, 要么 (b) 把
  raw tracepoint 部分挪到 #673 合入后再做。建议 (a) 更高效。
- **`Cargo.lock` 体积**: PR-D 单个 user/musl crate 的 lock 文件就近 800
  行, 7 个程序 ≈ 5500 行 lock。审查时 reviewer 可能要求合并 workspace
  lock。Task #8 决定 user 位置时一起评估。
- **loongarch64 LTO**: 源 commit `3fed46e` 关闭了 loongarch64 LTO,
  PR-A 的 Cargo.toml 改动里要确认这个仍是 tgoskits 需要的。
