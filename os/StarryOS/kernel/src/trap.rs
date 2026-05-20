#[ax_hal::trap::breakpoint_handler]
fn default_breakpoint_handler(_tf: &mut ax_hal::context::TrapFrame) -> bool {
    // TODO: integrate with kprobe::kprobe_handler_from_break once
    // TrapFrame <-> PtRegs conversion is implemented.
    false
}

#[cfg(target_arch = "x86_64")]
#[ax_hal::trap::debug_handler]
fn default_debug_handler(_tf: &mut ax_hal::context::TrapFrame) -> bool {
    // TODO: integrate with kprobe::kprobe_handler_from_debug once
    // TrapFrame <-> PtRegs conversion is implemented.
    false
}
