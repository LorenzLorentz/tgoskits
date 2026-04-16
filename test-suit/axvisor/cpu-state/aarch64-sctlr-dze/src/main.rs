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
        PreparedUserAspace, UserPageInit, build_user_aspace, code_blob, install_user_aspace,
        read_guest_bytes, restore_user_aspace,
    },
};

const CASE_ID: &str = "cpu.aarch64.sctlr.dze";
const SCTLR_DZE_BIT: u64 = 1 << 14;
const USER_CODE_START: usize = 0x4_000;
const USER_SHARED_START: usize = 0x8_000;
const USER_BUFFER_START: usize = 0xC_000;
const USER_STACK_START: usize = 0x10_000;
const BUFFER_SIZE: usize = 64;
const BUFFER_PATTERN: u8 = 0xA5;

#[page_fault_handler]
fn handle_page_fault(_vaddr: VirtAddr, _access_flags: ax_hal::paging::MappingFlags) -> bool {
    false
}

core::arch::global_asm!(
    r#"
    .section .text.axdiff_sctlr_dze_user, "ax"
    .p2align 2
    .global axdiff_sctlr_dze_user_entry
    .global axdiff_sctlr_dze_user_entry_end
axdiff_sctlr_dze_user_entry:
    mov x9, #1
    str x9, [x0, #8]
    // Record DCZID_EL0 first so EL1 can tell whether the architecture reports
    // zeroing as prohibited (DZP=1) when SCTLR_EL1.DZE is cleared.
    mrs x1, DCZID_EL0
    str x1, [x0]
    ldr x2, [x0, #16]
    dc zva, x2
    dsb ish
    mov x9, #2
    str x9, [x0, #8]
    mov x8, #0
    svc #0
1:
    b 1b
axdiff_sctlr_dze_user_entry_end:
"#
);

unsafe extern "C" {
    fn axdiff_sctlr_dze_user_entry();
    fn axdiff_sctlr_dze_user_entry_end();
}

#[derive(Clone, Copy)]
struct ProbeOutcome {
    trapped: bool,
    phase: u64,
    dzp: bool,
    buffer_all_zero: bool,
    buffer_unchanged: bool,
}

fn emit_error(stage: &str, detail: &str) -> ! {
    emit_json_result(
        CASE_ID,
        "error",
        &format!("{{\"stage\":\"{}\",\"detail\":\"{}\"}}", stage, detail),
    );
    power_off_or_hang();
}

fn prepare_probe_aspace() -> Result<PreparedUserAspace, &'static str> {
    let code_start = axdiff_sctlr_dze_user_entry as *const ();
    let code_end = axdiff_sctlr_dze_user_entry_end as *const ();
    let code_bytes = unsafe { code_blob(code_start, code_end) };
    // Shared page layout:
    //   [0..8]   = raw DCZID_EL0 value observed by EL0
    //   [8..16]  = phase marker
    //   [16..24] = probe buffer virtual address for DC ZVA
    let mut shared = [0u8; 24];
    shared[16..24].copy_from_slice(&(USER_BUFFER_START as u64).to_le_bytes());
    let buffer = [BUFFER_PATTERN; BUFFER_SIZE];
    build_user_aspace(
        code_bytes,
        VirtAddr::from(USER_CODE_START),
        VirtAddr::from(USER_SHARED_START),
        &shared,
        VirtAddr::from(USER_STACK_START),
        &[UserPageInit {
            start: VirtAddr::from(USER_BUFFER_START),
            bytes: &buffer,
        }],
    )
}

fn read_shared(
    aspace: &ax_mm::AddrSpace,
    shared_start: VirtAddr,
) -> Result<(u64, u64), &'static str> {
    let mut shared = [0u8; 24];
    read_guest_bytes(aspace, shared_start, &mut shared, "read_shared_page")?;
    Ok((
        u64::from_le_bytes(shared[0..8].try_into().unwrap()),
        u64::from_le_bytes(shared[8..16].try_into().unwrap()),
    ))
}

fn read_buffer(
    aspace: &ax_mm::AddrSpace,
    buffer_start: VirtAddr,
) -> Result<[u8; BUFFER_SIZE], &'static str> {
    let mut buffer = [0u8; BUFFER_SIZE];
    read_guest_bytes(aspace, buffer_start, &mut buffer, "read_probe_buffer")?;
    Ok(buffer)
}

fn run_probe(enable_dze: bool) -> Result<ProbeOutcome, &'static str> {
    let prepared = prepare_probe_aspace()?;
    let original = read_sctlr_el1();
    let configured = if enable_dze {
        original | SCTLR_DZE_BIT
    } else {
        original & !SCTLR_DZE_BIT
    };
    write_sctlr_el1(configured);

    // Enter EL0 once under the requested SCTLR_EL1.DZE state, then inspect the
    // shared metadata and probe buffer to tell "trapped" from "zeroed data".
    let old_ttbr0 = install_user_aspace(&prepared.aspace);
    let mut uctx = ax_hal::uspace::UserContext::new(
        USER_CODE_START,
        prepared.stack_top,
        prepared.shared_start.as_usize(),
    );
    let reason = uctx.run();

    restore_user_aspace(old_ttbr0);
    write_sctlr_el1(original);

    let (dczid_raw, phase) = read_shared(&prepared.aspace, prepared.shared_start)?;
    let buffer = read_buffer(&prepared.aspace, VirtAddr::from(USER_BUFFER_START))?;

    Ok(ProbeOutcome {
        trapped: matches!(reason, ax_hal::uspace::ReturnReason::Exception(_)),
        phase,
        dzp: ((dczid_raw >> 4) & 1) != 0,
        buffer_all_zero: buffer.iter().all(|byte| *byte == 0),
        buffer_unchanged: buffer.iter().all(|byte| *byte == BUFFER_PATTERN),
    })
}

#[cfg_attr(feature = "ax-std", unsafe(no_mangle))]
fn main() -> ! {
    println!("Running {}", CASE_ID);

    let trap_when_clear = match run_probe(false) {
        Ok(v) => v,
        Err(stage) => emit_error(stage, "clear_dze_probe_failed"),
    };
    let allow_when_set = match run_probe(true) {
        Ok(v) => v,
        Err(stage) => emit_error(stage, "set_dze_probe_failed"),
    };

    // Strong diff compares the semantic outcome under both control-bit states:
    // trap/phase reporting plus whether the target buffer was actually zeroed.
    emit_json_result(
        CASE_ID,
        "ok",
        &format!(
            r#"{{"trap_when_clear":{{"trapped":{},"phase":{},"dzp":{},"buffer_unchanged":{}}},"allow_when_set":{{"trapped":{},"phase":{},"dzp":{},"buffer_all_zero":{}}}}}"#,
            trap_when_clear.trapped,
            trap_when_clear.phase,
            trap_when_clear.dzp,
            trap_when_clear.buffer_unchanged,
            allow_when_set.trapped,
            allow_when_set.phase,
            allow_when_set.dzp,
            allow_when_set.buffer_all_zero,
        ),
    );

    power_off_or_hang();
}
