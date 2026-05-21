#![no_std]
#![no_main]
#![doc = include_str!("../../README.md")]

extern crate alloc;

use alloc::{borrow::ToOwned, vec::Vec};

pub const CMDLINE: &[&str] = &["/bin/sh", "-c", include_str!("init.sh")];

mod kallsyms_data {
    include!(concat!(env!("OUT_DIR"), "/kallsyms_data.rs"));
}

#[unsafe(no_mangle)]
fn main() {
    let args = CMDLINE
        .iter()
        .copied()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let envs = [];

    starry_kernel::entry::init_with_kallsyms(&args, &envs, kallsyms_data::KALLSYMS_DATA);
}

#[cfg(all(
    feature = "sg2002",
    any(target_arch = "riscv32", target_arch = "riscv64")
))]
extern crate ax_plat_riscv64_sg2002;
