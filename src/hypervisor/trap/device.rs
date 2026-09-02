use riscv::addr::BitField;

use crate::constants::csr::sie::STIE_BIT;
use crate::constants::csr::sip::STIP_BIT;
use crate::page_table::PageTable;
use crate::debug::PageDebug;
use crate::guest::Vcpu;
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

pub fn handle_qemu_virt<P: PageTable + PageDebug>(guest: &mut Vcpu<P>, ctx: &mut TrapContext) {
    let (len, inst) = decode_instruction_at_address(guest, ctx.sepc);
    if let Some(inst) = inst {
        match inst {
            riscv_decode::Instruction::Sw(i) => {
                let rs1 = i.rs1() as usize;
                let rs2 = i.rs2() as usize;
                let vaddr = instruction_address(register_value(ctx, rs1), i.imm());
                let value = register_value(ctx, rs2);
                if crate::device_emu::is_device_access(vaddr) {
                    guest
                        .virt_device
                        .virtio
                        .write(vaddr, value as u32, &guest.guest_memory);
                }else{
                    guest.virt_device.qemu_virt_tester.mmregs[vaddr] = value as u32;
                }
            },
            riscv_decode::Instruction::Lw(i) => {
                let vaddr = instruction_address(register_value(ctx, i.rs1() as usize), i.imm());
                let value = guest.virt_device.virtio.read(vaddr);
                if i.rd() != 0 {
                    ctx.x[i.rd() as usize] = value as i32 as isize as usize;
                }
            },
            riscv_decode::Instruction::Lwu(i) => {
                let vaddr = instruction_address(register_value(ctx, i.rs1() as usize), i.imm());
                let value = guest.virt_device.virtio.read(vaddr);
                if i.rd() != 0 {
                    ctx.x[i.rd() as usize] = value as usize;
                }
            }
            _ => panic!("stval: {:#x}", ctx.sepc)
        }
    }
    ctx.sepc += len;
}

/// 时钟中断处理函数
pub fn handle_time_interrupt<P: PageTable + PageDebug>(guest: &mut Vcpu<P>) {
    let time = get_time();
    let mut next = time + get_default_timer();
    if guest.shadow_state.csrs.sie.get_bit(STIE_BIT) {
        if guest.shadow_state.csrs.mtimecmp <= time {
            // 表明此时 Guest OS 发生中断
            guest.shadow_state.interrupt = true;
            // 设置 sip 寄存器
            guest.shadow_state.csrs.sip.set_bit(STIP_BIT, true);
        }else{
            // 未发生中断，设置下次中断
            next = next.min(guest.shadow_state.csrs.mtimecmp)
        }
    }
    // 设置下次中断
    set_timer(next);
}

#[inline(always)]
pub fn is_device_access(guest_pa: usize) -> bool {
    guest_pa >= 0x1000_1000 && guest_pa < 0x1000_1000 + 1000
}
