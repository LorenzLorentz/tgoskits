//! Minimal "hello world" loadable kernel module. Ported from
//! `Starry-OS/StarryOS:ebpf-kmod` (`modules/hello/src/lib.rs`); besides
//! the `axfeat` → `ax-feat` workspace-package rename, the original's
//! `vec![..]` + `{:?}` print is replaced with a stack array formatted via
//! `Display` (see below).
//!
//! Loaded via `init_module(2)` / `finit_module(2)` (see
//! `os/StarryOS/kernel/src/syscall/kmod.rs`). Every symbol a module
//! references is resolved at relocation time by `KmodHelper::resolve_symbol`
//! against the in-kernel `.kallsyms` table, so a module may only use kernel
//! symbols that survive into the final kernel image. That rules out:
//!   * `{:?}` Debug formatting — the `core::fmt` Debug builders are inlined
//!     away in the kernel and are not standalone kallsyms entries;
//!   * heap allocation via the global allocator — it pulls in the
//!     `__rust_no_alloc_shim_is_unstable_v2` link-time marker, which never
//!     exists as a runtime symbol and so can never be in kallsyms.
//! Hence the stack array + `Display` (`{}`) below, which only needs
//! `core::fmt::write` and `<i32 as Display>::fmt` — both retained.

#![no_std]

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
    let _ = core::fmt::write(&mut writer, format_args!("Hello, Kernel Module!\n"));
    // Stack array (no heap) formatted element-by-element with `Display` to
    // reproduce the original `[1, 2, 3, 4, 5]` output without `{:?}`/`vec!`.
    let v = [1, 2, 3, 4, 5];
    let _ = core::fmt::write(&mut writer, format_args!("Vector contents: ["));
    for (i, n) in v.iter().enumerate() {
        if i > 0 {
            let _ = core::fmt::write(&mut writer, format_args!(", "));
        }
        let _ = core::fmt::write(&mut writer, format_args!("{n}"));
    }
    let _ = core::fmt::write(&mut writer, format_args!("]\n"));
    0
}

#[exit_fn]
fn hello_exit() {
    let mut writer = Writer;
    let _ = core::fmt::write(&mut writer, format_args!("Goodbye, Kernel Module!\n"));
}

module!(
    name: "hello",
    license: "GPL",
    description: "A simple hello world kernel module",
    version: "0.1.0",
);
