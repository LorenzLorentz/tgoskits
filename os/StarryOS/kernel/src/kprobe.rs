use alloc::alloc::{Layout, alloc, dealloc};

use ax_memory_addr::{MemoryAddr, PAGE_SIZE_4K, VirtAddr};
use kprobe::KprobeAuxiliaryOps;

use crate::task::AsThread;

#[derive(Debug)]
#[allow(dead_code)]
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

#[allow(dead_code)]
fn flush_tlb_range(start: VirtAddr, size: usize) {
    for offset in (0..size).step_by(PAGE_SIZE_4K) {
        ax_hal::asm::flush_tlb(Some(start + offset));
    }
}
