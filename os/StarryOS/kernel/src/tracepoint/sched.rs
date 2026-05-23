//! `sched:*` tracepoints.
//!
//! `sched_switch` is fired by `ax-task` through the cross-crate
//! [`ax_task::SchedTracepoint`] interface (gated by `tracepoint-hooks`);
//! `sched_process_fork` and `sched_process_exit` are emitted directly from
//! Starry's clone and exit paths.

use ax_task::SchedTracepoint;

ktracepoint::define_event_trace!(
    sched_switch,
    TP_kops(crate::tracepoint::KernelTraceAux),
    TP_system(sched),
    TP_PROTO(prev_tid: u64, next_tid: u64, prev_state: u32),
    TP_STRUCT__entry {
        prev_tid: u64,
        next_tid: u64,
        prev_state: u32,
    },
    TP_fast_assign {
        prev_tid: prev_tid,
        next_tid: next_tid,
        prev_state: prev_state,
    },
    TP_ident(__entry),
    TP_printk({
        alloc::format!(
            "prev_tid={} next_tid={} prev_state={}",
            __entry.prev_tid,
            __entry.next_tid,
            __entry.prev_state,
        )
    })
);

ktracepoint::define_event_trace!(
    sched_process_fork,
    TP_kops(crate::tracepoint::KernelTraceAux),
    TP_system(sched),
    TP_PROTO(parent_tid: u64, child_tid: u64),
    TP_STRUCT__entry {
        parent_tid: u64,
        child_tid: u64,
    },
    TP_fast_assign {
        parent_tid: parent_tid,
        child_tid: child_tid,
    },
    TP_ident(__entry),
    TP_printk({
        alloc::format!(
            "parent_tid={} child_tid={}",
            __entry.parent_tid,
            __entry.child_tid,
        )
    })
);

ktracepoint::define_event_trace!(
    sched_process_exit,
    TP_kops(crate::tracepoint::KernelTraceAux),
    TP_system(sched),
    TP_PROTO(tid: u64, exit_code: i32),
    TP_STRUCT__entry {
        tid: u64,
        exit_code: i32,
    },
    TP_fast_assign {
        tid: tid,
        exit_code: exit_code,
    },
    TP_ident(__entry),
    TP_printk({
        alloc::format!(
            "tid={} exit_code={}",
            __entry.tid,
            __entry.exit_code,
        )
    })
);

struct SchedTracepointImpl;

#[ax_crate_interface::impl_interface]
impl SchedTracepoint for SchedTracepointImpl {
    fn on_sched_switch(prev_tid: u64, next_tid: u64, prev_state: u32) {
        trace_sched_switch(prev_tid, next_tid, prev_state);
    }
}
