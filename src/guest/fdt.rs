//! Per-VM flattened device-tree construction.
//!
//! PR #45 (`feature/multi-guest-qemu`) gives every Guest a private hardware
//! description containing only its RAM, boot CPU, virtual PLIC, and assigned
//! VirtIO block frontend. The Host DTB is never exposed to a Guest.

use alloc::vec::Vec;

use crate::constants::layout::{PAGE_SIZE, VM_MEMORY_SLOT_SIZE};
use crate::device_emu::{PLIC_GPA, PLIC_SIZE, VIRTIO_BLOCK_IRQ};
use crate::guest::GuestMemory;
use crate::identity::GuestHartId;

const FDT_MAGIC: u32 = 0xd00d_feed;
const FDT_VERSION: u32 = 17;
const FDT_LAST_COMP_VERSION: u32 = 16;
const FDT_BEGIN_NODE: u32 = 1;
const FDT_END_NODE: u32 = 2;
const FDT_PROP: u32 = 3;
const FDT_END: u32 = 9;
const HEADER_SIZE: usize = 40;
const RESERVATION_MAP_SIZE: usize = 16;
const VIRTIO_BLOCK_GPA: usize = 0x1000_1000;
const VIRTIO_MMIO_SIZE: usize = 0x1000;
const PLIC_PHANDLE: u32 = 1;

struct FdtBuilder {
    structure: Vec<u8>,
    strings: Vec<u8>,
}

impl FdtBuilder {
    fn new() -> Self {
        Self {
            structure: Vec::new(),
            strings: Vec::new(),
        }
    }

    fn begin_node(&mut self, name: &str) {
        push_u32(&mut self.structure, FDT_BEGIN_NODE);
        self.structure.extend_from_slice(name.as_bytes());
        self.structure.push(0);
        align_to_word(&mut self.structure);
    }

    fn end_node(&mut self) {
        push_u32(&mut self.structure, FDT_END_NODE);
    }

    fn property(&mut self, name: &str, value: &[u8]) {
        // PR #45 keeps the builder deliberately small: duplicate string-table
        // entries are valid FDT and avoid a map dependency during early boot.
        let name_offset = self.strings.len() as u32;
        self.strings.extend_from_slice(name.as_bytes());
        self.strings.push(0);
        push_u32(&mut self.structure, FDT_PROP);
        push_u32(&mut self.structure, value.len() as u32);
        push_u32(&mut self.structure, name_offset);
        self.structure.extend_from_slice(value);
        align_to_word(&mut self.structure);
    }

    fn property_u32(&mut self, name: &str, value: u32) {
        self.property(name, &value.to_be_bytes());
    }

    fn property_cells(&mut self, name: &str, cells: &[u32]) {
        let mut value = Vec::with_capacity(cells.len() * 4);
        for cell in cells {
            value.extend_from_slice(&cell.to_be_bytes());
        }
        self.property(name, &value);
    }

    fn finish(mut self, boot_hart_id: GuestHartId) -> Vec<u8> {
        push_u32(&mut self.structure, FDT_END);
        let structure_offset = HEADER_SIZE + RESERVATION_MAP_SIZE;
        let strings_offset = structure_offset + self.structure.len();
        let total_size = strings_offset + self.strings.len();
        assert!(total_size <= PAGE_SIZE, "Guest FDT exceeds its reserved page");

        let mut blob = Vec::with_capacity(total_size);
        for value in [
            FDT_MAGIC,
            total_size as u32,
            structure_offset as u32,
            strings_offset as u32,
            HEADER_SIZE as u32,
            FDT_VERSION,
            FDT_LAST_COMP_VERSION,
            boot_hart_id.index() as u32,
            self.strings.len() as u32,
            self.structure.len() as u32,
        ] {
            push_u32(&mut blob, value);
        }
        blob.resize(HEADER_SIZE + RESERVATION_MAP_SIZE, 0);
        blob.extend_from_slice(&self.structure);
        blob.extend_from_slice(&self.strings);
        blob
    }
}

/// Build and install one minimal DTB in the final page of this VM's RAM.
/// Returning the GPA lets the vCPU constructor place it in boot register `a1`.
pub fn install_guest_fdt(
    guest_memory: &GuestMemory,
    boot_hart_id: GuestHartId,
) -> usize {
    let mut fdt = FdtBuilder::new();
    fdt.begin_node("");
    fdt.property("compatible", b"hypocaust,guest\0");
    fdt.property_u32("#address-cells", 2);
    fdt.property_u32("#size-cells", 2);
    fdt.property_u32("hypocaust,vm-id", guest_memory.vm_id().index() as u32);

    fdt.begin_node("memory@80000000");
    fdt.property("device_type", b"memory\0");
    fdt.property_cells(
        "reg",
        &[
            0,
            guest_memory.guest_base() as u32,
            0,
            VM_MEMORY_SLOT_SIZE as u32,
        ],
    );
    fdt.end_node();

    fdt.begin_node("cpus");
    fdt.property_u32("#address-cells", 1);
    fdt.property_u32("#size-cells", 0);
    fdt.begin_node("cpu@0");
    fdt.property("device_type", b"cpu\0");
    fdt.property("compatible", b"riscv\0");
    fdt.property("status", b"okay\0");
    fdt.property_u32("reg", boot_hart_id.index() as u32);
    fdt.end_node();
    fdt.end_node();

    fdt.begin_node("soc");
    fdt.property("compatible", b"simple-bus\0");
    fdt.property_u32("#address-cells", 2);
    fdt.property_u32("#size-cells", 2);
    fdt.property("ranges", &[]);

    fdt.begin_node("plic@c000000");
    fdt.property("compatible", b"riscv,plic0\0");
    fdt.property_cells("reg", &[0, PLIC_GPA as u32, 0, PLIC_SIZE as u32]);
    fdt.property("interrupt-controller", &[]);
    fdt.property_u32("#interrupt-cells", 1);
    fdt.property_u32("phandle", PLIC_PHANDLE);
    fdt.property_u32("riscv,ndev", 31);
    fdt.end_node();

    fdt.begin_node("virtio_mmio@10001000");
    fdt.property("compatible", b"virtio,mmio\0");
    fdt.property_cells(
        "reg",
        &[0, VIRTIO_BLOCK_GPA as u32, 0, VIRTIO_MMIO_SIZE as u32],
    );
    fdt.property_u32("interrupt-parent", PLIC_PHANDLE);
    fdt.property_u32("interrupts", VIRTIO_BLOCK_IRQ);
    fdt.end_node();

    fdt.end_node();
    fdt.end_node();

    let blob = fdt.finish(boot_hart_id);
    let fdt_gpa = guest_memory.guest_end() - PAGE_SIZE;
    let fdt_hpa = guest_memory
        .translate_range(fdt_gpa, PAGE_SIZE)
        .expect("Guest FDT page is outside VM RAM");
    unsafe {
        core::ptr::write_bytes(fdt_hpa as *mut u8, 0, PAGE_SIZE);
        core::ptr::copy_nonoverlapping(blob.as_ptr(), fdt_hpa as *mut u8, blob.len());
    }
    // PR #45 parses the installed blob before publishing its GPA, catching a
    // malformed header or token stream even when the current example Guest
    // does not consume device-tree properties itself.
    let installed = unsafe { ::fdt::Fdt::from_ptr(fdt_hpa as *const u8) }
        .expect("synthesized Guest FDT is invalid");
    let memory = installed
        .memory()
        .regions()
        .next()
        .expect("synthesized Guest FDT has no memory region");
    assert_eq!(memory.starting_address as usize, guest_memory.guest_base());
    assert_eq!(memory.size, Some(VM_MEMORY_SLOT_SIZE));
    fdt_gpa
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn align_to_word(bytes: &mut Vec<u8>) {
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
}
