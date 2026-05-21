#[ax_hal::trap::breakpoint_handler]
fn default_breakpoint_handler(tf: &mut ax_hal::context::TrapFrame) -> bool {
    crate::kprobe::handle_breakpoint(tf)
}

#[cfg(target_arch = "x86_64")]
#[ax_hal::trap::debug_handler]
fn default_debug_handler(tf: &mut ax_hal::context::TrapFrame) -> bool {
    crate::kprobe::handle_debug(tf)
}
