# eBPF kmod-level dispatch: the syscall registration interface

> 承接 PR #880 review 中 @Godones 的建议: 让 kmod 形态的 eBPF "走系统调用注册
> 接口", 而不是编译期弱/强符号决议. 未注册时 `bpf(2)` 落到 #850 的内核内置
> `sys_bpf` (syscall-level eBPF).

## 1. 设计

内核暴露一个**运行时**的 syscall handler 注册表 (`kernel/src/syscall/registry.rs`):

```rust
pub type SyscallHandler = fn(args: [usize; 6]) -> AxResult<isize>;

pub fn register_syscall_handler(sysno: Sysno, handler: SyscallHandler) -> Option<SyscallHandler>;
pub fn unregister_syscall_handler(sysno: Sysno) -> Option<SyscallHandler>;
pub fn lookup_syscall_handler(sysno: Sysno) -> Option<SyscallHandler>;
```

`handle_syscall` 的 `Sysno::bpf` 臂先查注册表:

```rust
Sysno::bpf => match registry::lookup_syscall_handler(Sysno::bpf) {
    Some(handler) => handler([arg0, arg1, arg2, arg3, arg4, arg5]),
    None          => crate::ebpf::sys_bpf(arg0, arg1, arg2),   // 内核内置, 默认
},
```

- **未注册** → 走 `crate::ebpf::sys_bpf` (#850 syscall-level eBPF), 行为与未引入
  任何模块完全一致.
- **已注册** → 由模块提供的 handler 服务 `bpf(2)`.

`register` / `unregister` 是 push 进 / 拿出一个 `BTreeMap<Sysno, _>` 表项, 因此
**对"内置"和"可加载"两种形态是统一的**: 同一个 `kebpf` 模块, 无论被链入内核还是
作为 `.ko` 加载, 都在 `init` 里 `register_syscall_handler(Sysno::bpf, ...)`、在
`exit` 里 `unregister_syscall_handler(Sysno::bpf)`.

### 符号导出

`register_syscall_handler` / `unregister_syscall_handler` 只被模块调用、内核自身从不
调用 (内核只调 `lookup_syscall_handler`). StarryOS 内核链接虽不开 `--gc-sections`,
但 rustc/LLVM 仍会把这两个未被引用的 `pub fn` 从最终二进制里 DCE 掉, 它们就进不了
`.kallsyms`, 可加载 `.ko` 也就无从绑定 → `ENOEXEC`. 所以 `registry.rs` 里用一个
`#[used]` 的函数指针静态量把两个符号"钉"在内核镜像中 (`MODULE_SYSCALL_REGISTRATION_ABI`).
`crate::lib` 再把三个函数 re-export 到 crate root, 模块按 `starry_kernel::register_syscall_handler`
绑定.

## 2. kebpf 模块

`modules/kebpf/src/lib.rs`:

```rust
fn bpf_syscall_handler(args: [usize; 6]) -> AxResult<isize> {
    sys_bpf(args[0] as u32, args[1] as *mut u8, args[2] as u32)
}

#[init_fn]
pub fn kebpf_init() -> i32 {
    starry_kernel::register_syscall_handler(Sysno::bpf, bpf_syscall_handler);
    0
}

#[exit_fn]
fn kebpf_exit() {
    starry_kernel::unregister_syscall_handler(Sysno::bpf);
}
```

## 3. 两种形态 / 编译

### 3.1 内置 (built-in, 推荐, 已端到端验证)

`starryos` 新增 `kebpf` 编译 feature (默认关). 开启时把 `kebpf` 链入内核镜像, 并在
`main()` 启动 `init` 进程前调用 `kebpf::kebpf_init()` 完成注册:

```bash
# 在板级 config 的 features 里追加 "kebpf", 例如:
#   os/StarryOS/configs/board/qemu-x86_64.toml -> features += "kebpf"
cargo xtask starry build --arch x86_64 --config <带 kebpf 的 config>
```

内置形态下 `kebpf` 与内核**共享同一份 `starry_kernel` / `kbpf_basic` 编译产物**
(同一次 `cargo build`), 所以模块 handler 与内核 eBPF 运行时的类型/符号完全一致,
`bpf(2)` 由模块真正服务且可与内核 eBPF 状态互操作.

### 3.2 可加载 (loadable `.ko`, 已端到端验证)

`.ko` 的重定位在加载时由 `kmod_loader` 按**精确 mangled 名**对内核 `.kallsyms`
决议, 所以 `.ko` 里 `starry_kernel`/`core`/`alloc`/`kbpf_basic` 等符号的
StableCrateId (`CsXXXX_` hash) 必须与运行内核**逐位一致**, 且内核镜像里必须**真的
保留**这些符号. 关键卡点是 workspace `lto = true` 会把它们 inline/DCE 掉 —— 内核
`.kallsyms` 里根本不存在, `.ko` 无从绑定. (codegen rustflags 如 `static`/`large`
**不**影响 hash, 只有 LTO 这个 profile 设置和 feature 集合影响.)

为此引入 `STARRY_KMOD=y` 构建模式 (见 `scripts/axbuild/src/build.rs` 的
`kmod_build_mode` / `apply_kmod_build_mode` / `toolchain_rustflags`):

```bash
# 1) 以 kmod 模式构建内核: lto=false 保留全部符号; static/large 产生 loader 能
#    处理的 R_X86_64_64 重定位 (kmod_loader 实现了 R_X86_64_64/PC32/… 但不含
#    GOTPCREL), 且可达 0xffff_8000… 的模块加载地址.
STARRY_KMOD=y cargo xtask starry build --arch x86_64
# 2) 构建 .ko —— 与内核**同一份** cargo 解析 (`-p starryos -p <module>`,
#    `--features ax-hal/x86-pc,starryos/qemu`, 同样 static/large + lto=false,
#    共享 target/), 故 cargo 把 starry_kernel 等统一为同一份产物, hash 天然对齐:
cargo xtask starry kmod build --all --arch x86_64
# 3) guest 内经 finit_module(2) 加载 (见 kmod-modules 测试).
```

要点:
- `qemu` 必须限定为 `starryos/`: 裸 `qemu` 会同时点亮 `kebpf` 自己的同名 feature,
  扰动共享 feature 闭包导致 hash 偏移. 平台 feature 必须是 `ax-hal/x86-pc`
  (即 `cargo xtask starry build` 解析到的), 而非 `ax-feat/defplat`.
- raw cargo 链接 `starryos` 二进制会失败 (`axplat.x` 链接脚本仅由 ostool 路径生成),
  这是预期的: `kmod.rs` 在 `libkebpf.rlib` 已产出的前提下容忍该非零退出, 然后只对
  模块自身 rlib 做 `ld -r --whole-archive` (不 bundle 任何依赖 —— 全部由内核
  `.kallsyms` 决议).
- 内核侧为可加载路径补了几处符号导出: `EbpfKernelAuxiliary` 叶子方法 /
  `BpfMap::unified_map` / `BpfProg::drop` 标 `#[inline(never)]`; `handle_prog_load`
  比照模块 `bpf_prog_load` 也走 `debug!("{meta:#?}")` + `BpfProgVerifierInfo::from`,
  使 `BpfProgMeta: Debug` 与该 `From` 实例进入 `.kallsyms`.

结果: `.ko` 未决议符号 = **0**, 全部重定位为 `R_X86_64_64`, hash 对齐. 加载时
`kebpf_init` 执行并 `register_syscall_handler`, 从此 `bpf(2)` 由模块服务 —— 即
@Godones 期望的"可加载 `.ko` 接管 `bpf(2)`".

> 注: `STARRY_KMOD` 模式的 `lto=false` + 保留全符号会增大内核体积、禁用部分优化,
> 故作为**独立构建开关**, 不影响生产内核 (默认 `lto=true`).

### 3.3 CI 接线 (kmod-modules 测试自启 kmod 模式)

`STARRY_KMOD` 既可由进程环境变量开启 (上面的手跑步骤), 也可由**单个测试 build
wrapper** 的 `[env]` 开启 —— 见 `build::kmod_build_mode_with_env`. 主 CI 跑的是裸
`cargo xtask starry test qemu --arch x86_64` (不导出 `STARRY_KMOD`), 若整条命令导出
该变量会把**所有**用例的内核都重建成 large/no-LTO. 因此把 `kmod-modules` 用例单独
放进自己的 build wrapper `test-suit/starryos/normal/qemu-kmod/`, 其
`build-x86_64-unknown-none.toml` 在 `[env]` 里设 `STARRY_KMOD = "y"`: 只有这一个用例
的内核以 kmod 模式构建 (保留 `.kallsyms` 符号), 其余 normal 用例继续用默认 LTO 生产
内核. 该 wrapper 的 feature 集与 `configs/board/qemu-x86_64.toml` 完全一致, 故
`starry_kernel`/`core`/`alloc`/`kbpf_basic` 的 hash 与 `.ko` 共建产物逐位对齐.

## 4. 验证 (x86_64)

- `cargo fmt --all -- --check` ✅
- `cargo xtask starry build --arch x86_64` (feature 关) ✅ —— `bpf(2)` 走内核内置.
- `cargo xtask starry build --arch x86_64 --config <kebpf>` (feature 开) ✅ ——
  内置形态, ELF 中模块与内核共享同一 `starry_kernel` 实例.
- **可加载形态端到端 (kmod-modules QEMU 测试) ✅**:
  `cargo xtask starry test qemu --arch x86_64 -c kmod-modules` (无需手动导出
  `STARRY_KMOD` —— 该用例的 build wrapper 已在 `[env]` 里自启 kmod 模式, 见 §3.3)
  → 内核启动 (验证 static/large 内核可启动) → `finit_module` 加载 hello.ko /
  kebpf.ko → 串口可见 `LOADED /lib/modules/kebpf.ko`、`Hello, eBPF Kernel Module!`、
  `KMOD_SMOKE_DONE rc=0`, `PASS kmod-modules` (无 panic).
