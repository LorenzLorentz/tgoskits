use ax_sync::Mutex;
use axfs_ng_vfs::VfsResult;
use ktracepoint::{TraceCmdLineCacheSnapshot, TracePipeSnapshot};

use crate::pseudofs::DirectRwFsFileOps;

/// File representing the trace content.
pub struct TraceFile(Mutex<Option<TracePipeSnapshot>>);

impl TraceFile {
    /// Creates a new `TraceFile` instance.
    pub const fn new() -> Self {
        TraceFile(Mutex::new(None))
    }
}

impl DirectRwFsFileOps for TraceFile {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        let offset = offset as usize;

        let mut guard = self.0.lock();
        if guard.is_none() || offset == 0 {
            let snapshot = super::TRACE_RAW_PIPE.lock().snapshot();
            *guard = Some(snapshot);
        }

        let snapshot = guard.as_mut().unwrap();

        let default_fmt_str = snapshot.default_fmt_str();
        if offset >= default_fmt_str.len() {
            Ok(super::common_trace_pipe_read(snapshot, buf))
        } else {
            let len = buf.len().min(default_fmt_str.len() - offset);
            buf[..len].copy_from_slice(&default_fmt_str.as_bytes()[offset..offset + len]);
            Ok(len)
        }
    }

    fn write_at(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        let mut trace_raw_pipe = super::TRACE_RAW_PIPE.lock();
        trace_raw_pipe.clear();
        Ok(buf.len())
    }
}

/// File representing the trace command line cache.
pub struct TraceCmdLineFile(Mutex<Option<TraceCmdLineCacheSnapshot>>);

impl TraceCmdLineFile {
    /// Creates a new `TraceCmdLineFile` instance.
    pub const fn new() -> Self {
        TraceCmdLineFile(Mutex::new(None))
    }
}

impl DirectRwFsFileOps for TraceCmdLineFile {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        let mut guard = self.0.lock();
        if guard.is_none() || offset == 0 {
            let snapshot = super::TRACE_CMDLINE_CACHE.lock().snapshot();
            *guard = Some(snapshot);
        }

        let mut copy_len = 0;
        let mut peek_flag = false;
        let snapshot = guard.as_mut().unwrap();
        loop {
            if let Some(record_str) = snapshot.peek() {
                if copy_len + record_str.len() > buf.len() {
                    break;
                }
                let len = record_str.len();
                buf[copy_len..copy_len + len].copy_from_slice(record_str.as_bytes());
                copy_len += len;
                peek_flag = true;
            }
            if peek_flag {
                snapshot.pop(); // Remove the record after reading
                peek_flag = false;
            } else {
                break; // No more records to read
            }
        }
        Ok(copy_len)
    }
}
