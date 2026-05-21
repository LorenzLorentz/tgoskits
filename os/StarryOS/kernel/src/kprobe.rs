use alloc::{
    alloc::{Layout, alloc, dealloc},
    sync::Arc,
};
use core::sync::atomic::{AtomicBool, Ordering};

use ax_memory_addr::{MemoryAddr, PAGE_SIZE_4K, VirtAddr};
use kprobe::{
    KprobeAuxiliaryOps, KretprobeBuilder, ProbeBuilder, ProbeManager, ProbePointList,
    register_kprobe as kprobe_crate_register_kprobe,
    register_kretprobe as kprobe_crate_register_kretprobe,
    unregister_kprobe as kprobe_crate_unregister_kprobe,
    unregister_kretprobe as kprobe_crate_unregister_kretprobe,
};
use lock_api::RawMutex;

use crate::task::AsThread;

pub struct KernelRawMutex {
    locked: AtomicBool,
}

unsafe impl RawMutex for KernelRawMutex {
    const INIT: Self = KernelRawMutex {
        locked: AtomicBool::new(false),
    };

    type GuardMarker = lock_api::GuardNoSend;

    fn lock(&self) {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    fn try_lock(&self) -> bool {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    unsafe fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
pub struct KernelKprobeOps;

impl KprobeAuxiliaryOps for KernelKprobeOps {
    fn copy_memory(src: *const u8, dst: *mut u8, len: usize, user_pid: Option<i32>) {
        if let Some(_pid) = user_pid {
            unsafe {
                let buf =
                    core::slice::from_raw_parts_mut(dst as *mut core::mem::MaybeUninit<u8>, len);
                let _ = starry_vm::vm_read_slice(src, buf);
            }
        } else {
            unsafe {
                core::ptr::copy_nonoverlapping(src, dst, len);
            }
        }
    }

    fn set_writeable_for_address<F: FnOnce(*mut u8)>(
        address: usize,
        len: usize,
        user_pid: Option<i32>,
        action: F,
    ) {
        if user_pid.is_some() {
            unimplemented!("user space breakpoint insertion not yet supported")
        }
        let addr = VirtAddr::from(address);
        let aligned_addr = addr.align_down_4k();
        let aligned_end = (addr + len).align_up_4k();
        let aligned_length: usize = aligned_end - aligned_addr;

        crate::stop_machine::stop_machine(
            move || {
                let mut guard = ax_mm::kernel_aspace().lock();
                let (_, original_flags, _) = guard.page_table().query(aligned_addr).unwrap();
                guard
                    .protect(
                        aligned_addr,
                        aligned_length,
                        original_flags | ax_hal::paging::MappingFlags::WRITE,
                    )
                    .unwrap();
                flush_tlb_range(aligned_addr, aligned_length);
                action(addr.as_mut_ptr());
                #[cfg(target_arch = "aarch64")]
                ax_hal::asm::clean_dcache_range_to_pou(addr, len);
                guard
                    .protect(aligned_addr, aligned_length, original_flags)
                    .unwrap();
            },
            move || {
                flush_tlb_range(aligned_addr, aligned_length);
                ax_hal::asm::flush_icache_all();
            },
        );
    }

    fn alloc_kernel_exec_memory() -> *mut u8 {
        let layout = Layout::from_size_align(PAGE_SIZE_4K, PAGE_SIZE_4K).unwrap();
        unsafe { alloc(layout) }
    }

    fn free_kernel_exec_memory(ptr: *mut u8) {
        let layout = Layout::from_size_align(PAGE_SIZE_4K, PAGE_SIZE_4K).unwrap();
        unsafe { dealloc(ptr, layout) }
    }

    fn alloc_user_exec_memory<F: FnOnce(*mut u8)>(_pid: Option<i32>, _action: F) -> *mut u8 {
        unimplemented!("user exec memory allocation for uprobes not yet supported")
    }

    fn free_user_exec_memory(_pid: Option<i32>, _ptr: *mut u8) {
        unimplemented!("user exec memory deallocation for uprobes not yet supported")
    }

    fn insert_kretprobe_instance_to_task(instance: kprobe::retprobe::RetprobeInstance) {
        let curr = ax_task::current();
        curr.as_thread()
            .proc_data
            .kretprobe_stack
            .lock()
            .push(instance);
    }

    fn pop_kretprobe_instance_from_task() -> kprobe::retprobe::RetprobeInstance {
        let curr = ax_task::current();
        curr.as_thread()
            .proc_data
            .kretprobe_stack
            .lock()
            .pop()
            .expect("kretprobe instance stack underflow")
    }
}

type KprobeManager = kprobe::ProbeManager<KernelRawMutex, KernelKprobeOps>;
type KprobePointList = ProbePointList<KernelKprobeOps>;

/// Type alias matching what the upstream `ebpf-kmod` perf module names a
/// concrete `kprobe::Kprobe` parameterized on the kernel's mutex and
/// auxiliary ops.
pub type KernelKprobe = kprobe::Kprobe<KernelRawMutex, KernelKprobeOps>;
/// Type alias matching what the upstream `ebpf-kmod` perf module names a
/// concrete `kprobe::Kretprobe`.
pub type KernelKretprobe = kprobe::Kretprobe<KernelRawMutex, KernelKprobeOps>;
/// Re-export name used by `perf/{kprobe,uprobe}.rs` as the
/// `KprobeAuxiliaryOps` impl: source uses `KprobeAuxiliary`, we keep the
/// `KernelKprobeOps` impl from #805 unchanged and alias it here.
pub type KprobeAuxiliary = KernelKprobeOps;

static KPROBE_MANAGER: ax_sync::spin::SpinNoIrq<Option<KprobeManager>> =
    ax_sync::spin::SpinNoIrq::new(None);
static KPROBE_POINT_LIST: ax_sync::spin::SpinNoIrq<Option<KprobePointList>> =
    ax_sync::spin::SpinNoIrq::new(None);

fn with_manager<F, R>(f: F) -> R
where
    F: FnOnce(&mut KprobeManager) -> R,
{
    let mut guard = KPROBE_MANAGER.lock();
    if guard.is_none() {
        *guard = Some(KprobeManager::default());
    }
    f(guard.as_mut().unwrap())
}

fn with_manager_and_list<F, R>(f: F) -> R
where
    F: FnOnce(&mut KprobeManager, &mut KprobePointList) -> R,
{
    let mut mgr = KPROBE_MANAGER.lock();
    if mgr.is_none() {
        *mgr = Some(KprobeManager::default());
    }
    let mut list = KPROBE_POINT_LIST.lock();
    if list.is_none() {
        *list = Some(KprobePointList::new());
    }
    f(mgr.as_mut().unwrap(), list.as_mut().unwrap())
}

/// Register a kprobe described by `builder` into the global manager and
/// return the live `Arc<KernelKprobe>`.
pub fn register_kprobe(builder: ProbeBuilder<KernelKprobeOps>) -> Arc<KernelKprobe> {
    with_manager_and_list(|mgr, list| kprobe_crate_register_kprobe(mgr, list, builder))
}

/// Unregister a previously registered kprobe.
pub fn unregister_kprobe(kprobe: Arc<KernelKprobe>) {
    with_manager_and_list(|mgr, list| kprobe_crate_unregister_kprobe(mgr, list, kprobe));
}

/// Register a kretprobe and return its live handle.
pub fn register_kretprobe(builder: KretprobeBuilder<KernelRawMutex>) -> Arc<KernelKretprobe> {
    with_manager_and_list(|mgr, list| kprobe_crate_register_kretprobe(mgr, list, builder))
}

/// Unregister a previously registered kretprobe.
pub fn unregister_kretprobe(kretprobe: Arc<KernelKretprobe>) {
    with_manager_and_list(|mgr, list| kprobe_crate_unregister_kretprobe(mgr, list, kretprobe));
}

fn trapframe_to_ptregs(tf: &ax_hal::context::TrapFrame) -> kprobe::PtRegs {
    #[cfg(target_arch = "x86_64")]
    {
        kprobe::PtRegs {
            r15: tf.r15 as usize,
            r14: tf.r14 as usize,
            r13: tf.r13 as usize,
            r12: tf.r12 as usize,
            rbp: tf.rbp as usize,
            rbx: tf.rbx as usize,
            r11: tf.r11 as usize,
            r10: tf.r10 as usize,
            r9: tf.r9 as usize,
            r8: tf.r8 as usize,
            rax: tf.rax as usize,
            rcx: tf.rcx as usize,
            rdx: tf.rdx as usize,
            rsi: tf.rsi as usize,
            rdi: tf.rdi as usize,
            orig_rax: 0,
            rip: tf.rip as usize,
            cs: tf.cs as usize,
            rflags: tf.rflags as usize,
            rsp: tf.rsp as usize,
            ss: tf.ss as usize,
        }
    }
    #[cfg(target_arch = "riscv64")]
    {
        kprobe::PtRegs {
            epc: tf.sepc,
            ra: tf.regs.ra,
            sp: tf.regs.sp,
            gp: tf.regs.gp,
            tp: tf.regs.tp,
            t0: tf.regs.t0,
            t1: tf.regs.t1,
            t2: tf.regs.t2,
            s0: tf.regs.s0,
            s1: tf.regs.s1,
            a0: tf.regs.a0,
            a1: tf.regs.a1,
            a2: tf.regs.a2,
            a3: tf.regs.a3,
            a4: tf.regs.a4,
            a5: tf.regs.a5,
            a6: tf.regs.a6,
            a7: tf.regs.a7,
            s2: tf.regs.s2,
            s3: tf.regs.s3,
            s4: tf.regs.s4,
            s5: tf.regs.s5,
            s6: tf.regs.s6,
            s7: tf.regs.s7,
            s8: tf.regs.s8,
            s9: tf.regs.s9,
            s10: tf.regs.s10,
            s11: tf.regs.s11,
            t3: tf.regs.t3,
            t4: tf.regs.t4,
            t5: tf.regs.t5,
            t6: tf.regs.t6,
            status: tf.sstatus.bits(),
            badaddr: 0,
            cause: 0,
            orig_a0: tf.regs.a0,
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        kprobe::PtRegs {
            regs: tf.x,
            sp: 0,
            pc: tf.elr,
            pstate: tf.spsr,
            orig_x0: tf.x[0],
            syscallno: -1,
            unused2: 0,
        }
    }
}

fn ptregs_write_back(pt: &kprobe::PtRegs, tf: &mut ax_hal::context::TrapFrame) {
    #[cfg(target_arch = "x86_64")]
    {
        tf.r15 = pt.r15 as u64;
        tf.r14 = pt.r14 as u64;
        tf.r13 = pt.r13 as u64;
        tf.r12 = pt.r12 as u64;
        tf.rbp = pt.rbp as u64;
        tf.rbx = pt.rbx as u64;
        tf.r11 = pt.r11 as u64;
        tf.r10 = pt.r10 as u64;
        tf.r9 = pt.r9 as u64;
        tf.r8 = pt.r8 as u64;
        tf.rax = pt.rax as u64;
        tf.rcx = pt.rcx as u64;
        tf.rdx = pt.rdx as u64;
        tf.rsi = pt.rsi as u64;
        tf.rdi = pt.rdi as u64;
        tf.rip = pt.rip as u64;
        tf.cs = pt.cs as u64;
        tf.rflags = pt.rflags as u64;
        tf.rsp = pt.rsp as u64;
        tf.ss = pt.ss as u64;
    }
    #[cfg(target_arch = "riscv64")]
    {
        tf.sepc = pt.epc;
        tf.regs.ra = pt.ra;
        tf.regs.sp = pt.sp;
        tf.regs.gp = pt.gp;
        tf.regs.tp = pt.tp;
        tf.regs.t0 = pt.t0;
        tf.regs.t1 = pt.t1;
        tf.regs.t2 = pt.t2;
        tf.regs.s0 = pt.s0;
        tf.regs.s1 = pt.s1;
        tf.regs.a0 = pt.a0;
        tf.regs.a1 = pt.a1;
        tf.regs.a2 = pt.a2;
        tf.regs.a3 = pt.a3;
        tf.regs.a4 = pt.a4;
        tf.regs.a5 = pt.a5;
        tf.regs.a6 = pt.a6;
        tf.regs.a7 = pt.a7;
        tf.regs.s2 = pt.s2;
        tf.regs.s3 = pt.s3;
        tf.regs.s4 = pt.s4;
        tf.regs.s5 = pt.s5;
        tf.regs.s6 = pt.s6;
        tf.regs.s7 = pt.s7;
        tf.regs.s8 = pt.s8;
        tf.regs.s9 = pt.s9;
        tf.regs.s10 = pt.s10;
        tf.regs.s11 = pt.s11;
        tf.regs.t3 = pt.t3;
        tf.regs.t4 = pt.t4;
        tf.regs.t5 = pt.t5;
        tf.regs.t6 = pt.t6;
    }
    #[cfg(target_arch = "aarch64")]
    {
        tf.x = pt.regs;
        tf.elr = pt.pc;
        tf.spsr = pt.pstate;
    }
}

pub fn handle_breakpoint(tf: &mut ax_hal::context::TrapFrame) -> bool {
    let mut pt_regs = trapframe_to_ptregs(tf);
    let handled = with_manager(|manager| kprobe::kprobe_handler_from_break(manager, &mut pt_regs));
    if handled.is_some() {
        ptregs_write_back(&pt_regs, tf);
        return true;
    }
    false
}

#[cfg(target_arch = "x86_64")]
pub fn handle_debug(tf: &mut ax_hal::context::TrapFrame) -> bool {
    let mut pt_regs = trapframe_to_ptregs(tf);
    let handled = with_manager(|manager| kprobe::kprobe_handler_from_debug(manager, &mut pt_regs));
    if handled.is_some() {
        ptregs_write_back(&pt_regs, tf);
        return true;
    }
    false
}

#[allow(dead_code)]
fn flush_tlb_range(start: VirtAddr, size: usize) {
    for offset in (0..size).step_by(PAGE_SIZE_4K) {
        ax_hal::asm::flush_tlb(Some(start + offset));
    }
}
