//! BPF map creation helper used by the kebpf demo module. Ported from
//! `Starry-OS/StarryOS:ebpf-kmod` (`modules/kebpf/src/map.rs`).
//!
//! Adaptations for tgoskits:
//! * `axerrno` → `ax_errno`, `axlog` → `ax_log` (workspace-package renames).
//! * `starry_kernel::bpf::tansform` → `starry_kernel::ebpf::transform`
//!   (path + typo fix landed in PR-A).
//! * `starry_kernel::bpf::map` → `starry_kernel::ebpf::map`.
//! * `bpf_map_create` takes `Arc<dyn PollWaker>`; we explicitly upcast the
//!   `Arc<PollSetWrapper>` to match the same pattern used by PR-A's
//!   `starry_kernel::ebpf::map::create_map`.

use alloc::sync::Arc;

use ax_errno::AxResult;
use kbpf_basic::{PollWaker, linux_bpf::bpf_attr, map::BpfMapMeta};
use starry_kernel::{
    ebpf::{
        map::{BpfMap, PollSetWrapper},
        transform::{EbpfKernelAuxiliary, PerCpuImpl},
    },
    file::add_file_like,
};

use super::bpf_err;

pub fn bpf_map_create(attr: &bpf_attr) -> AxResult<isize> {
    let map_meta = BpfMapMeta::try_from(attr).map_err(bpf_err)?;
    ax_log::debug!("The map attr is {:#?}", map_meta);

    let poll_ready = Arc::new(PollSetWrapper::new());
    let poll_ready_dyn: Arc<dyn PollWaker> = poll_ready.clone();

    let unified_map = kbpf_basic::map::bpf_map_create::<EbpfKernelAuxiliary, PerCpuImpl>(
        map_meta,
        Some(poll_ready_dyn),
    )
    .map_err(bpf_err)?;

    let file = Arc::new(BpfMap::new(unified_map, poll_ready));
    let fd = add_file_like(file, false).map(|fd| fd as _);
    ax_log::info!("bpf_map_create: fd: {:?}", fd);
    fd
}
