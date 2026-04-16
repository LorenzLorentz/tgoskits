use ax_memory_addr::VirtAddr;
use ax_mm::AddrSpace;

pub struct UserPageInit<'a> {
    pub start: VirtAddr,
    pub bytes: &'a [u8],
}

pub struct PreparedUserAspace {
    pub aspace: AddrSpace,
    pub stack_top: VirtAddr,
    pub shared_start: VirtAddr,
}

#[cfg(target_arch = "aarch64")]
#[path = "arch/aarch64.rs"]
mod arch;

#[cfg(target_arch = "aarch64")]
pub use arch::*;
