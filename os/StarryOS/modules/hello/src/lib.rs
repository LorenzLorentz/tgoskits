//! Minimal "hello world" loadable kernel module. Ported from
//! `Starry-OS/StarryOS:ebpf-kmod` (`modules/hello/src/lib.rs`); only
//! the `axfeat` → `ax-feat` workspace-package rename was needed (the
//! `kmod-tools` crate name and the `write_char` C-ABI shim symbol
//! exported by PR-B's `kmod::kprint` are already aligned).
//!
//! Loaded via `init_module(2)` / `finit_module(2)` (see
//! `os/StarryOS/kernel/src/syscall/kmod.rs`). The `write_char` symbol
//! is resolved at relocation time by `KmodHelper::resolve_symbol`
//! against the global kallsyms table populated by PR #805.

#![no_std]
extern crate alloc;

use alloc::vec;

use kmod_tools::{exit_fn, init_fn, module};

unsafe extern "C" {
    fn write_char(c: u8);
}

struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for &b in s.as_bytes() {
            unsafe { write_char(b) };
        }
        Ok(())
    }
}

#[init_fn]
pub fn hello_init() -> i32 {
    let mut writer = Writer;
    core::fmt::write(&mut writer, format_args!("Hello, Kernel Module!\n")).unwrap();
    let v = vec![1, 2, 3, 4, 5];
    core::fmt::write(&mut writer, format_args!("Vector contents: {:?}\n", v)).unwrap();
    0
}

#[exit_fn]
fn hello_exit() {
    let mut writer = Writer;
    core::fmt::write(&mut writer, format_args!("Goodbye, Kernel Module!\n")).unwrap();
}

module!(
    name: "hello",
    license: "GPL",
    description: "A simple hello world kernel module",
    version: "0.1.0",
);
