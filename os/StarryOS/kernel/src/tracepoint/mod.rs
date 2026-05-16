//! See Linux Documentation for details: <https://docs.kernel.org/trace/ftrace.html>
mod control;
mod trace;
mod trace_pipe;

use alloc::{collections::BTreeMap, string::ToString, sync::Arc};
use core::{num::NonZero, ops::Deref};

use ax_errno::{AxError, AxResult};
use ax_hal::{percpu::this_cpu_id, time::monotonic_time_nanos};
use ax_lazyinit::LazyInit;
use ax_memory_addr::VirtAddr;
use ax_sync::Mutex;
use ax_task::current;
use axfs_ng_vfs::NodePermission;
use ktracepoint::*;

use crate::{
    pseudofs::{DirMaker, DirMapping, SeqObject, SimpleDir, SimpleFs, SpecialFsFile},
    task::AsThread,
};

pub type KernelExtTracePoint = Arc<Mutex<ExtTracePoint<KernelTraceAux>>>;

static TRACE_POINT_MAP: LazyInit<TracePointMap<KernelTraceAux>> = LazyInit::new();

static TRACE_RAW_PIPE: Mutex<TracePipeRaw> = Mutex::new(TracePipeRaw::new(4096));

static TRACE_CMDLINE_CACHE: LazyInit<Mutex<TraceCmdLineCache>> = LazyInit::new();

static EXT_TRACEPOINTS: LazyInit<BTreeMap<u32, KernelExtTracePoint>> = LazyInit::new();

pub struct KernelTraceAux;

impl KernelTraceOps for KernelTraceAux {
    fn current_pid() -> u32 {
        let curr = current();
        let proc_data = &curr.as_thread().proc_data;
        proc_data.proc.pid()
    }

    fn trace_pipe_push_raw_record(buf: &[u8]) {
        // log::debug!("trace_pipe_push_raw_record: {}", record.len());
        TRACE_RAW_PIPE
            .lock()
            .push_record(monotonic_time_nanos(), this_cpu_id() as _, buf.to_vec());
    }

    fn trace_cmdline_push(pid: u32) {
        let curr = current();
        let proc_data = &curr.as_thread().proc_data;
        let exe_path = proc_data.exe_path.read();
        let pname = exe_path
            .split(' ')
            .next()
            .unwrap_or("unknown")
            .split('/')
            .next_back()
            .unwrap_or("unknown");
        TRACE_CMDLINE_CACHE.lock().insert(pid, pname);
    }

    fn write_kernel_text(addr: *mut core::ffi::c_void, data: &[u8]) {
        crate::mm::write_kernel_text(VirtAddr::from_mut_ptr_of(addr), data)
            .expect("Failed to write kernel text");
    }

    fn read_tracepoint_state<R>(id: u32, f: impl FnOnce(&ExtTracePoint<Self>) -> R) -> R {
        let ext_tp = EXT_TRACEPOINTS
            .deref()
            .get(&id)
            .expect("Tracepoint not found");
        f(ext_tp.lock().deref())
    }

    fn write_tracepoint_state<R>(id: u32, f: impl FnOnce(&mut ExtTracePoint<Self>) -> R) -> R {
        let ext_tp = EXT_TRACEPOINTS
            .deref()
            .get(&id)
            .expect("Tracepoint not found");
        let mut ext_tp = ext_tp.lock();
        f(&mut ext_tp)
    }
}

fn common_trace_pipe_read(trace_buf: &mut dyn TracePipeOps, buf: &mut [u8]) -> usize {
    let trace_cmdline_cache = TRACE_CMDLINE_CACHE.lock();
    // read real trace data
    let mut copy_len = 0;
    let mut peek_flag = false;
    loop {
        if let Some(record) = trace_buf.peek() {
            let record_str = TraceEntryParser::parse::<KernelTraceAux>(
                &TRACE_POINT_MAP,
                &trace_cmdline_cache,
                record,
            );
            if copy_len + record_str.len() > buf.len() {
                break;
            }
            let len = record_str.len();
            buf[copy_len..copy_len + len].copy_from_slice(record_str.as_bytes());
            copy_len += len;
            peek_flag = true;
        }
        if peek_flag {
            trace_buf.pop(); // Remove the record after reading
            peek_flag = false;
        } else {
            break;
        }
    }
    copy_len
}

/// Initialize registered tracepoints. This should be called after static keys are initialized, and before any tracepoint is hit.
pub fn tracepoint_init() -> AxResult<()> {
    let (tp_map, ext_tps) =
        global_init_events::<KernelTraceAux>().map_err(|_| AxError::InvalidInput)?;

    let ext_tps = ext_tps
        .into_iter()
        .map(|ext_tp| (ext_tp.id(), Arc::new(Mutex::new(ext_tp))))
        .collect::<BTreeMap<_, _>>();

    ax_println!("Initialized {} tracepoints", tp_map.len());
    TRACE_POINT_MAP.init_once(tp_map);
    EXT_TRACEPOINTS.init_once(ext_tps);
    TRACE_CMDLINE_CACHE.init_once(Mutex::new(TraceCmdLineCache::new(
        NonZero::new(4096).unwrap(),
    )));
    Ok(())
}

/// Initialize events directory in debugfs
fn init_events(fs: Arc<SimpleFs>) -> DirMaker {
    let mut events_root = DirMapping::new();
    let mut subsystem = BTreeMap::new();

    for ext_tp in EXT_TRACEPOINTS.deref().values() {
        let tp = ext_tp.lock().trace_point();
        let subsystem_name = tp.system();
        let event_name = tp.name();

        let subsystem_root = {
            if !subsystem.contains_key(subsystem_name) {
                let new_root = DirMapping::new();
                subsystem.insert(subsystem_name.to_string(), new_root);
            }
            subsystem.get_mut(subsystem_name).unwrap()
        };

        let mut event_root = DirMapping::new();
        event_root.add(
            "enable",
            SpecialFsFile::new_regular_with_perm(
                fs.clone(),
                control::EventEnableObj::new(ext_tp.clone()),
                NodePermission::from_bits_truncate(0o640),
            ),
        );
        event_root.add("format", {
            let seq_obj = SeqObject::new({
                let format_file = TracePointFormatFile::new(tp);
                move || Ok(format_file.read())
            });
            SpecialFsFile::new_regular_with_perm(
                fs.clone(),
                seq_obj,
                NodePermission::from_bits_truncate(0o440),
            )
        });

        event_root.add("id", {
            let seq_obj = SeqObject::new({
                let id_file = TracePointIdFile::new(tp);
                move || Ok(id_file.read())
            });
            SpecialFsFile::new_regular_with_perm(
                fs.clone(),
                seq_obj,
                NodePermission::from_bits_truncate(0o440),
            )
        });
        event_root.add(
            "filter",
            SpecialFsFile::new_regular_with_perm(
                fs.clone(),
                control::EventFilterObj::new(ext_tp.clone()),
                NodePermission::from_bits_truncate(0o640),
            ),
        );
        subsystem_root.add(
            event_name,
            SimpleDir::new_maker(fs.clone(), Arc::new(event_root)),
        );
    }
    for (subsystem_name, subsystem_root) in subsystem {
        events_root.add(
            &subsystem_name,
            SimpleDir::new_maker(fs.clone(), Arc::new(subsystem_root)),
        );
    }
    SimpleDir::new_maker(fs, Arc::new(events_root))
}

/// Initialize tracing directory in debugfs
pub fn init_tracing_dir(fs: Arc<SimpleFs>) -> DirMaker {
    let mut tracing_root = DirMapping::new();
    tracing_root.add(
        "saved_cmdlines_size",
        SpecialFsFile::new_regular_with_perm(
            fs.clone(),
            control::TraceCmdLineSizeObj,
            NodePermission::from_bits_truncate(0o640),
        ),
    );
    tracing_root.add(
        "trace_pipe",
        SpecialFsFile::new_regular_with_perm(
            fs.clone(),
            trace_pipe::TracePipeFile,
            NodePermission::from_bits_truncate(0o440),
        ),
    );
    tracing_root.add(
        "saved_cmdlines",
        SpecialFsFile::new_regular_with_perm(
            fs.clone(),
            trace::TraceCmdLineFile::new(),
            NodePermission::from_bits_truncate(0o440),
        ),
    );
    tracing_root.add(
        "trace",
        SpecialFsFile::new_regular_with_perm(
            fs.clone(),
            trace::TraceFile::new(),
            NodePermission::from_bits_truncate(0o640),
        ),
    );
    tracing_root.add("events", init_events(fs.clone()));
    SimpleDir::new_maker(fs, Arc::new(tracing_root))
}
