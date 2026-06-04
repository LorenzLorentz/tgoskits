//! Dynamic syscall-handler registration interface.
//!
//! A loadable (or built-in) kernel module can take over an individual syscall
//! at runtime by registering a handler here. The static dispatch in
//! [`handle_syscall`](super::handle_syscall) consults this registry for the
//! syscalls that opt into dynamic override — currently only `bpf(2)` — and
//! falls back to the kernel's built-in implementation when no handler is
//! registered.
//!
//! This mirrors the upstream `Starry-OS/StarryOS:ebpf-kmod` `SyscallHandler`
//! registry that `kebpf` used to install itself as the `bpf(2)` dispatcher.
//! Because registration is a runtime operation keyed on the symbol resolved
//! from `.kallsyms`, the very same `kebpf` module works identically whether it
//! is linked into the kernel as a built-in or loaded later as a `.ko`: in both
//! cases its `init` calls [`register_syscall_handler`] and its `exit` calls
//! [`unregister_syscall_handler`].

use alloc::collections::btree_map::BTreeMap;

use ax_errno::AxResult;
use ax_kspin::SpinNoPreempt;
use syscalls::Sysno;

/// A dynamically-registered syscall handler.
///
/// It receives the raw syscall argument registers (`arg0..arg5`) and returns
/// the same `AxResult<isize>` the built-in handlers produce, so a registered
/// handler is drop-in interchangeable with the static dispatch arm it
/// overrides.
pub type SyscallHandler = fn(args: [usize; 6]) -> AxResult<isize>;

/// Registry of module-provided syscall handlers, keyed by syscall number.
static SYSCALL_HANDLERS: SpinNoPreempt<BTreeMap<Sysno, SyscallHandler>> =
    SpinNoPreempt::new(BTreeMap::new());

/// Register `handler` as the implementation of `sysno`, replacing any
/// previously-registered handler. Returns the handler that was displaced, if
/// any, so a module can chain to or later restore its predecessor.
///
/// Exported for kernel modules: it must remain resolvable in `.kallsyms` so a
/// `.ko` can bind to it at load time, hence it is never inlined away by the
/// kernel itself.
pub fn register_syscall_handler(sysno: Sysno, handler: SyscallHandler) -> Option<SyscallHandler> {
    SYSCALL_HANDLERS.lock().insert(sysno, handler)
}

/// Remove the handler registered for `sysno`, returning it if one was present.
/// Afterwards the kernel's built-in implementation services `sysno` again.
pub fn unregister_syscall_handler(sysno: Sysno) -> Option<SyscallHandler> {
    SYSCALL_HANDLERS.lock().remove(&sysno)
}

/// Look up the handler registered for `sysno`, if any. Returns a copy of the
/// function pointer so the registry lock is released before the handler runs.
pub fn lookup_syscall_handler(sysno: Sysno) -> Option<SyscallHandler> {
    SYSCALL_HANDLERS.lock().get(&sysno).copied()
}

/// `fn` type of [`register_syscall_handler`] (factored out for the `#[used]`
/// anchor below; keeps the static's type simple).
type RegisterFn = fn(Sysno, SyscallHandler) -> Option<SyscallHandler>;
/// `fn` type of [`unregister_syscall_handler`].
type UnregisterFn = fn(Sysno) -> Option<SyscallHandler>;

/// Anchor that force-retains the module-facing registration symbols in the
/// kernel image.
///
/// `register_syscall_handler` / `unregister_syscall_handler` are called only
/// by modules (the kernel itself only ever calls `lookup_syscall_handler`), so
/// without an explicit reference the compiler drops them from the final binary
/// — they then never reach `.kallsyms` and a `.ko` fails to bind to them at
/// load time (`ENOEXEC`). Taking their addresses in a `#[used]` static keeps
/// both functions (and their mangled symbols) in the linked kernel.
#[used]
static MODULE_SYSCALL_REGISTRATION_ABI: (RegisterFn, UnregisterFn) =
    (register_syscall_handler, unregister_syscall_handler);
