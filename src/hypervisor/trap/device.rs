use riscv::addr::BitField;

use crate::constants::csr::sie::STIE_BIT;
use crate::page_table::PageTable;
use crate::debug::PageDebug;
use crate::device_emu::DeviceBus;
use crate::guest::{Vcpu, VirtualInterrupt};
use crate::sbi::set_timer;
use crate::timer::get_default_timer;
use crate::timer::get_time;

use super::TrapContext;
use super::decode_instruction_at_address;

/// PR #17 (fix-bug/virtio-dma-translation): reconstruct the faulting MMIO address
/// using the sign-extended 12-bit load/store immediate.
fn instruction_address(base: usize, immediate: u32) -> usize {
    let offset = ((immediate << 20) as i32 >> 20) as isize;
    (base as isize + offset) as usize
}

/// Return the architectural value of a source register, including hardwired x0.
fn register_value(ctx: &TrapContext, register: usize) -> usize {
    if register == 0 { 0 } else { ctx.x[register] }
}

/// PR #39 decodes one trapped MMIO instruction and delegates the reconstructed
/// Guest address to the current VM's device bus.
pub fn handle_device_mmio<P: PageTable + PageDebug>(
    guest: &mut Vcpu<P>,
    device_bus: &mut DeviceBus,
    ctx: &mut TrapContext,
) -> bool {
    let (len, inst) = decode_instruction_at_address(guest, ctx.sepc);
    if let Some(inst) = inst {
        match inst {
            riscv_decode::Instruction::Sw(i) => {
                let rs1 = i.rs1() as usize;
                let rs2 = i.rs2() as usize;
                let vaddr = instruction_address(register_value(ctx, rs1), i.imm());
                let value = register_value(ctx, rs2);
                // PR #39 routes the reconstructed Guest address only through
                // the DeviceBus owned by this vCPU's VM.
                if !device_bus.write_u32(vaddr, value as u32) {
                    return false;
                }
            },
            riscv_decode::Instruction::Lw(i) => {
                let vaddr = instruction_address(register_value(ctx, i.rs1() as usize), i.imm());
                let Some(value) = device_bus.read_u32(vaddr) else {
                    return false;
                };
                if i.rd() != 0 {
                    ctx.x[i.rd() as usize] = value as i32 as isize as usize;
                }
            },
            riscv_decode::Instruction::Lwu(i) => {
                let vaddr = instruction_address(register_value(ctx, i.rs1() as usize), i.imm());
                let Some(value) = device_bus.read_u32(vaddr) else {
                    return false;
                };
                if i.rd() != 0 {
                    ctx.x[i.rd() as usize] = value as usize;
                }
            }
            // PR #48 forwards unsupported Guest MMIO widths/instructions as
            // the original page fault instead of panicking the Host.
            _ => return false,
        }
    } else {
        return false;
    }
    // PR #43 selects the VM-local PLIC context rather than the globally unique
    // vCPU ID. Claim or VirtIO ACK can then deassert this vCPU's SEIP.
    if device_bus.has_irq(guest.guest_hart_id.index()) {
        guest.inject_virtual_interrupt(VirtualInterrupt::External);
    } else {
        guest.clear_virtual_interrupt(VirtualInterrupt::External);
    }
    ctx.sepc += len;
    true
}

/// 时钟中断处理函数
pub fn handle_time_interrupt<P: PageTable + PageDebug>(guest: &mut Vcpu<P>) {
    let time = get_time();
    let mut next = time + get_default_timer();
    if guest.shadow_state.csrs.sie.get_bit(STIE_BIT) {
        if guest.shadow_state.csrs.mtimecmp <= time {
            // PR #40 queues the timer interrupt only on this vCPU. Delivery is
            // arbitrated immediately before the next Guest entry.
            guest.inject_virtual_interrupt(VirtualInterrupt::Timer);
        }else{
            // 未发生中断，设置下次中断
            next = next.min(guest.shadow_state.csrs.mtimecmp)
        }
    }
    // 设置下次中断
    set_timer(next);
}

/// PR #43 turns asynchronous backend completion into the current vCPU's
/// VM-local PLIC context rather than indexing it by a global scheduler ID.
pub fn poll_device_completions<P: PageTable + PageDebug>(
    guest: &mut Vcpu<P>,
    device_bus: &mut DeviceBus,
) {
    if device_bus.poll_async(guest.guest_hart_id.index()) {
        guest.inject_virtual_interrupt(VirtualInterrupt::External);
    }
}
