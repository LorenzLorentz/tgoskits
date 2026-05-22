//! Kernel-side eBPF demo loadable kernel module. Ported from
//! `Starry-OS/StarryOS:ebpf-kmod` (`modules/kebpf/src/lib.rs`).
//!
//! Scope differences from the source:
//!
//! * **No `register_syscall_handler` call.** The source kebpf module
//!   installed itself as the `bpf(2)` dispatcher via a dynamic
//!   `SyscallHandler` registry. tgoskits dispatches `Sysno::bpf`
//!   statically to the kernel-resident `crate::ebpf::sys_bpf` in
//!   `os/StarryOS/kernel/src/syscall/mod.rs` (PR-A), so the LKM has no
//!   reason — and no API — to intercept the syscall. The module
//!   instead retains `sys_bpf` / `bpf` as `pub` demonstrations that an
//!   LKM can drive `kbpf-basic` directly through tgoskits's public
//!   `starry_kernel::ebpf::*` surface. `init_fn` / `exit_fn` reduce to
//!   load-time / unload-time log lines.
//! * `starry_kernel::bpf::tansform` → `starry_kernel::ebpf::transform`
//!   (path + typo fix landed in PR-A).
//! * `axerrno` → `ax_errno`, `axlog` → `ax_log`, `axio` → `ax_io`
//!   (workspace-package renames per `crate-fork-audit.md §6`).
//! * `syscalls::Sysno` dependency dropped together with the registry call.

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
use kmod::{exit_fn, init_fn, module};
use starry_kernel::{ebpf::transform::EbpfKernelAuxiliary, mm::VmBytes};

mod map;
mod prog;

/// Handle the bpf syscall from a userland-supplied `bpf_attr` pointer
/// living in the calling task's address space. Retained from the source
/// module as a demonstration entry point; not wired into tgoskits's
/// static syscall dispatch (see module-level doc).
pub fn sys_bpf(cmd: u32, attr: *mut u8, size: u32) -> AxResult<isize> {
    let mut buf = vec![0u8; size as usize];
    let _l = VmBytes::new(attr, size as _).read(&mut buf)?;

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
            kbpf_basic::map::bpf_map_update_elem::<EbpfKernelAuxiliary>(update_arg)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_LOOKUP_ELEM => {
            kbpf_basic::map::bpf_lookup_elem::<EbpfKernelAuxiliary>(update_arg)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_GET_NEXT_KEY => {
            let update_arg = BpfMapGetNextKeyArg::from(attr);
            kbpf_basic::map::bpf_map_get_next_key::<EbpfKernelAuxiliary>(update_arg)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_DELETE_ELEM => {
            kbpf_basic::map::bpf_map_delete_elem::<EbpfKernelAuxiliary>(update_arg)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_LOOKUP_AND_DELETE_ELEM => {
            kbpf_basic::map::bpf_map_lookup_and_delete_elem::<EbpfKernelAuxiliary>(update_arg)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_LOOKUP_BATCH => {
            kbpf_basic::map::bpf_map_lookup_batch::<EbpfKernelAuxiliary>(update_arg)?;
            Ok(0)
        }
        bpf_cmd::BPF_MAP_FREEZE => {
            kbpf_basic::map::bpf_map_freeze::<EbpfKernelAuxiliary>(update_arg.map_fd)?;
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
        // Object creation commands
        bpf_cmd::BPF_BTF_LOAD | bpf_cmd::BPF_LINK_CREATE | bpf_cmd::BPF_OBJ_GET_INFO_BY_FD => {
            ax_log::warn!("bpf cmd: [{:?}] not implemented", cmd);
            Err(AxError::Unsupported)
        }
        ty => {
            unimplemented!("bpf cmd: [{:?}] not implemented", ty)
        }
    }
}

#[init_fn]
pub fn kebpf_init() -> i32 {
    ax_log::ax_println!("Hello, eBPF Kernel Module!");
    0
}

#[exit_fn]
fn kebpf_exit() {
    ax_log::ax_println!("Goodbye, eBPF Kernel Module!");
}

module!(
    name: "kebpf",
    license: "GPL",
    description: "kernel eBPF module",
    version: "0.1.0",
);
