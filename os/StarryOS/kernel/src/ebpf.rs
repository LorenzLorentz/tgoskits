use alloc::vec::Vec;

use ax_errno::{AxError, AxResult};
use ax_sync::spin::SpinNoIrq;

#[allow(dead_code)]
mod map_types {
    pub const BPF_MAP_TYPE_UNSPEC: u32 = 0;
    pub const BPF_MAP_TYPE_HASH: u32 = 1;
    pub const BPF_MAP_TYPE_ARRAY: u32 = 2;
    pub const BPF_MAP_TYPE_PROG_ARRAY: u32 = 3;
    pub const BPF_MAP_TYPE_PERF_EVENT_ARRAY: u32 = 4;
    pub const BPF_MAP_TYPE_PERCPU_HASH: u32 = 5;
    pub const BPF_MAP_TYPE_PERCPU_ARRAY: u32 = 6;
    pub const BPF_MAP_TYPE_STACK_TRACE: u32 = 7;
    pub const BPF_MAP_TYPE_CGROUP_ARRAY: u32 = 8;
    pub const BPF_MAP_TYPE_LRU_HASH: u32 = 9;
    pub const BPF_MAP_TYPE_LRU_PERCPU_HASH: u32 = 10;
    pub const BPF_MAP_TYPE_LPM_TRIE: u32 = 11;
    pub const BPF_MAP_TYPE_ARRAY_OF_MAPS: u32 = 12;
    pub const BPF_MAP_TYPE_HASH_OF_MAPS: u32 = 13;
    pub const BPF_MAP_TYPE_DEVMAP: u32 = 14;
    pub const BPF_MAP_TYPE_SOCKMAP: u32 = 15;
    pub const BPF_MAP_TYPE_CPUMAP: u32 = 16;
    pub const BPF_MAP_TYPE_XSKMAP: u32 = 17;
    pub const BPF_MAP_TYPE_SOCKHASH: u32 = 18;
    pub const BPF_MAP_TYPE_CGROUP_STORAGE: u32 = 19;
    pub const BPF_MAP_TYPE_REUSEPORT_SOCKARRAY: u32 = 20;
    pub const BPF_MAP_TYPE_PERCPU_CGROUP_STORAGE: u32 = 21;
    pub const BPF_MAP_TYPE_QUEUE: u32 = 22;
    pub const BPF_MAP_TYPE_STACK: u32 = 23;
    pub const BPF_MAP_TYPE_SK_STORAGE: u32 = 24;
    pub const BPF_MAP_TYPE_DEVMAP_HASH: u32 = 25;
    pub const BPF_MAP_TYPE_STRUCT_OPS: u32 = 26;
    pub const BPF_MAP_TYPE_RINGBUF: u32 = 27;
    pub const BPF_MAP_TYPE_INODE_STORAGE: u32 = 28;
    pub const BPF_MAP_TYPE_TASK_STORAGE: u32 = 29;
}

#[allow(dead_code)]
mod prog_types {
    pub const BPF_PROG_TYPE_UNSPEC: u32 = 0;
    pub const BPF_PROG_TYPE_SOCKET_FILTER: u32 = 1;
    pub const BPF_PROG_TYPE_KPROBE: u32 = 2;
    pub const BPF_PROG_TYPE_SCHED_CLS: u32 = 3;
    pub const BPF_PROG_TYPE_SCHED_ACT: u32 = 4;
    pub const BPF_PROG_TYPE_TRACEPOINT: u32 = 5;
    pub const BPF_PROG_TYPE_XDP: u32 = 6;
    pub const BPF_PROG_TYPE_PERF_EVENT: u32 = 7;
    pub const BPF_PROG_TYPE_CGROUP_SKB: u32 = 8;
    pub const BPF_PROG_TYPE_CGROUP_SOCK: u32 = 9;
    pub const BPF_PROG_TYPE_LWT_IN: u32 = 10;
    pub const BPF_PROG_TYPE_LWT_OUT: u32 = 11;
    pub const BPF_PROG_TYPE_LWT_XMIT: u32 = 12;
    pub const BPF_PROG_TYPE_SOCK_OPS: u32 = 13;
    pub const BPF_PROG_TYPE_SK_SKB: u32 = 14;
    pub const BPF_PROG_TYPE_CGROUP_DEVICE: u32 = 15;
    pub const BPF_PROG_TYPE_SK_MSG: u32 = 16;
    pub const BPF_PROG_TYPE_RAW_TRACEPOINT: u32 = 17;
    pub const BPF_PROG_TYPE_CGROUP_SOCK_ADDR: u32 = 18;
    pub const BPF_PROG_TYPE_LSM: u32 = 29;
    pub const BPF_PROG_TYPE_SYSCALL: u32 = 31;
}

#[allow(dead_code)]
mod cmd {
    pub const BPF_MAP_CREATE: u64 = 0;
    pub const BPF_MAP_LOOKUP_ELEM: u64 = 1;
    pub const BPF_MAP_UPDATE_ELEM: u64 = 2;
    pub const BPF_MAP_DELETE_ELEM: u64 = 3;
    pub const BPF_MAP_GET_NEXT_KEY: u64 = 4;
    pub const BPF_PROG_LOAD: u64 = 5;
    pub const BPF_OBJ_PIN: u64 = 6;
    pub const BPF_OBJ_GET: u64 = 7;
    pub const BPF_PROG_ATTACH: u64 = 8;
    pub const BPF_PROG_DETACH: u64 = 9;
    pub const BPF_PROG_TEST_RUN: u64 = 10;
    pub const BPF_PROG_GET_NEXT_ID: u64 = 11;
    pub const BPF_MAP_GET_NEXT_ID: u64 = 12;
    pub const BPF_PROG_GET_FD_BY_ID: u64 = 13;
    pub const BPF_MAP_GET_FD_BY_ID: u64 = 14;
    pub const BPF_OBJ_GET_INFO_BY_FD: u64 = 15;
    pub const BPF_PROG_QUERY: u64 = 16;
    pub const BPF_RAW_TRACEPOINT_OPEN: u64 = 17;
    pub const BPF_BTF_LOAD: u64 = 18;
    pub const BPF_BTF_GET_FD_BY_ID: u64 = 19;
    pub const BPF_TASK_FD_QUERY: u64 = 20;
    pub const BPF_MAP_LOOKUP_AND_DELETE_ELEM: u64 = 21;
    pub const BPF_MAP_FREEZE: u64 = 22;
    pub const BPF_BTF_GET_NEXT_ID: u64 = 23;
    pub const BPF_MAP_LOOKUP_BATCH: u64 = 24;
    pub const BPF_MAP_LOOKUP_AND_DELETE_BATCH: u64 = 25;
    pub const BPF_MAP_UPDATE_BATCH: u64 = 26;
    pub const BPF_MAP_DELETE_BATCH: u64 = 27;
    pub const BPF_LINK_CREATE: u64 = 28;
    pub const BPF_LINK_UPDATE: u64 = 29;
    pub const BPF_LINK_GET_FD_BY_ID: u64 = 30;
    pub const BPF_LINK_GET_NEXT_ID: u64 = 31;
    pub const BPF_ENABLE_STATS: u64 = 32;
    pub const BPF_ITER_CREATE: u64 = 33;
    pub const BPF_LINK_DETACH: u64 = 34;
    pub const BPF_PROG_BIND_MAP: u64 = 35;
}

#[allow(dead_code)]
struct BpfMapDef {
    map_type: u32,
    key_size: u32,
    value_size: u32,
    max_entries: u32,
    map_flags: u32,
    inner_map_fd: u32,
    numa_node: u32,
}

#[allow(dead_code)]
struct BpfProgDef {
    prog_type: u32,
    insn_cnt: u32,
    insns: usize,
    license: usize,
    log_level: u32,
    log_size: u32,
    log_buf: usize,
    kern_version: u32,
    prog_flags: u32,
    prog_name: [u8; 16],
    prog_ifindex: u32,
    expected_attach_type: u32,
}

#[allow(dead_code)]
struct BpfMap {
    map_type: u32,
    key_size: u32,
    value_size: u32,
    max_entries: u32,
    map_flags: u32,
    id: u32,
}

#[allow(dead_code)]
struct BpfProg {
    prog_type: u32,
    insn_cnt: u32,
    id: u32,
}

#[allow(dead_code)]
static BPF_MAPS: SpinNoIrq<Vec<BpfMap>> = SpinNoIrq::new(Vec::new());
#[allow(dead_code)]
static BPF_PROGS: SpinNoIrq<Vec<BpfProg>> = SpinNoIrq::new(Vec::new());

fn next_map_id() -> u32 {
    BPF_MAPS.lock().len() as u32
}

fn next_prog_id() -> u32 {
    BPF_PROGS.lock().len() as u32
}

fn handle_map_create(_uattr: usize, _size: u32) -> AxResult<isize> {
    let id = next_map_id();
    BPF_MAPS.lock().push(BpfMap {
        map_type: 0,
        key_size: 0,
        value_size: 0,
        max_entries: 0,
        map_flags: 0,
        id,
    });
    warn!("bpf: BPF_MAP_CREATE stub, returning fd for map id {id}");
    Ok(id as isize)
}

fn handle_prog_load(_uattr: usize, _size: u32) -> AxResult<isize> {
    let id = next_prog_id();
    BPF_PROGS.lock().push(BpfProg {
        prog_type: 0,
        insn_cnt: 0,
        id,
    });
    warn!("bpf: BPF_PROG_LOAD stub, returning fd for prog id {id}");
    Ok(id as isize)
}

fn handle_raw_tracepoint_open(_uattr: usize, _size: u32) -> AxResult<isize> {
    warn!("bpf: BPF_RAW_TRACEPOINT_OPEN stub");
    Err(AxError::Unsupported)
}

pub fn sys_bpf(cmd: u64, uattr: usize, size: u32) -> AxResult<isize> {
    match cmd {
        cmd::BPF_MAP_CREATE => handle_map_create(uattr, size),
        cmd::BPF_PROG_LOAD => handle_prog_load(uattr, size),
        cmd::BPF_RAW_TRACEPOINT_OPEN => handle_raw_tracepoint_open(uattr, size),
        cmd::BPF_MAP_LOOKUP_ELEM
        | cmd::BPF_MAP_UPDATE_ELEM
        | cmd::BPF_MAP_DELETE_ELEM
        | cmd::BPF_MAP_GET_NEXT_KEY
        | cmd::BPF_MAP_LOOKUP_AND_DELETE_ELEM => {
            warn!("bpf: map elem operation {cmd} not yet implemented");
            Err(AxError::Unsupported)
        }
        cmd::BPF_OBJ_PIN | cmd::BPF_OBJ_GET => {
            warn!("bpf: obj pin/get not yet implemented");
            Err(AxError::Unsupported)
        }
        cmd::BPF_PROG_ATTACH | cmd::BPF_PROG_DETACH => {
            warn!("bpf: prog attach/detach not yet implemented");
            Err(AxError::Unsupported)
        }
        cmd::BPF_LINK_CREATE => {
            warn!("bpf: BPF_LINK_CREATE not yet implemented");
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
