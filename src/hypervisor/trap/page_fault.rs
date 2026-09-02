use riscv::register::stval;

use crate::page_table::{PageTable,  PageTableEntry};
use crate::debug::PageDebug;
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
        return handle_device_mmio(guest, device_bus, ctx);
    }
    if guest_va % core::mem::size_of::<PageTableEntry>() != 0 {
        // PR #48 (`fix-bug/guest-exception-forwarding`) treats an ordinary
        // unaligned Guest page fault as a Guest exception, not a Host assert.
        return false;
    }
    let sepc = ctx.sepc;
    let (len, inst) = decode_instruction_at_address(guest, sepc);
    if let Some(translation) = guest.translate_guest_vaddr(guest_va) {
        // PR #48 proves that the fault targets a tracked Guest page-table page
        // before interpreting a store value as a PTE. This prevents aligned
        // writes to ordinary read-only Guest memory from corrupting the SPT.
        let Some(guest_pte_gpa) = guest.guest_memory.hpa_to_gpa(translation) else {
            return false;
        };
        if !guest
            .shadow_state
            .shadow_page_tables
            .tracks_page_table_page(guest_pte_gpa)
        {
            return false;
        }
        // PR #19 (fix-bug/guest-pte-write-translation): use the guest page-table walk's
        // host address because kernel virtual PTE aliases are not linear GPAs.
        let pte_bits = match inst {
            Some(riscv_decode::Instruction::Sd(i)) => {
                let rs1 = i.rs1() as usize;
                let rs2 = i.rs2() as usize;
                let offset: isize = if i.imm() > 2048 { ((0b1111 << 12) | i.imm()) as i16 as isize }else{  i.imm() as isize };
                let vaddr = (ctx.x[rs1] as isize + offset) as usize;
                if vaddr != guest_va {
                    return false;
                }
                ctx.x[rs2]
            }
            _ => return false,
        };
        let pte = PageTableEntry { bits: pte_bits };
        let guest_pte_addr = translation;
        // PR #27 (`feature/track-valid-pte-count`) supplies both V-bit states so
        // the per-page count can be updated without scanning all 512 PTEs.
        let old_pte = PageTableEntry {
            bits: unsafe { core::ptr::read(guest_pte_addr as *const usize) },
        };
        unsafe{ core::ptr::write(guest_pte_addr as *mut usize, pte.bits)}

        // PR #48 synchronizes by canonical GPA so a kernel virtual alias never
        // escapes the owning VM's shadow-memory slot.
        guest.synchronize_page_table(guest_pte_gpa, old_pte, pte);
        ctx.sepc += len;
        return true;
    }
    false
}
