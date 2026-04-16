#[cfg(target_arch = "aarch64")]
#[path = "arch/aarch64.rs"]
mod arch;

#[cfg(target_arch = "aarch64")]
pub use arch::*;
