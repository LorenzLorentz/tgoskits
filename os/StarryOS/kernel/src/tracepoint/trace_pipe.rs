use core::{future::poll_fn, task::Poll};

use ax_task::{
    current,
    future::{block_on, interruptible},
};
use axfs_ng_vfs::VfsResult;
use ktracepoint::TracePipeOps;

use crate::{pseudofs::DirectRwFsFileOps, task::AsThread, tracepoint::TRACE_RAW_PIPE};

/// File representing the trace pipe.
pub struct TracePipeFile;

impl TracePipeFile {
    fn readable(&self) -> bool {
        let trace_raw_pipe = TRACE_RAW_PIPE.lock();
        !trace_raw_pipe.is_empty()
    }
}

impl DirectRwFsFileOps for TracePipeFile {
    fn read_at(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        let curr = current();
        let proc_data = &curr.as_thread().proc_data;

        let read_len = loop {
            let mut trace_raw_pipe = TRACE_RAW_PIPE.lock();
            let read_len = super::common_trace_pipe_read(&mut *trace_raw_pipe, buf);
            if read_len != 0 {
                break read_len;
            }
            // Release the lock before waiting
            drop(trace_raw_pipe);
            // wait for new data
            let _result = block_on(interruptible(poll_fn(|cx| {
                if self.readable() {
                    Poll::Ready(true)
                } else {
                    proc_data.child_exit_event.register(cx.waker());
                    Poll::Pending
                }
            })))?;
        };
        Ok(read_len)
    }
}
