use core::arch::asm;

pub fn read_sctlr_el1() -> u64 {
    let raw: u64;
    unsafe {
        asm!("mrs {value}, SCTLR_EL1", value = out(reg) raw);
    }
    raw
}

pub fn write_sctlr_el1(value: u64) {
    // ISB makes subsequent instruction fetch and privilege checks observe the
    // new SCTLR_EL1 value before the caller re-enters EL0.
    unsafe {
        asm!("msr SCTLR_EL1, {value}", value = in(reg) value);
        asm!("isb");
    }
}
