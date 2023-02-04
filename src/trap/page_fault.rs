use riscv::register::{stval, scause};

use crate::page_table::{PageTable, PTEFlags, translate_guest_address, PageTableEntry, PhysPageNum};
use crate::debug::{PageDebug};
use crate::guest::{GuestKernel, PageTableRoot, gpt2spt, gpa2hpa};
use crate::trap::fault::{decode_instruction_at_address, handle_qemu_virt}; 

use super::TrapContext;

pub fn handle_page_fault<P: PageTable + PageDebug>(guest: &mut GuestKernel<P>, ctx: &mut TrapContext) -> bool {
    let hart_id = guest.index;
    let shadow = guest.shadow();
    if shadow == PageTableRoot::GPA {
        hdebug!("Page fault without paginf enabled?");
        return false;
    }
    let access = match scause::read().cause() {
        scause::Trap::Exception(scause::Exception::InstructionPageFault) => PTEFlags::X,
        scause::Trap::Exception(scause::Exception::LoadPageFault) => PTEFlags::R,
        scause::Trap::Exception(scause::Exception::StorePageFault) => PTEFlags::W,
        _ => unreachable!()
    };
    // 获取发生错误的 guest virtual address
    let guest_va = stval::read();
    let (len, _) = decode_instruction_at_address(guest, ctx.sepc);
    // 处理设备错误
    if guest.virt_device.qemu_virt_tester.in_region(guest_va){
        handle_qemu_virt(guest, ctx);
        ctx.sepc += len;
        return true;
    }

    let root_page_table = (guest.shadow_state.get_satp() & 0xfff_ffff_ffff) << 12;
    hdebug!("root page table: {:#x}", root_page_table);
    if let Some(translation) = translate_guest_address::<P>(hart_id, root_page_table, guest_va) {
        // 获得翻译信息
        // Check R/W/X bits
        if translation.pte.flags() & access == PTEFlags::empty() {
            return false;
        }
        // Check U bit
        match shadow {
            PageTableRoot::UVA => if !translation.pte.is_user(){ return false }
            PageTableRoot::GVA => if translation.pte.is_user(){ return false }
            _ => unreachable!()
        }

        // Set non leaf ptes
        let path = &translation.page_walk.path;
        for i in 0..=1 {
            let guest_pa = path[i].addr;
            let host_pa = gpt2spt(guest_pa, hart_id);
            let non_leaf_pte = path[i].pte;
            let new_non_leaf_pte = PageTableEntry::new(PhysPageNum::from(gpt2spt(non_leaf_pte.ppn().0 << 12, hart_id) >> 12), non_leaf_pte.flags());
            unsafe{core::ptr::write(host_pa as *mut usize, new_non_leaf_pte.bits);}
        }
        
        // leaf host pa
        let host_pa = gpa2hpa(translation.guest_pa, hart_id);

        // Set A and D bits
        let new_pte = if !translation.pte.dirty() && access == PTEFlags::W {
            PageTableEntry::new(translation.pte.ppn(), translation.pte.flags() | PTEFlags::D | PTEFlags::A)
        }else if !translation.pte.accessed() {
            PageTableEntry::new(translation.pte.ppn(), translation.pte.flags() | PTEFlags::A)
        }else{
            translation.pte
        };

        if new_pte != translation.pte {
            unsafe{
                core::ptr::write(gpa2hpa(translation.pte_addr, hart_id) as *mut usize, new_pte.bits);
            }
        }

        let perm = if !new_pte.dirty() && access != PTEFlags::W {
            new_pte.flags() & (PTEFlags::R | PTEFlags::X)
        }else{
            new_pte.flags() & (PTEFlags::R | PTEFlags::W | PTEFlags::X)
        };

        let new_shadow_pte = PageTableEntry::new(
            PhysPageNum::from(host_pa >> 12), 
            perm | PTEFlags::U | PTEFlags::A | PTEFlags::D | PTEFlags::V
        );
        let shadow_pte_addr = gpt2spt(translation.pte_addr, hart_id);
        unsafe{ core::ptr::write(shadow_pte_addr as *mut usize, new_shadow_pte.bits)}

        ctx.sepc += len;
        return true;
    }
    false
}