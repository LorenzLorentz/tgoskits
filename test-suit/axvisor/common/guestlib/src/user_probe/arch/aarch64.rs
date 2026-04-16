use core::arch::asm;

use ax_memory_addr::{PAGE_SIZE_4K, PhysAddr, VirtAddr};
use ax_mm::new_user_aspace;
use ax_std::os::arceos::modules::ax_hal;

use super::{PreparedUserAspace, UserPageInit};

pub fn sync_executable_range(start: VirtAddr, size: usize) {
    // The EL0 payload is copied into memory at runtime, so we must push data
    // cache to the point of unification and invalidate matching I-cache lines
    // before executing from that range.
    let mut addr = start.as_usize() & !63usize;
    let end = (start.as_usize() + size + 63) & !63usize;

    while addr < end {
        unsafe {
            asm!("dc cvau, {addr}", addr = in(reg) addr);
        }
        addr += 64;
    }
    unsafe {
        asm!("dsb ish");
    }

    let mut addr = start.as_usize() & !63usize;
    while addr < end {
        unsafe {
            asm!("ic ivau, {addr}", addr = in(reg) addr);
        }
        addr += 64;
    }
    unsafe {
        asm!("dsb ish");
        asm!("isb");
    }
}

pub unsafe fn code_blob(start: *const (), end: *const ()) -> &'static [u8] {
    unsafe {
        core::slice::from_raw_parts(
            start.cast::<u8>(),
            (end as usize).saturating_sub(start as usize),
        )
    }
}

pub fn build_user_aspace(
    code_bytes: &[u8],
    user_code_start: VirtAddr,
    user_shared_start: VirtAddr,
    shared_init: &[u8],
    user_stack_start: VirtAddr,
    extra_pages: &[UserPageInit<'_>],
) -> Result<PreparedUserAspace, &'static str> {
    // Build the smallest user address space that can host one in-memory EL0
    // probe plus the shared pages it uses to communicate results back to EL1.
    let mut end = (user_stack_start + PAGE_SIZE_4K).as_usize();
    end = end.max((user_shared_start + PAGE_SIZE_4K).as_usize());
    end = end.max((user_code_start + PAGE_SIZE_4K).as_usize());
    for page in extra_pages {
        end = end.max((page.start + PAGE_SIZE_4K).as_usize());
    }

    let mut aspace = new_user_aspace(user_code_start, end - user_code_start.as_usize())
        .map_err(|_| "create_user_aspace")?;

    let code_rw_flags = ax_hal::paging::MappingFlags::READ
        | ax_hal::paging::MappingFlags::WRITE
        | ax_hal::paging::MappingFlags::EXECUTE
        | ax_hal::paging::MappingFlags::USER;
    let code_rx_flags = ax_hal::paging::MappingFlags::READ
        | ax_hal::paging::MappingFlags::EXECUTE
        | ax_hal::paging::MappingFlags::USER;
    let data_flags = ax_hal::paging::MappingFlags::READ
        | ax_hal::paging::MappingFlags::WRITE
        | ax_hal::paging::MappingFlags::USER;

    aspace
        .map_alloc(user_code_start, PAGE_SIZE_4K, code_rw_flags, true)
        .map_err(|_| "map_user_code")?;
    aspace
        .map_alloc(user_shared_start, PAGE_SIZE_4K, data_flags, true)
        .map_err(|_| "map_shared_page")?;
    aspace
        .map_alloc(user_stack_start, PAGE_SIZE_4K, data_flags, true)
        .map_err(|_| "map_user_stack")?;

    for page in extra_pages {
        aspace
            .map_alloc(page.start, PAGE_SIZE_4K, data_flags, true)
            .map_err(|_| "map_extra_page")?;
    }

    aspace
        .write(user_code_start, code_bytes)
        .map_err(|_| "copy_user_code")?;
    aspace
        .write(user_shared_start, shared_init)
        .map_err(|_| "init_shared_page")?;
    for page in extra_pages {
        aspace
            .write(page.start, page.bytes)
            .map_err(|_| "init_extra_page")?;
    }
    aspace
        .protect(user_code_start, PAGE_SIZE_4K, code_rx_flags)
        .map_err(|_| "protect_user_code")?;
    sync_executable_range(user_code_start, code_bytes.len());

    Ok(PreparedUserAspace {
        aspace,
        stack_top: user_stack_start + PAGE_SIZE_4K,
        shared_start: user_shared_start,
    })
}

pub fn install_user_aspace(aspace: &ax_mm::AddrSpace) -> PhysAddr {
    // Swap in the case-local user page table and return the previous TTBR0 so
    // the caller can always restore the host/kernel user mapping afterwards.
    let old_ttbr0 = ax_hal::asm::read_user_page_table();
    unsafe {
        ax_hal::asm::write_user_page_table(aspace.page_table_root());
    }
    ax_hal::asm::flush_tlb(None);
    old_ttbr0
}

pub fn restore_user_aspace(old_ttbr0: PhysAddr) {
    unsafe {
        ax_hal::asm::write_user_page_table(old_ttbr0);
    }
    ax_hal::asm::flush_tlb(None);
}

pub fn read_guest_bytes(
    aspace: &ax_mm::AddrSpace,
    start: VirtAddr,
    buf: &mut [u8],
    stage: &'static str,
) -> Result<(), &'static str> {
    // Cases keep their own shared-page schemas; this helper only normalizes the
    // low-level "read from guest aspace or return a stage label" pattern.
    aspace.read(start, buf).map_err(|_| stage)
}
