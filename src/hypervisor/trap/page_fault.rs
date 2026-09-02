use riscv::register::stval;

use crate::page_table::{PageTable,  PageTableEntry};
use crate::debug::{PageDebug, print_guest_backtrace};
use crate::device_emu::DeviceBus;
use crate::guest::{Vcpu, PageTableRoot};
use super::{decode_instruction_at_address, handle_device_mmio};

use super::TrapContext;

pub fn handle_page_fault<P: PageTable + PageDebug>(
    guest: &mut Vcpu<P>,
    device_bus: &mut DeviceBus,
    ctx: &mut TrapContext,
) -> bool {
    let shadow = guest.shadow();
    if shadow == PageTableRoot::GPA {
        hdebug!("Page fault without paging enabled?");
        return false;
    }
    if shadow == PageTableRoot::UVA {
        // 用户态触发异常，进行转发
        hwarning!("Page fault from U mode?");
        return false;
    }

    let guest_va = stval::read();
    // PR #17 (fix-bug/virtio-dma-translation): MMIO is word-aligned, so
    // route them before enforcing the page-table-entry alignment invariant.
    // PR #39 asks the current VM's bus whether this fault is MMIO. Global
    // address tests could accidentally route one VM to another VM's device.
    if device_bus.contains(guest_va) {
        handle_device_mmio(guest, device_bus, ctx);
        return true;
    }
    if guest_va % core::mem::size_of::<PageTableEntry>() != 0 {
        hwarning!("guest va: {:#x}, sepc: {:#x}", guest_va, ctx.sepc);
        print_guest_backtrace::<P>(&guest.shadow_state.shadow_page_tables.guest_page_table().unwrap(), guest.shadow_state.csrs.satp, ctx)
    }
    assert_eq!(guest_va % core::mem::size_of::<PageTableEntry>(), 0);
    let sepc = ctx.sepc;
    let (len, inst) = decode_instruction_at_address(guest, sepc);
    let mut pte = 0;
    if let Some(translation) = guest.translate_guest_vaddr(guest_va) {
        // PR #19 (fix-bug/guest-pte-write-translation): use the guest page-table walk's
        // host address because kernel virtual PTE aliases are not linear GPAs.
        if let Some(inst) = inst {
            match inst {
                riscv_decode::Instruction::Sd(i) => {
                    let rs1 = i.rs1() as usize;
                    let rs2 = i.rs2() as usize;
                    let offset: isize = if i.imm() > 2048 { ((0b1111 << 12) | i.imm()) as i16 as isize }else{  i.imm() as isize };
                    let vaddr = (ctx.x[rs1] as isize + offset) as usize; 
                    assert_eq!(vaddr, guest_va);
                    pte = ctx.x[rs2];
                },
                riscv_decode::Instruction::Sb(_) | riscv_decode::Instruction::Sw(_) => {
                    panic!("Unsporrted instruction sepc -> {:#x}, stval: {:#x}", ctx.sepc, stval::read());
                }
                _ => { return false }
            }
        }
        let pte = PageTableEntry{ bits: pte };       
        let guest_pte_addr = translation;
        if guest_pte_addr >=  0x4000000000 {
            print_guest_backtrace(guest.shadow_state.shadow_page_tables.guest_page_table().unwrap(), guest.shadow_state.csrs.satp, ctx);
            panic!("guest va -> {:#x}, guest_pte_addr: {:#x}, sepc: {:#x}, translation: {:#x}", guest_va, guest_pte_addr, ctx.sepc, translation);
        }
        // PR #27 (`feature/track-valid-pte-count`) supplies both V-bit states so
        // the per-page count can be updated without scanning all 512 PTEs.
        let old_pte = PageTableEntry {
            bits: unsafe { core::ptr::read(guest_pte_addr as *const usize) },
        };
        unsafe{ core::ptr::write(guest_pte_addr as *mut usize, pte.bits)}

        guest.synchronize_page_table(guest_va, old_pte, pte);
        ctx.sepc += len;
        return true;
    }
    false
}
