//! BPF program load helper used by the kebpf demo module. Ported from
//! `Starry-OS/StarryOS:ebpf-kmod` (`modules/kebpf/src/prog/mod.rs`).
//!
//! Adaptations for tgoskits:
//! * `axerrno` → `ax_errno`, `axlog` → `ax_log` (workspace-package renames).
//! * `starry_kernel::bpf::{tansform,prog}` → `starry_kernel::ebpf::{transform,prog}`
//!   (path + typo fix landed in PR-A).

use alloc::sync::Arc;

use ax_errno::AxResult;
use kbpf_basic::{
    linux_bpf::bpf_attr,
    preprocessor::EbpfPreProcessor,
    prog::{BpfProgMeta, BpfProgVerifierInfo},
};
use starry_kernel::{
    ebpf::{prog::BpfProg, transform::EbpfKernelAuxiliary},
    file::add_file_like,
};

/// Load a BPF program into the kernel.
///
/// See https://ebpf-docs.dylanreimerink.nl/linux/syscall/BPF_PROG_LOAD/
pub fn bpf_prog_load(attr: &bpf_attr) -> AxResult<isize> {
    let mut args = BpfProgMeta::try_from_bpf_attr::<EbpfKernelAuxiliary>(attr)?;
    ax_log::warn!("bpf_prog_load: {:#?}", args);
    let _log_info = BpfProgVerifierInfo::from(attr);
    let prog_insn = args.take_insns().unwrap();
    let preprocessor =
        EbpfPreProcessor::preprocess::<EbpfKernelAuxiliary>(prog_insn).expect("preprocess failed");
    let prog = Arc::new(BpfProg::new(args, preprocessor));

    let fd = add_file_like(prog, false)?;

    ax_log::warn!("bpf_prog_load: fd: {}", fd);
    Ok(fd as _)
}
