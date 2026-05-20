use ax_errno::{AxError, AxResult};

pub fn sys_bpf(cmd: u64, _uattr: usize, _size: u32) -> AxResult<isize> {
    match cmd {
        0 => {
            warn!("bpf: BPF_MAP_CREATE not yet implemented");
            Err(AxError::Unsupported)
        }
        5 => {
            warn!("bpf: BPF_PROG_LOAD not yet implemented");
            Err(AxError::Unsupported)
        }
        1..=4 => {
            warn!("bpf: map operation {cmd} not yet implemented");
            Err(AxError::Unsupported)
        }
        17 => {
            warn!("bpf: BPF_RAW_TRACEPOINT_OPEN not yet implemented");
            Err(AxError::Unsupported)
        }
        6..=7 => {
            warn!("bpf: obj operation {cmd} not yet implemented");
            Err(AxError::Unsupported)
        }
        _ => {
            warn!("bpf: unknown command {cmd}");
            Err(AxError::Unsupported)
        }
    }
}

pub fn sys_perf_event_open(
    _attr_uptr: usize,
    pid: i32,
    cpu: i32,
    group_fd: i32,
    flags: u64,
) -> AxResult<isize> {
    warn!(
        "perf_event_open: pid={pid}, cpu={cpu}, group_fd={group_fd}, flags={flags:#x} not yet \
         implemented"
    );
    Err(AxError::Unsupported)
}
