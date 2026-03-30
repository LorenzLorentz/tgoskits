// Copyright 2025 The Axvisor Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use axaddrspace::{GuestPhysAddr, GuestVirtAddr, HostPhysAddr, MappingFlags, device::AccessWidth};
use axerrno::{AxError, AxErrorKind, AxResult};
use axvcpu::AxVCpuExitReason;
use riscv::register::{scause, sie, sstatus};
use riscv_decode::{
    Instruction,
    types::{IType, SType},
};
use riscv_h::register::{
    hie, hip, hstatus, htimedelta, hvip,
    vsatp::{self, Vsatp},
    vscause::{self, Vscause},
    vsepc,
    vsie::{self, Vsie},
    vsip, vsscratch,
    vsstatus::{self, Vsstatus},
    vstval,
    vstvec::{self, Vstvec},
};
use rustsbi::{Forward, RustSBI};
use sbi_spec::{hsm, legacy, srst};

use crate::{
    EID_HVC, RISCVVCpuCreateConfig, consts::traps::irq::S_EXT, guest_mem, regs::*, sbi_console::*,
};

unsafe extern "C" {
    fn _run_guest(state: *mut VmCpuRegisters);
}

const TINST_PSEUDO_STORE: u32 = 0x3020;
const TINST_PSEUDO_LOAD: u32 = 0x3000;
const EID_TIME: usize = 0x5449_4D45;
const FID_SET_TIMER: usize = 0;

#[inline]
fn instr_is_pseudo(ins: u32) -> bool {
    ins == TINST_PSEUDO_STORE || ins == TINST_PSEUDO_LOAD
}

#[inline]
fn set_host_timer(deadline: u64) {
    unsafe {
        core::arch::asm!(
            "csrw {csr}, {value}",
            csr = const 0x14d,
            value = in(reg) deadline,
        );
    }
}

#[inline]
fn clear_host_timer_pending() {
    unsafe {
        core::arch::asm!(
            "csrc sip, {mask}",
            mask = in(reg) (1usize << 5),
        );
    }
}

#[inline]
fn set_guest_timer(deadline: u64) {
    unsafe {
        core::arch::asm!(
            "csrw {csr}, {value}",
            csr = const 0x24d,
            value = in(reg) deadline,
        );
    }
}

#[inline]
fn read_guest_timer() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "csrr {value}, {csr}",
            value = out(reg) value,
            csr = const 0x24d,
        );
    }
    value
}

/// The architecture dependent configuration of a `AxArchVCpu`.
#[derive(Clone, Copy, Debug, Default)]
pub struct VCpuConfig {}

#[derive(Default)]
/// A virtual CPU within a guest
pub struct RISCVVCpu {
    regs: VmCpuRegisters,
    sbi: RISCVVCpuSbi,
}

#[derive(RustSBI)]
struct RISCVVCpuSbi {
    #[rustsbi(console, pmu, fence, reset, info, hsm, timer)]
    forward: Forward,
}

impl Default for RISCVVCpuSbi {
    #[inline]
    fn default() -> Self {
        Self { forward: Forward }
    }
}

impl axvcpu::AxArchVCpu for RISCVVCpu {
    type CreateConfig = RISCVVCpuCreateConfig;

    type SetupConfig = ();

    fn new(_vm_id: usize, _vcpu_id: usize, config: Self::CreateConfig) -> AxResult<Self> {
        let mut regs = VmCpuRegisters::default();
        // Setup the guest's general purpose registers.
        // `a0` is the hartid
        regs.guest_regs.gprs.set_reg(GprIndex::A0, config.hart_id);
        // `a1` is the address of the device tree blob.
        regs.guest_regs.gprs.set_reg(GprIndex::A1, config.dtb_addr);

        Ok(Self {
            regs,
            sbi: RISCVVCpuSbi::default(),
        })
    }

    fn setup(&mut self, _config: Self::SetupConfig) -> AxResult {
        // Set sstatus.
        let mut sstatus = sstatus::read();
        sstatus.set_sie(false);
        sstatus.set_spie(false);
        sstatus.set_spp(sstatus::SPP::Supervisor);
        self.regs.guest_regs.sstatus = sstatus.bits();

        // Set hstatus.
        let mut hstatus = hstatus::read();
        hstatus.set_spv(true);
        hstatus.set_vsxl(hstatus::VsxlValues::Vsxl64);
        // Set SPVP bit in order to accessing VS-mode memory from HS-mode.
        hstatus.set_spvp(true);
        // Let the guest execute its normal supervisor instructions without
        // spuriously trapping them back to the hypervisor.
        hstatus.set_vtvm(false);
        hstatus.set_vtw(false);
        hstatus.set_vtsr(false);
        unsafe {
            hstatus.write();
        }
        self.regs.guest_regs.hstatus = hstatus.bits();

        let mut guest_hie = hie::Hie::from_bits(0);
        guest_hie.set_vssie(true);
        guest_hie.set_vstie(true);
        guest_hie.set_vseie(true);
        self.regs.virtual_hs_csrs.hie = guest_hie.bits();
        self.regs.vs_csrs.vstimecmp = u64::MAX as usize;
        Ok(())
    }

    fn set_entry(&mut self, entry: GuestPhysAddr) -> AxResult {
        self.regs.guest_regs.sepc = entry.as_usize();
        Ok(())
    }

    fn set_ept_root(&mut self, ept_root: HostPhysAddr) -> AxResult {
        // AxVM builds a 4-level guest stage-2 page table on RISC-V, so hgatp
        // must use Sv48x4 as well.
        self.regs.virtual_hs_csrs.hgatp = 9usize << 60 | usize::from(ept_root) >> 12;
        Ok(())
    }

    fn run(&mut self) -> AxResult<AxVCpuExitReason> {
        unsafe {
            // Keep guest instruction fetch coherent with any code patching done
            // during early Linux boot (alternatives/jump labels).
            core::arch::asm!("fence.i");
        }
        unsafe {
            sstatus::clear_sie();
            sie::set_sext();
            sie::set_ssoft();
            sie::set_stimer();
        }
        unsafe {
            // Safe to run the guest as it only touches memory assigned to it by being owned
            // by its page table
            _run_guest(&mut self.regs);
        }
        unsafe {
            sie::clear_sext();
            sie::clear_ssoft();
            sie::clear_stimer();
            sstatus::set_sie();
            core::arch::asm!("fence.i");
        }
        self.vmexit_handler()
    }

    fn bind(&mut self) -> AxResult {
        // Load the vCPU's CSRs from the stored state.
        unsafe {
            let vsatp = Vsatp::from_bits(self.regs.vs_csrs.vsatp);
            vsatp.write();
            let vstvec = Vstvec::from_bits(self.regs.vs_csrs.vstvec);
            vstvec.write();
            let vsepc = self.regs.vs_csrs.vsepc;
            vsepc::write(vsepc);
            let vstval = self.regs.vs_csrs.vstval;
            vstval::write(vstval);
            let htimedelta = self.regs.vs_csrs.htimedelta;
            htimedelta::write(htimedelta);
            let vscause = Vscause::from_bits(self.regs.vs_csrs.vscause);
            vscause.write();
            let vsscratch = self.regs.vs_csrs.vsscratch;
            vsscratch::write(vsscratch);
            let vsstatus = Vsstatus::from_bits(self.regs.vs_csrs.vsstatus);
            vsstatus.write();
            let vsie = Vsie::from_bits(self.regs.vs_csrs.vsie);
            vsie.write();
            set_guest_timer(self.regs.vs_csrs.vstimecmp as u64);
            let hie = hie::Hie::from_bits(self.regs.virtual_hs_csrs.hie);
            hie.write();
            core::arch::asm!(
                "csrw hgatp, {hgatp}",
                hgatp = in(reg) self.regs.virtual_hs_csrs.hgatp,
            );
            core::arch::riscv64::hfence_gvma_all();
        }
        Ok(())
    }

    fn unbind(&mut self) -> AxResult {
        // Store the vCPU's CSRs to the stored state.
        unsafe {
            self.regs.vs_csrs.vsatp = vsatp::read().bits();
            self.regs.vs_csrs.vstvec = vstvec::read().bits();
            self.regs.vs_csrs.vsepc = vsepc::read();
            self.regs.vs_csrs.vstval = vstval::read();
            self.regs.vs_csrs.htimedelta = htimedelta::read();
            self.regs.vs_csrs.vscause = vscause::read().bits();
            self.regs.vs_csrs.vsscratch = vsscratch::read();
            self.regs.vs_csrs.vsstatus = vsstatus::read().bits();
            self.regs.vs_csrs.vsie = vsie::read().bits();
            self.regs.vs_csrs.vstimecmp = read_guest_timer() as usize;
            self.regs.virtual_hs_csrs.hie = hie::read().bits();
            core::arch::asm!(
                "csrr {hgatp}, hgatp",
                hgatp = out(reg) self.regs.virtual_hs_csrs.hgatp,
            );
            core::arch::asm!("csrw hgatp, x0");
            core::arch::riscv64::hfence_gvma_all();
        }
        Ok(())
    }

    /// Set one of the vCPU's general purpose register.
    fn set_gpr(&mut self, index: usize, val: usize) {
        match index {
            0 => {
                // Do nothing, x0 is hardwired to zero
            }
            1..=31 => {
                if let Some(gpr_index) = GprIndex::from_raw(index as u32) {
                    self.set_gpr_from_gpr_index(gpr_index, val);
                } else {
                    warn!("RISCVVCpu: Failed to map general purpose register index: {index}");
                }
            }
            _ => {
                warn!("RISCVVCpu: Unsupported general purpose register index: {index}");
            }
        }
    }

    fn inject_interrupt(&mut self, _vector: usize) -> AxResult {
        unimplemented!("RISCVVCpu::inject_interrupt is not implemented yet");
    }

    fn set_return_value(&mut self, val: usize) {
        self.set_gpr_from_gpr_index(GprIndex::A0, val);
    }
}

impl RISCVVCpu {
    /// Gets one of the vCPU's general purpose registers.
    pub fn get_gpr(&self, index: GprIndex) -> usize {
        self.regs.guest_regs.gprs.reg(index)
    }

    /// Set one of the vCPU's general purpose register.
    pub fn set_gpr_from_gpr_index(&mut self, index: GprIndex, val: usize) {
        self.regs.guest_regs.gprs.set_reg(index, val);
    }

    /// Advance guest pc by `instr_len` bytes
    pub fn advance_pc(&mut self, instr_len: usize) {
        self.regs.guest_regs.sepc += instr_len
    }

    /// Gets the vCPU's registers.
    pub fn regs(&mut self) -> &mut VmCpuRegisters {
        &mut self.regs
    }
}

impl RISCVVCpu {
    fn vmexit_handler(&mut self) -> AxResult<AxVCpuExitReason> {
        self.regs.trap_csrs.load_from_hw();
        self.regs.vs_csrs.load_from_hw();

        let scause = scause::read();
        use riscv::interrupt::{Interrupt, Trap};

        use super::trap::Exception;

        trace!(
            "vmexit_handler: {:?}, sepc: {:#x}, stval: {:#x}",
            scause.cause(),
            self.regs.guest_regs.sepc,
            self.regs.trap_csrs.stval
        );

        // Try to convert the raw trap cause to a standard RISC-V trap cause.
        let trap: Trap<Interrupt, Exception> = scause.cause().try_into().map_err(|_| {
            error!("Unknown trap cause: scause={:#x}", scause.bits());
            AxError::from(AxErrorKind::InvalidData)
        })?;

        match trap {
            Trap::Exception(Exception::VirtualSupervisorEnvCall) => {
                let a = self.regs.guest_regs.gprs.a_regs();
                let param = [a[0], a[1], a[2], a[3], a[4], a[5]];
                let extension_id = a[7];
                let function_id = a[6];

                trace!(
                    "sbi_call: eid {:#x} ('{}') fid {:#x} param {:?}",
                    extension_id,
                    alloc::string::String::from_utf8_lossy(&(extension_id as u32).to_be_bytes()),
                    function_id,
                    param
                );
                match extension_id {
                    // Compatibility with Legacy Extensions.
                    legacy::LEGACY_SET_TIMER..=legacy::LEGACY_SHUTDOWN => match extension_id {
                        legacy::LEGACY_SET_TIMER => {
                            self.regs.vs_csrs.vstimecmp = param[0];
                            set_guest_timer(param[0] as u64);
                            unsafe {
                                // Clear guest timer interrupt
                                clear_host_timer_pending();
                                hip::clear_vstip();
                                hvip::clear_vstip();
                                vsip::clear_stip();
                                sie::set_stimer();
                            }

                            self.set_gpr_from_gpr_index(GprIndex::A0, 0);
                        }
                        legacy::LEGACY_CONSOLE_PUTCHAR => {
                            sbi_call_legacy_1(legacy::LEGACY_CONSOLE_PUTCHAR, param[0]);
                        }
                        legacy::LEGACY_CONSOLE_GETCHAR => {
                            let c = sbi_call_legacy_0(legacy::LEGACY_CONSOLE_GETCHAR);
                            self.set_gpr_from_gpr_index(GprIndex::A0, c);
                        }
                        legacy::LEGACY_SHUTDOWN => {
                            // sbi_call_legacy_0(LEGACY_SHUTDOWN)
                            return Ok(AxVCpuExitReason::SystemDown);
                        }
                        _ => {
                            warn!(
                                "Unsupported SBI legacy extension id {extension_id:#x} function \
                                 id {function_id:#x}"
                            );
                        }
                    },
                    EID_TIME => match function_id {
                        FID_SET_TIMER => {
                            self.regs.vs_csrs.vstimecmp = param[0];
                            set_guest_timer(param[0] as u64);
                            unsafe {
                                clear_host_timer_pending();
                                hip::clear_vstip();
                                hvip::clear_vstip();
                                vsip::clear_stip();
                                sie::set_stimer();
                            }
                            self.sbi_return(RET_SUCCESS, 0);
                            return Ok(AxVCpuExitReason::Nothing);
                        }
                        _ => {
                            self.sbi_return(RET_ERR_NOT_SUPPORTED, 0);
                            return Ok(AxVCpuExitReason::Nothing);
                        }
                    },
                    // Handle HSM extension
                    hsm::EID_HSM => match function_id {
                        hsm::HART_START => {
                            let hartid = a[0];
                            let start_addr = a[1];
                            let opaque = a[2];
                            self.advance_pc(4);
                            return Ok(AxVCpuExitReason::CpuUp {
                                target_cpu: hartid as _,
                                entry_point: GuestPhysAddr::from(start_addr),
                                arg: opaque as _,
                            });
                        }
                        hsm::HART_STOP => {
                            return Ok(AxVCpuExitReason::CpuDown { _state: 0 });
                        }
                        hsm::HART_SUSPEND => {
                            // Todo: support these parameters.
                            let _suspend_type = a[0];
                            let _resume_addr = a[1];
                            let _opaque = a[2];
                            return Ok(AxVCpuExitReason::Halt);
                        }
                        _ => todo!(),
                    },
                    // Handle hypercall
                    EID_HVC => {
                        self.advance_pc(4);
                        return Ok(AxVCpuExitReason::Hypercall {
                            nr: function_id as _,
                            args: [
                                param[0] as _,
                                param[1] as _,
                                param[2] as _,
                                param[3] as _,
                                param[4] as _,
                                param[5] as _,
                            ],
                        });
                    }
                    // Debug Console Extension
                    EID_DBCN => match function_id {
                        // Write from memory region to debug console.
                        FID_CONSOLE_WRITE => {
                            let num_bytes = param[0];
                            let gpa = join_u64(param[1], param[2]);

                            if num_bytes == 0 {
                                self.sbi_return(RET_SUCCESS, 0);
                                return Ok(AxVCpuExitReason::Nothing);
                            }

                            let mut buf = alloc::vec![0u8; num_bytes];
                            let copied = guest_mem::copy_from_guest(
                                &mut buf,
                                GuestPhysAddr::from(gpa as usize),
                            );
                            let copied = if copied == buf.len() {
                                copied
                            } else {
                                debug!(
                                    "DBCN write fallback to GVA copy: addr={:#x}, copied={}, \
                                     len={}",
                                    gpa,
                                    copied,
                                    buf.len()
                                );
                                guest_mem::copy_from_guest_va(
                                    &mut buf,
                                    GuestVirtAddr::from(gpa as usize),
                                )
                            };

                            if copied == buf.len() {
                                let ret = console_write(&buf);
                                self.sbi_return(ret.error, ret.value);
                            } else {
                                self.sbi_return(RET_ERR_FAILED, 0);
                            }

                            return Ok(AxVCpuExitReason::Nothing);
                        }
                        // Read to memory region from debug console.
                        FID_CONSOLE_READ => {
                            let num_bytes = param[0];
                            let gpa = join_u64(param[1], param[2]);

                            if num_bytes == 0 {
                                self.sbi_return(RET_SUCCESS, 0);
                                return Ok(AxVCpuExitReason::Nothing);
                            }

                            let mut buf = alloc::vec![0u8; num_bytes];
                            let ret = console_read(&mut buf);

                            if ret.is_ok() && ret.value <= buf.len() {
                                let copied = guest_mem::copy_to_guest(
                                    &buf[..ret.value],
                                    GuestPhysAddr::from(gpa as usize),
                                );
                                let copied = if copied == ret.value {
                                    copied
                                } else {
                                    debug!(
                                        "DBCN read fallback to GVA copy: addr={:#x}, copied={}, \
                                         len={}",
                                        gpa, copied, ret.value
                                    );
                                    guest_mem::copy_to_guest_va(
                                        &buf[..ret.value],
                                        GuestVirtAddr::from(gpa as usize),
                                    )
                                };
                                if copied == ret.value {
                                    self.sbi_return(RET_SUCCESS, ret.value);
                                } else {
                                    self.sbi_return(RET_ERR_FAILED, 0);
                                }
                            } else {
                                self.sbi_return(ret.error, ret.value);
                            }

                            return Ok(AxVCpuExitReason::Nothing);
                        }
                        // Write a single byte to debug console.
                        FID_CONSOLE_WRITE_BYTE => {
                            let byte = (param[0] & 0xff) as u8;
                            print_byte(byte);
                            self.sbi_return(RET_SUCCESS, 0);
                            return Ok(AxVCpuExitReason::Nothing);
                        }
                        // Unknown FID.
                        _ => {
                            self.sbi_return(RET_ERR_NOT_SUPPORTED, 0);
                            return Ok(AxVCpuExitReason::Nothing);
                        }
                    },
                    srst::EID_SRST => match function_id {
                        srst::SYSTEM_RESET => {
                            let reset_type = param[0];
                            if reset_type == srst::RESET_TYPE_SHUTDOWN as _ {
                                // Shutdown the system.
                                return Ok(AxVCpuExitReason::SystemDown);
                            } else {
                                unimplemented!("Unsupported reset type {}", reset_type);
                            }
                        }
                        _ => {
                            self.sbi_return(RET_ERR_NOT_SUPPORTED, 0);
                            return Ok(AxVCpuExitReason::Nothing);
                        }
                    },
                    // By default, forward the SBI call to the RustSBI implementation.
                    // See [`RISCVVCpuSbi`].
                    _ => {
                        let ret = self.sbi.handle_ecall(extension_id, function_id, param);
                        if ret.is_err() {
                            debug!(
                                "forward ecall eid {:#x} fid {:#x} param {:#x?} err {:#x} value \
                                 {:#x}",
                                extension_id, function_id, param, ret.error, ret.value
                            );
                        }
                        self.set_gpr_from_gpr_index(GprIndex::A0, ret.error);
                        self.set_gpr_from_gpr_index(GprIndex::A1, ret.value);
                    }
                };

                self.advance_pc(4);
                Ok(AxVCpuExitReason::Nothing)
            }
            Trap::Interrupt(Interrupt::SupervisorTimer) => {
                unsafe {
                    // Deliver exactly one virtual timer interrupt and let the guest
                    // arm the next deadline via SBI set_timer().
                    set_host_timer(u64::MAX);
                    clear_host_timer_pending();
                    sie::clear_stimer();
                }
                unsafe {
                    hvip::set_vstip();
                }

                Ok(AxVCpuExitReason::Nothing)
            }
            Trap::Interrupt(Interrupt::SupervisorExternal) => {
                // 9 == Interrupt::SupervisorExternal
                //
                // It's a great fault in the `riscv` crate that `Interrupt` and `Exception` are not
                // explicitly numbered, and they provide no way to convert them to a number. Also,
                // `as usize` will give use a wrong value.
                Ok(AxVCpuExitReason::ExternalInterrupt { vector: S_EXT as _ })
            }
            Trap::Exception(
                gpf @ (Exception::LoadGuestPageFault | Exception::StoreGuestPageFault),
            ) => self.handle_guest_page_fault(gpf == Exception::StoreGuestPageFault),
            Trap::Exception(Exception::VirtualInstruction) => self.handle_virtual_instruction(),
            _ => {
                panic!(
                    "Unhandled trap: {:?}, sepc: {:#x}, stval: {:#x}, htval: {:#x}, htinst: \
                     {:#x}, vsepc: {:#x}, vstval: {:#x}, vsatp: {:#x}, hgatp: {:#x}, a0-a3: \
                     [{:#x}, {:#x}, {:#x}, {:#x}]",
                    scause.cause(),
                    self.regs.guest_regs.sepc,
                    self.regs.trap_csrs.stval,
                    self.regs.trap_csrs.htval,
                    self.regs.trap_csrs.htinst,
                    self.regs.vs_csrs.vsepc,
                    self.regs.vs_csrs.vstval,
                    self.regs.vs_csrs.vsatp,
                    self.regs.virtual_hs_csrs.hgatp,
                    self.regs.guest_regs.gprs.reg(GprIndex::A0),
                    self.regs.guest_regs.gprs.reg(GprIndex::A1),
                    self.regs.guest_regs.gprs.reg(GprIndex::A2),
                    self.regs.guest_regs.gprs.reg(GprIndex::A3)
                );
            }
        }
    }

    #[inline]
    fn sbi_return(&mut self, a0: usize, a1: usize) {
        self.set_gpr_from_gpr_index(GprIndex::A0, a0);
        self.set_gpr_from_gpr_index(GprIndex::A1, a1);
        self.advance_pc(4);
    }

    /// Decode the instruction at the given virtual address. Return the decoded instruction and its
    /// length in bytes.
    fn decode_instr_at(&self, vaddr: GuestVirtAddr) -> AxResult<(Instruction, usize)> {
        // The htinst CSR contains "transformed instruction" that caused the page fault. We
        // can use it but we use the sepc to fetch the original instruction instead for now.
        let mut instr = riscv_h::register::htinst::read();
        let instr_len;
        if instr == 0 {
            // Read the instruction from guest memory.
            instr = guest_mem::fetch_guest_instruction(vaddr) as _;
            instr_len = riscv_decode::instruction_length(instr as u16);
            instr = match instr_len {
                2 => instr & 0xffff,
                4 => instr,
                _ => unreachable!("Unsupported instruction length: {}", instr_len),
            };
        } else if instr_is_pseudo(instr as u32) {
            error!("fault on 1st stage page table walk");
            return Err(axerrno::ax_err_type!(
                Unsupported,
                "risc-v vcpu guest page fault handler encountered pseudo instruction"
            ));
        } else {
            // Transform htinst value to standard instruction.
            // According to RISC-V Spec:
            //      Bits 1:0 of a transformed standard instruction will be binary 01 if
            //      the trapping instruction is compressed and 11 if not.
            instr_len = match (instr as u16) & 0x3 {
                0x1 => 2,
                0x3 => 4,
                _ => unreachable!("Unsupported instruction length"),
            };
            instr |= 0x2;
        }

        riscv_decode::decode(instr as u32)
            .map_err(|_| {
                axerrno::ax_err_type!(
                    Unsupported,
                    "risc-v vcpu guest pf handler decoding instruction failed"
                )
            })
            .map(|instr| (instr, instr_len))
    }

    /// Handle a guest page fault. Return an exit reason.
    fn handle_guest_page_fault(&mut self, _writing: bool) -> AxResult<AxVCpuExitReason> {
        let fault_addr = self.regs.trap_csrs.gpt_page_fault_addr();
        let sepc = self.regs.guest_regs.sepc;
        let sepc_vaddr = GuestVirtAddr::from(sepc);

        /// Temporary enum to represent the decoded operation.
        enum DecodedOp {
            Read {
                i: IType,
                width: AccessWidth,
                signed_ext: bool,
            },
            Write {
                s: SType,
                width: AccessWidth,
            },
        }

        use DecodedOp::*;

        let (decoded_instr, instr_len) = self.decode_instr_at(sepc_vaddr)?;
        let op = match decoded_instr {
            Instruction::Lb(i) => Read {
                i,
                width: AccessWidth::Byte,
                signed_ext: true,
            },
            Instruction::Lh(i) => Read {
                i,
                width: AccessWidth::Word,
                signed_ext: true,
            },
            Instruction::Lw(i) => Read {
                i,
                width: AccessWidth::Dword,
                signed_ext: true,
            },
            Instruction::Ld(i) => Read {
                i,
                width: AccessWidth::Qword,
                signed_ext: true,
            },
            Instruction::Lbu(i) => Read {
                i,
                width: AccessWidth::Byte,
                signed_ext: false,
            },
            Instruction::Lhu(i) => Read {
                i,
                width: AccessWidth::Word,
                signed_ext: false,
            },
            Instruction::Lwu(i) => Read {
                i,
                width: AccessWidth::Dword,
                signed_ext: false,
            },
            Instruction::Sb(s) => Write {
                s,
                width: AccessWidth::Byte,
            },
            Instruction::Sh(s) => Write {
                s,
                width: AccessWidth::Word,
            },
            Instruction::Sw(s) => Write {
                s,
                width: AccessWidth::Dword,
            },
            Instruction::Sd(s) => Write {
                s,
                width: AccessWidth::Qword,
            },
            _ => {
                // Not a load or store instruction, so we cannot handle it here, return a nested page fault.
                return Ok(AxVCpuExitReason::NestedPageFault {
                    addr: fault_addr,
                    access_flags: MappingFlags::empty(),
                });
            }
        };

        // WARN: This is a temporary place to add the instruction length to the guest's sepc.
        self.advance_pc(instr_len);

        Ok(match op {
            Read {
                i,
                width,
                signed_ext,
            } => AxVCpuExitReason::MmioRead {
                addr: fault_addr,
                width,
                reg: i.rd() as _,
                reg_width: AccessWidth::Qword,
                signed_ext,
            },
            Write { s, width } => {
                let source_reg = s.rs2();
                let value = self.get_gpr(unsafe {
                    // SAFETY: `source_reg` is guaranteed to be in [0, 31]
                    GprIndex::from_raw(source_reg).unwrap_unchecked()
                });

                AxVCpuExitReason::MmioWrite {
                    addr: fault_addr,
                    width,
                    data: value as _,
                }
            }
        })
    }

    /// Handle virtual instruction traps caused by VS-mode privileged instructions
    /// that Linux executes during early boot, such as `satp` updates, `sfence.vma`,
    /// and `wfi`.
    fn handle_virtual_instruction(&mut self) -> AxResult<AxVCpuExitReason> {
        use riscv_decode::{
            Instruction,
            types::{CsrIType, CsrType, IType, UType},
        };

        const CSR_SATP: u32 = 0x180;
        const CSR_SSTATUS: u32 = 0x100;
        const CSR_SIE: u32 = 0x104;
        const CSR_STVEC: u32 = 0x105;
        const CSR_SSCRATCH: u32 = 0x140;
        const CSR_SEPC: u32 = 0x141;

        enum CsrOp {
            Write(CsrType),
            Set(CsrType),
            Clear(CsrType),
            WriteImm(CsrIType),
            SetImm(CsrIType),
            ClearImm(CsrIType),
        }

        fn sign_extend_12(imm: u32) -> isize {
            (((imm & 0xfff) as i32) << 20 >> 20) as isize
        }

        fn virtual_csr_read(this: &RISCVVCpu, csr_num: u32) -> Option<usize> {
            Some(match csr_num {
                CSR_SSTATUS => this.regs.vs_csrs.vsstatus,
                CSR_SIE => this.regs.vs_csrs.vsie,
                CSR_STVEC => this.regs.vs_csrs.vstvec,
                CSR_SSCRATCH => this.regs.vs_csrs.vsscratch,
                CSR_SEPC => this.regs.vs_csrs.vsepc,
                CSR_SATP => this.regs.vs_csrs.vsatp,
                _ => return None,
            })
        }

        fn virtual_csr_write(this: &mut RISCVVCpu, csr_num: u32, value: usize) -> bool {
            match csr_num {
                CSR_SSTATUS => this.regs.vs_csrs.vsstatus = value,
                CSR_SIE => this.regs.vs_csrs.vsie = value,
                CSR_STVEC => this.regs.vs_csrs.vstvec = value,
                CSR_SSCRATCH => this.regs.vs_csrs.vsscratch = value,
                CSR_SEPC => this.regs.vs_csrs.vsepc = value,
                CSR_SATP => {
                    this.regs.vs_csrs.vsatp = value;
                    unsafe {
                        let vsatp = Vsatp::from_bits(value);
                        vsatp.write();
                        core::arch::riscv64::hfence_vvma_all();
                    }
                }
                _ => return false,
            }
            true
        }

        self.regs.guest_regs.sepc = self.regs.vs_csrs.vsepc;
        let guest_pc = GuestVirtAddr::from(self.regs.guest_regs.sepc);
        let mut inst = [0u8; 12];
        if guest_mem::copy_from_guest_va(&mut inst, guest_pc) != inst.len() {
            return Err(axerrno::ax_err_type!(
                Unsupported,
                "failed to fetch virtual instruction from guest memory"
            ));
        }
        let raw_instr = u32::from_le_bytes(inst[0..4].try_into().unwrap());
        let instr_len = riscv_decode::instruction_length(raw_instr as u16);
        let raw_instr = match instr_len {
            2 => raw_instr & 0xffff,
            4 => raw_instr,
            _ => {
                return Err(axerrno::ax_err_type!(
                    Unsupported,
                    "unsupported virtual instruction length"
                ));
            }
        };
        let decoded_instr = riscv_decode::decode(raw_instr).map_err(|_| {
            axerrno::ax_err_type!(Unsupported, "failed to decode virtual instruction")
        })?;

        debug!(
            "guest virtual instruction at pc {:#x}: {:?}",
            self.regs.guest_regs.sepc, decoded_instr
        );

        let mut synthesized: Option<(CsrOp, usize, usize)> = None;
        if let Instruction::Auipc(u) = decoded_instr {
            let next1_raw = u32::from_le_bytes(inst[4..8].try_into().unwrap());
            let next2_raw = u32::from_le_bytes(inst[8..12].try_into().unwrap());
            let next1 = riscv_decode::decode(next1_raw);
            let next2 = riscv_decode::decode(next2_raw);
            if let (Ok(Instruction::Addi(i)), Ok(csr_instr)) = (next1, next2)
                && i.rd() == u.rd()
                && i.rs1() == u.rd()
            {
                let base = self.regs.guest_regs.sepc as isize + u.imm() as i32 as isize;
                let loaded_val = (base + sign_extend_12(i.imm())) as usize;
                let csr_op = match csr_instr {
                    Instruction::Csrrw(csr) if csr.rs1() == u.rd() => Some(CsrOp::Write(csr)),
                    Instruction::Csrrs(csr) if csr.rs1() == u.rd() => Some(CsrOp::Set(csr)),
                    Instruction::Csrrc(csr) if csr.rs1() == u.rd() => Some(CsrOp::Clear(csr)),
                    _ => None,
                };
                if let Some(csr_op) = csr_op {
                    synthesized = Some((csr_op, loaded_val, 12));
                }
            } else {
                warn!(
                    "virtual instruction window at guest pc {:#x}: first={:?}, next1={:?}, \
                     next2={:?}",
                    self.regs.guest_regs.sepc, decoded_instr, next1, next2
                );
            }
        }

        let (csr_op, current_val_override, handled_instr_len) =
            if let Some((op, val, len)) = synthesized {
                (op, Some(val), len)
            } else {
                let op = match decoded_instr {
                    Instruction::SfenceVma(_) | Instruction::Wfi => {
                        self.advance_pc(instr_len);
                        self.regs.vs_csrs.vsepc = self.regs.guest_regs.sepc;
                        return Ok(AxVCpuExitReason::Nothing);
                    }
                    Instruction::Csrrw(csr) => CsrOp::Write(csr),
                    Instruction::Csrrs(csr) => CsrOp::Set(csr),
                    Instruction::Csrrc(csr) => CsrOp::Clear(csr),
                    Instruction::Csrrwi(csr) => CsrOp::WriteImm(csr),
                    Instruction::Csrrsi(csr) => CsrOp::SetImm(csr),
                    Instruction::Csrrci(csr) => CsrOp::ClearImm(csr),
                    _ => {
                        warn!(
                            "unhandled virtual instruction at guest pc {:#x}: {:?}",
                            self.regs.guest_regs.sepc, decoded_instr
                        );
                        return Err(axerrno::ax_err_type!(
                            Unsupported,
                            "unhandled virtual instruction"
                        ));
                    }
                };
                (op, None, instr_len)
            };

        let (csr_num, rd, new_val) = match csr_op {
            CsrOp::Write(csr) => (
                csr.csr(),
                csr.rd(),
                current_val_override
                    .unwrap_or_else(|| self.get_gpr(GprIndex::from_raw(csr.rs1()).unwrap())),
            ),
            CsrOp::Set(csr) => (
                csr.csr(),
                csr.rd(),
                virtual_csr_read(self, csr.csr()).unwrap_or(0)
                    | current_val_override
                        .unwrap_or_else(|| self.get_gpr(GprIndex::from_raw(csr.rs1()).unwrap())),
            ),
            CsrOp::Clear(csr) => (
                csr.csr(),
                csr.rd(),
                virtual_csr_read(self, csr.csr()).unwrap_or(0)
                    & !current_val_override
                        .unwrap_or_else(|| self.get_gpr(GprIndex::from_raw(csr.rs1()).unwrap())),
            ),
            CsrOp::WriteImm(csr) => (csr.csr(), csr.rd(), csr.zimm() as usize),
            CsrOp::SetImm(csr) => (
                csr.csr(),
                csr.rd(),
                virtual_csr_read(self, csr.csr()).unwrap_or(0) | csr.zimm() as usize,
            ),
            CsrOp::ClearImm(csr) => (
                csr.csr(),
                csr.rd(),
                virtual_csr_read(self, csr.csr()).unwrap_or(0) & !(csr.zimm() as usize),
            ),
        };

        let current_val = match virtual_csr_read(self, csr_num) {
            Some(val) => val,
            None => {
                warn!(
                    "unsupported virtual CSR access at guest pc {:#x}: csr={:#x}, instr={:?}",
                    self.regs.guest_regs.sepc, csr_num, decoded_instr
                );
                return Err(axerrno::ax_err_type!(
                    Unsupported,
                    "unsupported virtual CSR access"
                ));
            }
        };

        if rd != 0 {
            self.set_gpr_from_gpr_index(GprIndex::from_raw(rd).unwrap(), current_val);
        }

        if !virtual_csr_write(self, csr_num, new_val) {
            warn!(
                "unsupported virtual CSR access at guest pc {:#x}: csr={:#x}, instr={:?}",
                self.regs.guest_regs.sepc, csr_num, decoded_instr
            );
            return Err(axerrno::ax_err_type!(
                Unsupported,
                "unsupported virtual CSR access"
            ));
        }

        self.advance_pc(handled_instr_len);
        self.regs.vs_csrs.vsepc = self.regs.guest_regs.sepc;
        Ok(AxVCpuExitReason::Nothing)
    }
}

#[inline(always)]
fn sbi_call_legacy_0(eid: usize) -> usize {
    let error;
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            lateout("a0") error,
        );
    }
    error
}

#[inline(always)]
fn sbi_call_legacy_1(eid: usize, arg0: usize) -> usize {
    let error;
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            inlateout("a0") arg0 => error,
        );
    }
    error
}
