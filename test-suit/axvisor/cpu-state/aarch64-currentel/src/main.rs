#![cfg_attr(feature = "ax-std", no_std)]
#![cfg_attr(feature = "ax-std", no_main)]

#[cfg(feature = "ax-std")]
#[macro_use]
extern crate ax_std as std;

use std::{os::arceos::modules::ax_hal, println, string::String};

use ax_hal::trap::page_fault_handler;
use ax_memory_addr::VirtAddr;
use axvisor_guestlib::{
    emit_json_result, power_off_or_hang,
    user_probe::{
        build_user_aspace, code_blob, install_user_aspace, read_guest_bytes, restore_user_aspace,
    },
};

const CASE_ID: &str = "cpu.aarch64.currentel";
const USER_CODE_START: usize = 0x4_000;
const USER_SHARED_START: usize = 0x8_000;
const USER_STACK_START: usize = 0x10_000;

#[page_fault_handler]
fn handle_page_fault(_vaddr: VirtAddr, _access_flags: ax_hal::paging::MappingFlags) -> bool {
    false
}

core::arch::global_asm!(
    r#"
    .section .text.axdiff_currentel_user, "ax"
    .p2align 2
    .global axdiff_currentel_user_entry
    .global axdiff_currentel_user_after_fault
    .global axdiff_currentel_user_entry_end
axdiff_currentel_user_entry:
    mov x1, #1
    str x1, [x0, #8]
    // CurrentEL is not accessible from EL0 on AArch64. The first run is
    // expected to trap here so the guest can record how the hypervisor and
    // guest kernel report the exception back to EL1.
    mrs x1, CurrentEL
    mov x1, #0xff
    str x1, [x0]
axdiff_currentel_user_after_fault:
    mov x1, #2
    str x1, [x0, #8]
    // The resumed path exits via SVC so the case can distinguish "resumed
    // correctly after the fault" from "returned for some other reason".
    mov x8, #0
    svc #0
1:
    b 1b
axdiff_currentel_user_entry_end:
"#
);

unsafe extern "C" {
    fn axdiff_currentel_user_entry();
    fn axdiff_currentel_user_after_fault();
    fn axdiff_currentel_user_entry_end();
}

#[cfg(target_arch = "aarch64")]
fn read_current_el() -> u64 {
    use core::arch::asm;

    let raw: u64;
    unsafe {
        asm!("mrs {value}, CurrentEL", value = out(reg) raw);
    }
    raw
}

#[cfg(not(target_arch = "aarch64"))]
fn read_current_el() -> u64 {
    0
}

fn decoded_el(raw: u64) -> u64 {
    (raw >> 2) & 0b11
}

fn prepare_user_aspace() -> Result<(ax_mm::AddrSpace, VirtAddr, VirtAddr), &'static str> {
    let code_start = axdiff_currentel_user_entry as *const ();
    let code_end = axdiff_currentel_user_entry_end as *const ();
    let code_bytes = unsafe { code_blob(code_start, code_end) };
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

fn read_shared_words(
    aspace: &ax_mm::AddrSpace,
    shared_start: VirtAddr,
) -> Result<[u64; 2], &'static str> {
    let mut shared = [0u8; 16];
    read_guest_bytes(aspace, shared_start, &mut shared, "read_shared_page")?;
    Ok([
        u64::from_le_bytes(shared[0..8].try_into().unwrap()),
        u64::from_le_bytes(shared[8..16].try_into().unwrap()),
    ])
}

fn format_return_reason(reason: ax_hal::uspace::ReturnReason) -> String {
    match reason {
        ax_hal::uspace::ReturnReason::Syscall => String::from("\"syscall\""),
        ax_hal::uspace::ReturnReason::Interrupt => String::from("\"interrupt\""),
        ax_hal::uspace::ReturnReason::Unknown => String::from("\"unknown\""),
        ax_hal::uspace::ReturnReason::PageFault(addr, flags) => format!(
            "{{\"kind\":\"page_fault\",\"addr\":{},\"flags\":\"{:?}\"}}",
            addr.as_usize(),
            flags
        ),
        ax_hal::uspace::ReturnReason::Exception(exc) => {
            #[cfg(target_arch = "aarch64")]
            {
                format!(
                    "{{\"kind\":\"exception\",\"exception_kind\":\"{:?}\",\"esr\":{},\"far\":{}}}",
                    exc.kind(),
                    exc.esr.get(),
                    exc.far
                )
            }
            #[cfg(not(target_arch = "aarch64"))]
            {
                format!(
                    "{{\"kind\":\"exception\",\"exception_kind\":\"{:?}\"}}",
                    exc.kind()
                )
            }
        }
    }
}

fn emit_error(stage: &str, detail: &str) -> ! {
    emit_json_result(
        CASE_ID,
        "error",
        &format!("{{\"stage\":\"{}\",\"detail\":\"{}\"}}", stage, detail),
    );
    power_off_or_hang();
}

#[cfg_attr(feature = "ax-std", unsafe(no_mangle))]
fn main() -> ! {
    println!("Running {}", CASE_ID);

    let el1_before = read_current_el();
    let (user_aspace, user_stack_top, user_shared_start) = match prepare_user_aspace() {
        Ok(v) => v,
        Err(stage) => emit_error(stage, "failed"),
    };
    let user_entry = axdiff_currentel_user_entry as *const () as usize;
    let user_after_fault_label = axdiff_currentel_user_after_fault as *const () as usize;
    let user_after_fault = USER_CODE_START + user_after_fault_label.saturating_sub(user_entry);

    // Install the case-local user page table so UserContext::run enters the
    // address space we just built rather than whatever the kernel used before.
    let old_ttbr0 = install_user_aspace(&user_aspace);

    let mut uctx = ax_hal::uspace::UserContext::new(
        USER_CODE_START,
        user_stack_top,
        user_shared_start.as_usize(),
    );

    // First entry to EL0 should fault on the CurrentEL read. The shared page
    // phase marker lets us distinguish "fault happened at the intended site"
    // from "user payload never ran".
    let first_reason = uctx.run();
    let [el0_raw_after_exception, phase_after_exception] =
        match read_shared_words(&user_aspace, user_shared_start) {
            Ok(words) => words,
            Err(stage) => emit_error(stage, "failed"),
        };
    if phase_after_exception != 1 {
        emit_error("enter_el0", "user code did not reach the EL0 probe marker");
    }
    if !matches!(first_reason, ax_hal::uspace::ReturnReason::Exception(_)) {
        emit_error(
            "currentel_el0_access",
            "expected EL0 CurrentEL access to trap as an exception",
        );
    }

    // Resume at the instruction after the faulting MRS. This checks that the
    // guest can recover from the exception path and continue executing at EL0.
    uctx.set_ip(user_after_fault);
    let second_reason = uctx.run();
    let [el0_raw_after_syscall, phase_after_syscall] =
        match read_shared_words(&user_aspace, user_shared_start) {
            Ok(words) => words,
            Err(stage) => emit_error(stage, "failed"),
        };
    if phase_after_syscall != 2 {
        emit_error(
            "resume_after_fault",
            "user code did not resume to the post-fault path",
        );
    }
    if !matches!(second_reason, ax_hal::uspace::ReturnReason::Syscall) {
        emit_error(
            "resume_after_fault",
            "expected resumed EL0 path to exit via SVC",
        );
    }

    restore_user_aspace(old_ttbr0);

    let el1_after = read_current_el();

    // Keep both return reasons and the before/after EL1 observations so the
    // baseline can be compared against the DUT at the semantic level.
    emit_json_result(
        CASE_ID,
        "ok",
        &format!(
            "{{\"el1_before\":{{\"raw\":{},\"decoded_el\":{}}},\"el0_probe\":{{\"\
             raw_after_exception\":{},\"phase_after_exception\":{},\"raw_after_resume\":{},\"\
             phase_after_resume\":{}}},\"first_return_reason\":{},\"second_return_reason\":{},\"\
             el1_after\":{{\"raw\":{},\"decoded_el\":{}}}}}",
            el1_before,
            decoded_el(el1_before),
            el0_raw_after_exception,
            phase_after_exception,
            el0_raw_after_syscall,
            phase_after_syscall,
            format_return_reason(first_reason),
            format_return_reason(second_reason),
            el1_after,
            decoded_el(el1_after),
        ),
    );

    power_off_or_hang();
}
