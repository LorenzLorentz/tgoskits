#![no_std]
#![no_main]
#![doc = include_str!("../../README.md")]

extern crate alloc;

use alloc::{borrow::ToOwned, vec::Vec};

pub const CMDLINE: &[&str] = &["/bin/sh", "-c", include_str!("init.sh")];

#[unsafe(no_mangle)]
fn main() {
    let args = CMDLINE
        .iter()
        .copied()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let envs = [];

    // When the `kebpf` module is selected as a built-in, install it as the
    // `bpf(2)` handler via the kernel's syscall registration interface before
    // the init process can issue any syscall. This is the in-tree counterpart
    // to loading `kebpf.ko`; in both forms the kernel falls back to its
    // built-in eBPF runtime whenever no module handler is registered.
    #[cfg(feature = "kebpf")]
    let _ = kebpf::kebpf_init();

    starry_kernel::entry::init(&args, &envs);
}
