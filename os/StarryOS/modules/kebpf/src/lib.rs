//! Kernel-side eBPF demo loadable kernel module. Ported from
//! `Starry-OS/StarryOS:ebpf-kmod` (`modules/kebpf/src/lib.rs`).
//!
//! Scope differences from the source:
//!
//! * **Registers `bpf(2)` through the syscall registration interface.**
//!   Like the source, this module installs itself as the `bpf(2)`
//!   dispatcher: `init` calls
//!   [`starry_kernel::register_syscall_handler`]`(Sysno::bpf, ...)` and
//!   `exit` calls [`starry_kernel::unregister_syscall_handler`]. While a
//!   handler is registered, the kernel's `Sysno::bpf` arm dispatches to
//!   this module; once it is removed (or never installed) the kernel falls
//!   back to its built-in `crate::ebpf::sys_bpf`. The same mechanism works
//!   whether `kebpf` is linked in as a built-in or loaded as a `.ko`.
//! * `starry_kernel::bpf::tansform` → `starry_kernel::ebpf::transform`
//!   (path + typo fix landed in PR-A).
//! * `axerrno` → `ax_errno`, `axlog` → `ax_log`, `axio` → `ax_io`
//!   (workspace-package renames per `crate-fork-audit.md §6`).

#![no_std]
extern crate alloc;

use alloc::vec;

use ax_errno::{AxError, AxResult};
use ax_io::Read;
use kbpf_basic::{
    linux_bpf::{bpf_attr, bpf_cmd},
    map::{BpfMapGetNextKeyArg, BpfMapUpdateArg},
    raw_tracepoint::BpfRawTracePointArg,
};
use kmod_tools::{exit_fn, init_fn, module};
use starry_kernel::{ebpf::transform::EbpfKernelAuxiliary, mm::VmBytes};
use syscalls::Sysno;

/// Convert `kbpf_basic::BpfError` (`axerrno::LinuxError`) to `ax_errno::AxError`.
///
/// `kbpf-basic` depends on `axerrno` (crates.io v0.2.x) while this
/// workspace uses `ax-errno` (local v0.5.x). The two `LinuxError` types
/// are distinct, so we convert through the shared `i32` error code.
fn bpf_err(e: kbpf_basic::BpfError) -> AxError {
    use ax_errno::LinuxError;
    let linux_err = LinuxError::try_from(e.code()).unwrap_or(LinuxError::EIO);
    linux_err.into()
}

mod map;
mod prog;

/// Adapter matching [`starry_kernel::SyscallHandler`]: unpacks the raw
/// `bpf(2)` argument registers (`cmd`, `attr`, `size`) and forwards them to
/// this module's [`sys_bpf`]. Installed as the `Sysno::bpf` handler by
/// [`kebpf_init`].
fn bpf_syscall_handler(args: [usize; 6]) -> AxResult<isize> {
    sys_bpf(args[0] as u32, args[1] as *mut u8, args[2] as u32)
}

/// Handle the bpf syscall from a userland-supplied `bpf_attr` pointer
/// living in the calling task's address space. While this module is loaded
/// it services every `bpf(2)` call via [`bpf_syscall_handler`]; see the
/// module-level doc.
pub fn sys_bpf(cmd: u32, attr: *mut u8, size: u32) -> AxResult<isize> {
    // Match the kernel's read_bpf_attr pattern: allocate a zero-initialised
    // buffer sized to the kernel's bpf_attr, copy only min(size, sizeof),
    // and interpret the result. This avoids OOB read when the user buffer
    // is shorter than bpf_attr, and prevents unbounded allocation when the
    // user passes an excessively large size.
    let mut buf = vec![0u8; core::mem::size_of::<bpf_attr>()];
    let copy_len = (size as usize).min(buf.len());
    let mut reader = VmBytes::new(attr, copy_len as _);
    reader.read(&mut buf[..copy_len])?;
    // SAFETY: bpf_attr is a transparent C union with all-bytes layout; the
    // user-supplied bytes are copied into the front of a zero-initialised
    // buffer whose size matches sizeof(bpf_attr).
    let attr = unsafe { &*(buf.as_ptr() as *const bpf_attr) };
    let cmd = bpf_cmd::try_from(cmd).map_err(|_| AxError::InvalidInput)?;
    bpf(cmd, attr)
}

/// Dispatch a single bpf(2) command on a kernel-side `bpf_attr`. Mirrors
/// the source's command table; calls into `kbpf-basic` directly for map
/// operations and delegates program load / raw-tracepoint open to the
/// per-cmd helpers.
pub fn bpf(cmd: bpf_cmd, attr: &bpf_attr) -> AxResult<isize> {
    let update_arg = BpfMapUpdateArg::from(attr);
    match cmd {
        // Map related commands
        bpf_cmd::BPF_MAP_CREATE => map::bpf_map_create(attr),
        bpf_cmd::BPF_MAP_UPDATE_ELEM => {
            kbpf_basic::map::bpf_map_update_elem::<EbpfKernelAuxiliary>(update_arg)
                .map_err(bpf_err)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_LOOKUP_ELEM => {
            kbpf_basic::map::bpf_lookup_elem::<EbpfKernelAuxiliary>(update_arg).map_err(bpf_err)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_GET_NEXT_KEY => {
            let update_arg = BpfMapGetNextKeyArg::from(attr);
            kbpf_basic::map::bpf_map_get_next_key::<EbpfKernelAuxiliary>(update_arg)
                .map_err(bpf_err)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_DELETE_ELEM => {
            kbpf_basic::map::bpf_map_delete_elem::<EbpfKernelAuxiliary>(update_arg)
                .map_err(bpf_err)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_LOOKUP_AND_DELETE_ELEM => {
            kbpf_basic::map::bpf_map_lookup_and_delete_elem::<EbpfKernelAuxiliary>(update_arg)
                .map_err(bpf_err)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_LOOKUP_BATCH => {
            kbpf_basic::map::bpf_map_lookup_batch::<EbpfKernelAuxiliary>(update_arg)
                .map_err(bpf_err)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_FREEZE => {
            kbpf_basic::map::bpf_map_freeze::<EbpfKernelAuxiliary>(update_arg.map_fd)
                .map_err(bpf_err)?;
            Ok(0)
        }
        // Attaches the program to the given tracepoint.
        bpf_cmd::BPF_RAW_TRACEPOINT_OPEN => {
            let arg = BpfRawTracePointArg::try_from_bpf_attr::<EbpfKernelAuxiliary>(attr)
                .map_err(|_| AxError::InvalidInput)?;
            starry_kernel::perf::raw_tracepoint::bpf_raw_tracepoint_open(arg)
        }
        // Program related commands
        bpf_cmd::BPF_PROG_LOAD => prog::bpf_prog_load(attr),
        // Object creation commands. Linux's `bpf(2)` (and the kernel's built-in
        // `sys_bpf` from #850) returns `-EINVAL` for an unrecognized/unsupported
        // command — userspace feature-probing relies on that errno — so return
        // `InvalidInput` (EINVAL), not `Unsupported` (which maps to ENOSYS).
        bpf_cmd::BPF_BTF_LOAD | bpf_cmd::BPF_LINK_CREATE | bpf_cmd::BPF_OBJ_GET_INFO_BY_FD => {
            ax_log::warn!("bpf cmd: [{:?}] not implemented", cmd);
            Err(AxError::InvalidInput)
        }
        ty => {
            ax_log::warn!("bpf cmd: [{:?}] not implemented", ty);
            Err(AxError::InvalidInput)
        }
    }
}

#[init_fn]
pub fn kebpf_init() -> i32 {
    // Use `ax_log::print_fmt` (a real, retained kernel symbol) rather than the
    // `ax_println!` macro: the macro expands to `ax_log::__print_impl`, which the
    // kernel inlines into its own callsites and therefore does not keep as a
    // standalone symbol in `.kallsyms` — so a module relocation against it cannot
    // be resolved at load time. `print_fmt` is the function the kernel actually
    // retains.
    let _ = ax_log::print_fmt(format_args!("Hello, eBPF Kernel Module!\n"));
    // Take over `bpf(2)` via the kernel's syscall registration interface. From
    // now until `kebpf_exit`, the kernel dispatches `Sysno::bpf` to this module
    // instead of its built-in `crate::ebpf::sys_bpf`.
    starry_kernel::register_syscall_handler(Sysno::bpf, bpf_syscall_handler);
    0
}

#[exit_fn]
fn kebpf_exit() {
    // Relinquish `bpf(2)`; the kernel reverts to its built-in implementation.
    starry_kernel::unregister_syscall_handler(Sysno::bpf);
    let _ = ax_log::print_fmt(format_args!("Goodbye, eBPF Kernel Module!\n"));
}

module!(
    name: "kebpf",
    license: "GPL",
    description: "kernel eBPF module",
    version: "0.1.0",
);
