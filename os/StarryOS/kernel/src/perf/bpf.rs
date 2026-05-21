//! Software perf event with BPF program attachment + ringbuf output.
//!
//! Ported from `Starry-OS/StarryOS:ebpf-kmod` (`kernel/src/perf/bpf.rs`).
//! The user-visible ringbuf is created by `BpfPerfEvent::do_mmap`; tgoskits'
//! `FileLike` does not expose a `custom_mmap()` hook on perf-event fds at
//! the moment (#805 + #673 do not introduce one), so the mmap pathway is
//! reachable only through internal callers for now — a follow-up PR will
//! wire it into `mmap(2)`. The rest of the event lifecycle (enable /
//! disable / set_bpf_prog / write_event) matches the upstream behaviour.

use alloc::sync::Arc;
use core::{any::Any, fmt::Debug};

use ax_errno::{AxError, AxResult};
use ax_memory_addr::PhysAddr;
use axpoll::{IoEvents, PollSet, Pollable};
use kbpf_basic::{
    linux_bpf::perf_event_sample_format,
    perf::{PerfProbeArgs, bpf::BpfPerfEvent},
};
use rbpf::EbpfVmRaw;

use super::PerfEventOps;
use crate::{
    ebpf::{BPF_HELPER_FUN_SET, prog::BpfProg},
    file::FileLike,
};

/// Wraps `kbpf_basic::perf::bpf::BpfPerfEvent` with kernel state: a poll
/// set so readers can wait for new records, and the backing
/// `(PhysAddr, page_count)` produced by `do_mmap` (Some after the user
/// `mmap`s the ringbuf; None before).
pub struct BpfPerfEventWrapper {
    inner: BpfPerfEvent,
    poll_ready: PollSet,
    phys_addr: Option<(PhysAddr, usize)>,
}

impl BpfPerfEventWrapper {
    /// Construct the wrapper around a freshly-built `BpfPerfEvent`.
    pub fn new(inner: BpfPerfEvent) -> Self {
        Self {
            inner,
            poll_ready: PollSet::new(),
            phys_addr: None,
        }
    }

    /// Write a record into the ringbuf and wake any readers. Pre-mmap
    /// calls are accepted as no-ops (matching the source behaviour).
    pub fn write_event(&mut self, data: &[u8]) -> AxResult<()> {
        if self.phys_addr.is_none() {
            // Ringbuf not yet mapped by userland; drop the sample silently
            // — Linux behavior on EINVAL would alarm libbpf-style readers.
            return Ok(());
        }
        self.inner
            .write_event(data)
            .map_err(|_| AxError::InvalidInput)?;
        if self.inner.enabled() {
            self.poll_ready.wake();
        }
        Ok(())
    }
}

impl Debug for BpfPerfEventWrapper {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BpfPerfEventWrapper")
    }
}

impl PerfEventOps for BpfPerfEventWrapper {
    fn enable(&mut self) -> AxResult<()> {
        self.inner.enable().map_err(|_| AxError::InvalidInput)?;
        Ok(())
    }

    fn disable(&mut self) -> AxResult<()> {
        self.inner.disable().map_err(|_| AxError::InvalidInput)?;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Drop for BpfPerfEventWrapper {
    fn drop(&mut self) {
        // The mmap'd ringbuf pages, if any, were allocated via the global
        // page allocator in the (not yet wired) mmap path; once that path
        // lands, this drop will need to call `frame_dealloc` for each. For
        // now the field stays `None`, so this is effectively a no-op.
        let _ = self.phys_addr.take();
    }
}

impl Pollable for BpfPerfEventWrapper {
    fn poll(&self) -> axpoll::IoEvents {
        if self.inner.readable() {
            IoEvents::IN
        } else {
            IoEvents::empty()
        }
    }

    fn register(&self, context: &mut core::task::Context<'_>, events: axpoll::IoEvents) {
        if events.contains(IoEvents::IN) {
            self.poll_ready.register(context.waker());
        }
    }
}

/// Build a `BpfPerfEventWrapper` from `perf_event_open` args. The upstream
/// code asserts `sample_type == PERF_SAMPLE_RAW`; we keep that assertion
/// to match the verifier contract and surface bad input early.
pub fn perf_event_open_bpf(args: PerfProbeArgs) -> BpfPerfEventWrapper {
    debug_assert_eq!(
        args.sample_type,
        Some(perf_event_sample_format::PERF_SAMPLE_RAW)
    );
    BpfPerfEventWrapper::new(BpfPerfEvent::new(args))
}

/// Build a minimal `rbpf::EbpfVmRaw` around a loaded `BpfProg`'s
/// instruction stream and register the kernel helper table on it.
pub fn create_basic_ebpf_vm(bpf_prog: Arc<dyn FileLike>) -> AxResult<EbpfVmRaw<'static>> {
    let prog = bpf_prog
        .into_any_arc()
        .downcast::<BpfProg>()
        .map_err(|_| AxError::InvalidInput)?;
    let prog_slice = prog.insns();

    // SAFETY: `BpfProg`'s instruction buffer is `Arc`-rooted and the
    // resulting slice has the same lifetime as the `Arc`; we transmute the
    // borrow lifetime to `'static` because the `EbpfVmRaw` we return only
    // executes while a strong reference to `prog` is held by the caller
    // (kept alive via `PerfEventOps::set_bpf_prog`).
    let prog_slice = unsafe { core::slice::from_raw_parts(prog_slice.as_ptr(), prog_slice.len()) };
    let mut vm = EbpfVmRaw::new(Some(prog_slice)).map_err(|e| {
        axlog::error!("rbpf::EbpfVmRaw::new failed: {e:?}");
        AxError::InvalidInput
    })?;

    if let Some(table) = BPF_HELPER_FUN_SET.get() {
        for (key, value) in table.iter() {
            let _ = vm.register_helper(*key, *value);
        }
    }

    vm.register_allowed_memory(0..u64::MAX);
    Ok(vm)
}
