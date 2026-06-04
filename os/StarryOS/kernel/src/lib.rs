//! The core functionality of a monolithic kernel, including loading user
//! programs and managing processes.

#![no_std]
#![feature(likely_unlikely)]
#![feature(c_variadic)]
#![allow(missing_docs)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate alloc;
extern crate ax_runtime;

#[macro_use]
extern crate ax_log;

#[macro_use]
pub mod dyn_debug; // Re-export debug macros for use in other modules. It will override the `debug` macro from `log` crate when `dynamic_debug` feature is enabled.

pub mod entry;

mod cgroup;
mod config;
pub mod ebpf;
pub mod file;
mod kmod;
mod kprobe;
pub mod mm;
pub mod perf;
mod pseudofs;
mod stop_machine;
mod syscall;

// The syscall registration interface is part of the kernel's module-facing
// surface: a loadable/built-in module binds to these symbols to take over an
// individual syscall (see `syscall::registry`).
pub use syscall::{
    SyscallHandler, lookup_syscall_handler, register_syscall_handler, unregister_syscall_handler,
};

mod task;
mod time;
mod tracepoint;
mod trap;
