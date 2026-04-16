#![cfg_attr(feature = "ax-std", no_std)]
#![cfg_attr(feature = "ax-std", no_main)]

#[cfg(feature = "ax-std")]
#[macro_use]
extern crate ax_std as std;

use std::{os::arceos::modules::ax_hal, println};

use ax_hal::trap::page_fault_handler;
use ax_memory_addr::VirtAddr;
use axvisor_guestlib::{
    emit_json_result, power_off_or_hang,
    sysreg::{read_sctlr_el1, write_sctlr_el1},
    user_probe::{
        build_user_aspace, code_blob, install_user_aspace, read_guest_bytes, restore_user_aspace,
    },
};

const CASE_ID: &str = "cpu.aarch64.sctlr.uct";
const SCTLR_UCT_BIT: u64 = 1 << 15;
const USER_CODE_START: usize = 0x4_000;
const USER_SHARED_START: usize = 0x8_000;
const USER_STACK_START: usize = 0x10_000;

#[page_fault_handler]
fn handle_page_fault(_vaddr: VirtAddr, _access_flags: ax_hal::paging::MappingFlags) -> bool {
    false
}

core::arch::global_asm!(
    r#"
    .section .text.axdiff_sctlr_uct_user, "ax"
    .p2align 2
    .global axdiff_sctlr_uct_user_entry
    .global axdiff_sctlr_uct_user_entry_end
axdiff_sctlr_uct_user_entry:
    mov x9, #1
    str x9, [x0, #8]
    // CTR_EL0 is EL0-readable only when SCTLR_EL1.UCT permits it.
    mrs x1, CTR_EL0
    str x1, [x0]
    mov x9, #2
    str x9, [x0, #8]
    mov x8, #0
    svc #0
1:
    b 1b
axdiff_sctlr_uct_user_entry_end:
"#
);

unsafe extern "C" {
    fn axdiff_sctlr_uct_user_entry();
    fn axdiff_sctlr_uct_user_entry_end();
}

#[derive(Clone, Copy)]
struct ProbeOutcome {
    trapped: bool,
    phase: u64,
    ctr_el0_raw: u64,
}

fn emit_error(stage: &str, detail: &str) -> ! {
    emit_json_result(
        CASE_ID,
        "error",
        &format!("{{\"stage\":\"{}\",\"detail\":\"{}\"}}", stage, detail),
    );
    power_off_or_hang();
}

fn prepare_user_aspace() -> Result<(ax_mm::AddrSpace, VirtAddr, VirtAddr), &'static str> {
    let code_start = axdiff_sctlr_uct_user_entry as *const ();
    let code_end = axdiff_sctlr_uct_user_entry_end as *const ();
    let code_bytes = unsafe { code_blob(code_start, code_end) };
    // Shared page layout:
    //   [0..8]  = raw CTR_EL0 value if the read succeeds
    //   [8..16] = phase marker
    let prepared = build_user_aspace(
        code_bytes,
        VirtAddr::from(USER_CODE_START),
        VirtAddr::from(USER_SHARED_START),
        &[0; 16],
        VirtAddr::from(USER_STACK_START),
        &[],
    )?;
    Ok((prepared.aspace, prepared.stack_top, prepared.shared_start))
}

fn read_shared(
    aspace: &ax_mm::AddrSpace,
    shared_start: VirtAddr,
) -> Result<(u64, u64), &'static str> {
    let mut shared = [0u8; 16];
    read_guest_bytes(aspace, shared_start, &mut shared, "read_shared_page")?;
    Ok((
        u64::from_le_bytes(shared[0..8].try_into().unwrap()),
        u64::from_le_bytes(shared[8..16].try_into().unwrap()),
    ))
}

fn run_probe(enable_uct: bool) -> Result<ProbeOutcome, &'static str> {
    let (aspace, stack_top, shared_start) = prepare_user_aspace()?;
    let original = read_sctlr_el1();
    let configured = if enable_uct {
        original | SCTLR_UCT_BIT
    } else {
        original & !SCTLR_UCT_BIT
    };
    write_sctlr_el1(configured);

    // The return reason tells whether EL0 trapped; the shared page tells
    // whether the payload reached and completed the intended read path.
    let old_ttbr0 = install_user_aspace(&aspace);

    let mut uctx =
        ax_hal::uspace::UserContext::new(USER_CODE_START, stack_top, shared_start.as_usize());
    let reason = uctx.run();

    restore_user_aspace(old_ttbr0);
    write_sctlr_el1(original);

    let (ctr_el0_raw, phase) = read_shared(&aspace, shared_start)?;
    Ok(ProbeOutcome {
        trapped: matches!(reason, ax_hal::uspace::ReturnReason::Exception(_)),
        phase,
        ctr_el0_raw,
    })
}

#[cfg_attr(feature = "ax-std", unsafe(no_mangle))]
fn main() -> ! {
    println!("Running {}", CASE_ID);

    let trap_when_clear = match run_probe(false) {
        Ok(v) => v,
        Err(stage) => emit_error(stage, "clear_uct_probe_failed"),
    };
    let allow_when_set = match run_probe(true) {
        Ok(v) => v,
        Err(stage) => emit_error(stage, "set_uct_probe_failed"),
    };

    // Only the enabled subcase needs the raw CTR_EL0 payload; when disabled we
    // care only that the access traps before phase 2 is reached.
    emit_json_result(
        CASE_ID,
        "ok",
        &format!(
            r#"{{"trap_when_clear":{{"trapped":{},"phase":{}}},"allow_when_set":{{"trapped":{},"phase":{},"ctr_el0_nonzero":{}}}}}"#,
            trap_when_clear.trapped,
            trap_when_clear.phase,
            allow_when_set.trapped,
            allow_when_set.phase,
            allow_when_set.ctr_el0_raw != 0,
        ),
    );

    power_off_or_hang();
}
