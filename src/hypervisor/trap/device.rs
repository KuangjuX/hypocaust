use riscv::addr::BitField;

use crate::constants::csr::sie::STIE_BIT;
use crate::constants::csr::sip::STIP_BIT;
use crate::page_table::PageTable;
use crate::debug::PageDebug;
use crate::guest::GuestKernel;
use crate::sbi::set_timer;
use crate::timer::get_default_timer;
use crate::timer::get_time;

use super::TrapContext;
use super::decode_instruction_at_address;


pub fn handle_qemu_virt<P: PageTable + PageDebug>(guest: &mut GuestKernel<P>, ctx: &mut TrapContext) {
    let (len, inst) = decode_instruction_at_address(guest, ctx.sepc);
    if let Some(inst) = inst {
        match inst {
            riscv_decode::Instruction::Sw(i) => {
                let rs1 = i.rs1() as usize;
                let rs2 = i.rs2() as usize;
                let offset: isize = if i.imm() > 2048 { ((0b1111 << 12) | i.imm()) as i16 as isize }else{  i.imm() as isize };
                let vaddr = (ctx.x[rs1] as isize + offset) as usize; 
                let value = ctx.x[rs2];
                guest.virt_device.qemu_virt_tester.mmregs[vaddr] = value as u32;
            }
            _ => panic!("stval: {:#x}", ctx.sepc)
        }
    }
    ctx.sepc += len;
}

/// 时钟中断处理函数
pub fn handle_time_interrupt<P: PageTable + PageDebug>(guest: &mut GuestKernel<P>) {
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

// pub fn handle_device_access<P: PageTable + PageDebug>(guest: &mut GuestKernel<P>, ctx: &TrapContext, guest_pa: usize) {
//     let device = ((guest_pa - 0x1000_1000) / 0x1000) as usize;
//     // 目前只支持 0 号设备
//     assert_eq!(device, 0);
//     let offset = guest_pa & 0xfff;
//     let (len, inst) = decode_instruction_at_address(guest, ctx.sepc);
//     match inst {
//         Some(riscv_decode::Instruction::Lw(i)) => {

//         }
//         Some(riscv_decode::Instruction::Lb(i)) => {

//         }
//         Some(riscv_decode::Instruction::Sw(i)) => {

//         }
//         Some(instr) => {
//             hwarning!("VIRTIO: Instruction {:?} used to target addr {:#x} from pc {:#x}", instr, guest_pa, ctx.sepc);
//             loop{}
//         }
//         None => {
//             hwarning!("Unrecognized instruction targetting VIRTIO {:#x} at {:#x}", guest_pa, ctx.sepc);
//             loop{}
//         }
//     }
//     ctx.sepc += len;
// }