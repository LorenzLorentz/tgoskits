# StarryOS 用户态 eBPF 程序

从 `Starry-OS/StarryOS:ebpf-kmod` 的 `user/musl/` 迁移而来 (PR-D, see
`docs/WORKFLOW_EBPF_LKM_MIGRATION.md`)。配合 `os/StarryOS/kernel/src/ebpf/`
与 `os/StarryOS/kernel/src/perf/` (PR-A) 的内核侧实现使用。

## 子目录

| 程序 | 测试目标 | 内核侧依赖 |
|---|---|---|
| `kret/` | kretprobe 综合 (`sys_getpid` 返回值) | `kernel/src/perf/kprobe.rs` |
| `rawtp/` | raw tracepoint (`sys_clone`) | `kernel/src/perf/raw_tracepoint.rs` |
| `mytrace/` | tracepoint (`syscalls:sys_enter_openat`) | `kernel/src/perf/tracepoint.rs` |
| `syscall_ebpf/` | syscall 计数 (kprobe + HashMap) | `kernel/src/perf/kprobe.rs` + `ebpf/map.rs` |
| `upb/` | uprobe (用户函数) | `kernel/src/perf/uprobe.rs` (Phase 3 未接通,见 §限制) |
| `upb2/` | uprobe + uretprobe (musl libc) | 同上 |
| `async_test/` | tokio breakpoint 测试 (不含 eBPF) | 仅依赖 `core::arch::breakpoint` |

每个 eBPF 程序自身是一个独立的 Cargo workspace, 内部分三个 sub-crate:

- `<prog>/<prog>/` — 用户态 loader (std, 链 aya)
- `<prog>/<prog>-common/` — userspace ↔ eBPF 共享类型 (`#![no_std]`)
- `<prog>/<prog>-ebpf/` — eBPF 字节码 (`#![no_std] #![no_main]`, 通过 aya-build
  在 `<prog>/<prog>/build.rs` 内串行编译)

`async_test/` 是单一 crate, 用于 `core::arch::breakpoint` 与 tokio 多线程
异步路径的 smoke 测试, 与 eBPF 无关。

## 与 tgoskits workspace 的关系

这 7 个 crate 在仓根 `Cargo.toml` 的 `[workspace] exclude` 列表中, 不被主
workspace 解析。原因:

1. 每个程序都是自带 `[workspace]` 的子 workspace, 嵌入主 workspace 会触发
   `nested workspace not allowed`。
2. 依赖 `aya-rs/aya` 的 git 版本 (上游未发到 crates.io), 与主 workspace
   `resolver = "3"` + nightly toolchain 闭包独立。
3. 构建需要 `bpf-linker` 与 `nightly --component rust-src`, 与主 workspace
   `cargo xtask starry build` 的目标 (no_std kernel binary) 不重叠。

## 构建

通过 xtask 子命令统一驱动 (替代源仓的 `user/musl/Makefile`):

```bash
cargo xtask starry user-ebpf build --program kret --arch x86_64
cargo xtask starry user-ebpf build --all --arch x86_64
```

详见 `scripts/axbuild/src/starry/user_ebpf.rs` 的实现与 `--help` 输出。

前置环境 (参考各子 `README.md`):

1. `rustup toolchain install nightly --component rust-src`
2. `rustup target add <arch>-unknown-linux-musl` 三/四架构按需
3. C 工具链: 例如 `brew install filosottile/musl-cross/musl-cross`
   (macOS) 或对应 Linux apk/apt 安装 `*-linux-musl-cc`
4. `cargo install bpf-linker`

## qemu 内运行

构建产物落到 `target/<musl-target>/release/<prog>`。把它 install 到 rootfs
的 `/usr/bin/`, 在 qemu starryos 内:

```sh
# kret: 跟踪 sys_getpid 返回值
kret __aarch64_sys_getpid

# rawtp: sys_clone raw tp
rawtp

# mytrace: sys_enter_openat 文件名抓取
mytrace
```

`trace_pipe` (`/sys/kernel/tracing/trace_pipe`) 应输出对应 `info!` 日志。

## 限制

- `upb` / `upb2` 依赖 `kernel/src/perf/uprobe.rs` 真正接通 `ProcessData`
  的 `uprobe_manager` / `uprobe_point_list` 与 `AddrSpace::memoryset`
  accessor。PR-A 落地范围仅占位返回 `Unsupported`, 这两个程序待后续
  PR 解锁。
- `loongarch64-unknown-linux-musl` 的工具链在大多数发行版没有现成包,
  目前仅在 OrangePi 测试链上可用; CI 默认只跑 `x86_64` / `aarch64` /
  `riscv64gc` 三个 musl target。
