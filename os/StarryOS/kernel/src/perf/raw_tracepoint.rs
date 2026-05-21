//! Raw-tracepoint perf event, created via `bpf(BPF_RAW_TRACEPOINT_OPEN)`
//! rather than `perf_event_open(2)`. This gives the BPF program access to
//! the raw `&[u64]` args slice instead of the cooked text record.
//!
//! Adapted from `Starry-OS/StarryOS:ebpf-kmod`
//! (`kernel/src/perf/raw_tracepoint.rs`) to ktracepoint 0.6:
//! `RawTraceEventFunc::new(closure, data)` replaces the trait-object
//! callback, and registration goes through
//! `ExtTracePoint::register(TraceCallbackType::RawEvent(...))`.

use alloc::{borrow::Cow, boxed::Box, sync::Arc};
use core::any::Any;

use ax_errno::{AxError, AxResult};
use axpoll::Pollable;
use kbpf_basic::raw_tracepoint::BpfRawTracePointArg;
use ktracepoint::{RawTraceEventFunc, TraceCallbackType};
use rbpf::EbpfVmRaw;

use crate::{
    file::{FileLike, add_file_like, get_file_like},
    tracepoint::{KernelExtTracePoint, find_ext_tracepoint_by_name},
};

/// Per-fd raw tracepoint event: owns the ExtTracePoint Arc + the
/// registered callback so Drop can unregister it.
pub struct RawTracepointPerfEvent {
    ext_tp: KernelExtTracePoint,
    callback: Arc<RawTraceEventFunc>,
}

impl Pollable for RawTracepointPerfEvent {
    fn poll(&self) -> axpoll::IoEvents {
        axpoll::IoEvents::empty()
    }

    fn register(&self, _context: &mut core::task::Context<'_>, _events: axpoll::IoEvents) {
        // Raw tracepoint events deliver through the attached BPF program,
        // never through fd readiness.
    }
}

impl FileLike for RawTracepointPerfEvent {
    fn read(&self, _dst: &mut crate::file::IoDst) -> AxResult<usize> {
        Err(AxError::Unsupported)
    }

    fn write(&self, _src: &mut crate::file::IoSrc) -> AxResult<usize> {
        Err(AxError::Unsupported)
    }

    fn stat(&self) -> AxResult<crate::file::Kstat> {
        Ok(crate::file::Kstat::default())
    }

    fn path(&self) -> Cow<'_, str> {
        "anon_inode:[raw_tracepoint_perf_event]".into()
    }
}

impl Drop for RawTracepointPerfEvent {
    fn drop(&mut self) {
        self.ext_tp
            .lock()
            .unregister(TraceCallbackType::RawEvent(self.callback.clone()));
    }
}

impl RawTracepointPerfEvent {
    /// Register a BPF program as a raw-tracepoint callback on `ext_tp`.
    pub fn new(ext_tp: KernelExtTracePoint, bpf_prog: Arc<dyn FileLike>) -> AxResult<Self> {
        let vm = super::bpf::create_basic_ebpf_vm(bpf_prog.clone())?;

        struct Ctx {
            _bpf_prog: Arc<dyn FileLike>,
            vm: EbpfVmRaw<'static>,
        }
        let ctx = Box::new(Ctx {
            _bpf_prog: bpf_prog,
            vm,
        });

        let func: Box<dyn Fn(&[u64], &(dyn Any + Send + Sync)) + Send + Sync> =
            Box::new(|args: &[u64], data: &(dyn Any + Send + Sync)| {
                let ctx = data
                    .downcast_ref::<Ctx>()
                    .expect("raw_tracepoint Ctx mismatch");
                // SAFETY: raw tracepoint hands us the raw `&[u64]` arg
                // slice on the tracing fast path; the slice lives for the
                // duration of the call. The BPF VM wants a `&mut [u8]`
                // context view of the same bytes.
                let arg_bytes = unsafe {
                    core::slice::from_raw_parts_mut(
                        args.as_ptr() as *mut u8,
                        args.len() * core::mem::size_of::<u64>(),
                    )
                };
                if let Err(e) = ctx.vm.execute_program(arg_bytes) {
                    axlog::error!("raw_tracepoint BPF program failed: {e:?}");
                }
            });
        let callback = Arc::new(RawTraceEventFunc::new(func, ctx));
        ext_tp
            .lock()
            .register(TraceCallbackType::RawEvent(callback.clone()));
        Ok(Self { ext_tp, callback })
    }
}

/// Implementation of `bpf(BPF_RAW_TRACEPOINT_OPEN)`: look up the named
/// tracepoint, attach `prog_fd`, and return a fresh fd for the resulting
/// event (its lifetime keeps the callback registered).
pub fn bpf_raw_tracepoint_open(arg: BpfRawTracePointArg) -> AxResult<isize> {
    let ext_tp = find_ext_tracepoint_by_name(&arg.name).ok_or(AxError::InvalidInput)?;
    let prog = get_file_like(arg.prog_fd as _)?;
    let event = RawTracepointPerfEvent::new(ext_tp, prog)?;
    let fd = add_file_like(Arc::new(event), false)?;
    Ok(fd as isize)
}
