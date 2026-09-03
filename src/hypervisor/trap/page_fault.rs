use riscv::register::stval;

use crate::page_table::{PageTable,  PageTableEntry};
use crate::debug::PageDebug;
use crate::device_emu::DeviceBus;
use crate::guest::{Vcpu, PageTableRoot};
use super::{decode_instruction_at_address, handle_device_mmio};

use super::TrapContext;

const AMO_OPCODE: u32 = 0b010_1111;
const AMO_WIDTH_DOUBLEWORD: u32 = 0b011;
const AMO_ADD: u32 = 0b00000;
const AMO_SWAP: u32 = 0b00001;
const AMO_XOR: u32 = 0b00100;
const AMO_OR: u32 = 0b01000;
const AMO_AND: u32 = 0b01100;
const AMO_MIN: u32 = 0b10000;
const AMO_MAX: u32 = 0b10100;
const AMO_MIN_UNSIGNED: u32 = 0b11000;
const AMO_MAX_UNSIGNED: u32 = 0b11100;

#[derive(Clone, Copy)]
struct EmulatedPteWrite {
    new_bits: usize,
    rd: Option<usize>,
}

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
    // PR #68 walks Linux's high virtual MMIO alias back to a GPA before asking
    // the VM-local bus. The shadow leaf intentionally stays absent so the
    // virtual PLIC receives every access instead of exposing the Host PLIC.
    if let Some(guest_pa) = guest
        .translate_guest_vaddr_to_gpa(guest_va)
        .filter(|guest_pa| device_bus.contains(*guest_pa))
    {
        return handle_device_mmio(guest, device_bus, ctx, guest_va, guest_pa);
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
        let guest_pte_addr = translation;
        let old_pte = PageTableEntry {
            bits: unsafe { core::ptr::read(guest_pte_addr as *const usize) },
        };
        // PR #19 (fix-bug/guest-pte-write-translation): use the guest page-table
        // walk's Host address because kernel virtual PTE aliases are not GPAs.
        let write = match inst {
            Some(riscv_decode::Instruction::Sd(i)) => {
                let rs1 = i.rs1() as usize;
                let rs2 = i.rs2() as usize;
                let offset: isize = if i.imm() > 2048 { ((0b1111 << 12) | i.imm()) as i16 as isize }else{  i.imm() as isize };
                let vaddr = (register_value(ctx, rs1) as isize + offset) as usize;
                if vaddr != guest_va {
                    return false;
                }
                EmulatedPteWrite {
                    new_bits: register_value(ctx, rs2),
                    rd: None,
                }
            }
            // PR #57 (`feature/linux-pte-atomics`) handles Linux's 64-bit AMO
            // PTE updates even though the legacy instruction decoder does not
            // expose A-extension variants.
            _ => match emulate_pte_amo(guest, ctx, sepc, guest_va, old_pte.bits) {
                Some(write) => write,
                None => return false,
            },
        };
        let pte = PageTableEntry { bits: write.new_bits };
        // PR #27 (`feature/track-valid-pte-count`) supplies both V-bit states so
        // the per-page count can be updated without scanning all 512 PTEs.
        unsafe{ core::ptr::write(guest_pte_addr as *mut usize, pte.bits)}

        if let Some(rd) = write.rd {
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            write_register(ctx, rd, old_pte.bits);
        }

        // PR #48 synchronizes by canonical GPA so a kernel virtual alias never
        // escapes the owning VM's shadow-memory slot.
        guest.synchronize_page_table(guest_pte_gpa, old_pte, pte);
        ctx.sepc += len;
        return true;
    }
    false
}

fn register_value(ctx: &TrapContext, register: usize) -> usize {
    if register == 0 { 0 } else { ctx.x[register] }
}

fn write_register(ctx: &mut TrapContext, register: usize, value: usize) {
    if register != 0 {
        ctx.x[register] = value;
    }
}

/// Decode and execute a trapped RV64 AMO against a tracked PTE.
///
/// PR #57 uses a stronger-than-required full fence because all Guest exits are
/// serialized by the Hypervisor lock. This preserves aq/rl ordering without
/// allowing the faulting instruction to touch the read-only page directly.
fn emulate_pte_amo<P: PageTable + PageDebug>(
    guest: &Vcpu<P>,
    ctx: &TrapContext,
    instruction_va: usize,
    fault_va: usize,
    old: usize,
) -> Option<EmulatedPteWrite> {
    // Read two halfwords through separate translations so a 32-bit instruction
    // at a Guest page boundary never assumes contiguous Host backing pages.
    let low_hpa = guest.translate_guest_vaddr(instruction_va)?;
    let high_hpa = guest.translate_guest_vaddr(instruction_va.checked_add(2)?)?;
    let low = unsafe { core::ptr::read(low_hpa as *const u16) } as u32;
    let high = unsafe { core::ptr::read(high_hpa as *const u16) } as u32;
    let instruction = low | (high << 16);
    if instruction & 0x7f != AMO_OPCODE || (instruction >> 12) & 0x7 != AMO_WIDTH_DOUBLEWORD {
        return None;
    }

    let rs1 = ((instruction >> 15) & 0x1f) as usize;
    let rs2 = ((instruction >> 20) & 0x1f) as usize;
    let rd = ((instruction >> 7) & 0x1f) as usize;
    if register_value(ctx, rs1) != fault_va {
        return None;
    }
    let operation = (instruction >> 27) & 0x1f;
    let operand = register_value(ctx, rs2);
    let new_bits = apply_amo(operation, old, operand)?;
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    Some(EmulatedPteWrite {
        new_bits,
        rd: Some(rd),
    })
}

fn apply_amo(operation: u32, old: usize, operand: usize) -> Option<usize> {
    match operation {
        AMO_ADD => Some(old.wrapping_add(operand)),
        AMO_SWAP => Some(operand),
        AMO_XOR => Some(old ^ operand),
        AMO_OR => Some(old | operand),
        AMO_AND => Some(old & operand),
        AMO_MIN => Some(((old as isize).min(operand as isize)) as usize),
        AMO_MAX => Some(((old as isize).max(operand as isize)) as usize),
        AMO_MIN_UNSIGNED => Some(old.min(operand)),
        AMO_MAX_UNSIGNED => Some(old.max(operand)),
        _ => None,
    }
}

/// PR #57 checks the PTE-relevant RV64 AMO arithmetic at boot.
pub(super) fn pte_atomic_self_test() {
    assert_eq!(apply_amo(AMO_OR, 0b0101, 0b1010), Some(0b1111));
    assert_eq!(apply_amo(AMO_AND, 0b1101, 0b1011), Some(0b1001));
    assert_eq!(apply_amo(AMO_ADD, usize::MAX, 1), Some(0));
    assert_eq!(apply_amo(AMO_MIN, usize::MAX, 1), Some(usize::MAX));
    assert_eq!(apply_amo(0b00010, 1, 2), None);
}
