//! Per-VM flattened device-tree construction.
//!
//! PR #45 (`feature/multi-guest-qemu`) gives every Guest a private hardware
//! description containing only its RAM, boot CPU, virtual PLIC, and assigned
//! VirtIO block frontend. The Host DTB is never exposed to a Guest.
//! PR #52 (`feature/linux-guest-fdt`) extends that private description with the
//! architectural properties Linux consumes during early RISC-V boot.

use alloc::vec::Vec;

use crate::constants::layout::CLOCK_FREQ;
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
const CPU_INTC_PHANDLE: u32 = 2;
const MACHINE_EXTERNAL_INTERRUPT: u32 = 11;
const SUPERVISOR_EXTERNAL_INTERRUPT: u32 = 9;

/// Optional Linux boot data published through the Guest `/chosen` node.
///
/// PR #52 keeps boot policy outside the FDT builder: the xv6 example can use
/// an empty command line today, while the Linux example can add its console and
/// rootfs arguments without receiving a copy of the Host device tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestFdtConfig<'a> {
    pub boot_hart_id: GuestHartId,
    pub bootargs: &'a str,
    pub initrd: Option<GuestInitrdRange>,
}

impl<'a> GuestFdtConfig<'a> {
    pub const fn new(boot_hart_id: GuestHartId) -> Self {
        Self {
            boot_hart_id,
            bootargs: "",
            initrd: None,
        }
    }

    pub const fn linux(
        boot_hart_id: GuestHartId,
        bootargs: &'a str,
        initrd: Option<GuestInitrdRange>,
    ) -> Self {
        Self {
            boot_hart_id,
            bootargs,
            initrd,
        }
    }
}

/// Half-open Guest-physical interval containing a Linux initramfs image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestInitrdRange {
    pub start_gpa: usize,
    pub end_gpa: usize,
}

/// Install one Linux initramfs immediately below the reserved Guest FDT page.
///
/// PR #62 (`feature/linux-initramfs-boot`) copies the same immutable build
/// artifact into each VM's private RAM slot. The returned physical range is
/// suitable for `/chosen/linux,initrd-{start,end}` and never aliases the FDT.
pub fn install_guest_initrd(
    guest_memory: &GuestMemory,
    initrd: &[u8],
) -> GuestInitrdRange {
    assert!(!initrd.is_empty(), "Linux initramfs is empty");
    let fdt_gpa = guest_memory.guest_end() - PAGE_SIZE;
    let unaligned_start = fdt_gpa
        .checked_sub(initrd.len())
        .expect("Linux initramfs does not fit below the Guest FDT");
    let start_gpa = unaligned_start & !(PAGE_SIZE - 1);
    let end_gpa = start_gpa
        .checked_add(initrd.len())
        .expect("Linux initramfs range overflow");
    let start_hpa = guest_memory
        .translate_range(start_gpa, initrd.len())
        .expect("Linux initramfs is outside VM RAM");
    unsafe {
        core::ptr::copy_nonoverlapping(initrd.as_ptr(), start_hpa as *mut u8, initrd.len());
    }
    GuestInitrdRange {
        start_gpa,
        end_gpa,
    }
}

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

    fn property_string(&mut self, name: &str, value: &str) {
        // PR #52 centralizes the required FDT NUL terminator so configurable
        // Linux boot arguments cannot accidentally produce a malformed string.
        assert!(
            !value.as_bytes().contains(&0),
            "FDT string contains a NUL byte"
        );
        let mut bytes = Vec::with_capacity(value.len() + 1);
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
        self.property(name, &bytes);
    }

    fn property_cells(&mut self, name: &str, cells: &[u32]) {
        let mut value = Vec::with_capacity(cells.len() * 4);
        for cell in cells {
            value.extend_from_slice(&cell.to_be_bytes());
        }
        self.property(name, &value);
    }

    fn property_address(&mut self, name: &str, address: usize) {
        self.property_cells(name, &[(address >> 32) as u32, address as u32]);
    }

    fn finish(mut self, boot_hart_id: GuestHartId) -> Vec<u8> {
        push_u32(&mut self.structure, FDT_END);
        let structure_offset = HEADER_SIZE + RESERVATION_MAP_SIZE;
        let strings_offset = structure_offset + self.structure.len();
        let total_size = strings_offset + self.strings.len();
        assert!(
            total_size <= PAGE_SIZE,
            "Guest FDT exceeds its reserved page"
        );

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
pub fn install_guest_fdt(guest_memory: &GuestMemory, boot_hart_id: GuestHartId) -> usize {
    install_configured_guest_fdt(guest_memory, GuestFdtConfig::new(boot_hart_id))
}

/// Build and install a Linux-compatible, per-VM device tree.
///
/// PR #52 publishes only virtual hardware owned by this VM and validates any
/// initramfs interval before Linux is allowed to discover it.
pub fn install_configured_guest_fdt(
    guest_memory: &GuestMemory,
    config: GuestFdtConfig<'_>,
) -> usize {
    if let Some(initrd) = config.initrd {
        assert!(
            initrd.start_gpa < initrd.end_gpa,
            "Guest initrd range is empty"
        );
        assert!(
            guest_memory
                .translate_range(initrd.start_gpa, initrd.end_gpa - initrd.start_gpa)
                .is_some(),
            "Guest initrd range is outside VM RAM",
        );
        assert!(
            initrd.end_gpa <= guest_memory.guest_end() - PAGE_SIZE,
            "Guest initrd overlaps the reserved FDT page",
        );
    }

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
    // PR #52 tells Linux how the RISC-V `time` counter maps to seconds. This
    // frequency is a platform property and must match the Host QEMU board.
    fdt.property_u32("timebase-frequency", CLOCK_FREQ as u32);
    fdt.begin_node("cpu@0");
    fdt.property("device_type", b"cpu\0");
    fdt.property("compatible", b"riscv\0");
    fdt.property("status", b"okay\0");
    fdt.property_u32("reg", config.boot_hart_id.index() as u32);
    // PR #52 advertises only architectural state Hypocaust currently preserves
    // across exits. In particular, F/D are omitted until vCPU FP state exists.
    fdt.property("riscv,isa", b"rv64imac_zicsr_zifencei\0");
    fdt.property("mmu-type", b"riscv,sv39\0");

    fdt.begin_node("interrupt-controller");
    fdt.property("compatible", b"riscv,cpu-intc\0");
    fdt.property("interrupt-controller", &[]);
    fdt.property_u32("#interrupt-cells", 1);
    fdt.property_u32("phandle", CPU_INTC_PHANDLE);
    fdt.end_node();
    fdt.end_node();
    fdt.end_node();

    fdt.begin_node("chosen");
    fdt.property_string("bootargs", config.bootargs);
    if let Some(initrd) = config.initrd {
        fdt.property_address("linux,initrd-start", initrd.start_gpa);
        fdt.property_address("linux,initrd-end", initrd.end_gpa);
    }
    fdt.end_node();

    fdt.begin_node("soc");
    fdt.property("compatible", b"simple-bus\0");
    fdt.property_u32("#address-cells", 2);
    fdt.property_u32("#size-cells", 2);
    fdt.property("ranges", &[]);

    fdt.begin_node("plic@c000000");
    fdt.property("compatible", b"sifive,plic-1.0.0\0riscv,plic0\0");
    fdt.property_cells("reg", &[0, PLIC_GPA as u32, 0, PLIC_SIZE as u32]);
    fdt.property("interrupt-controller", &[]);
    fdt.property_u32("#interrupt-cells", 1);
    fdt.property_u32("phandle", PLIC_PHANDLE);
    fdt.property_u32("riscv,ndev", 31);
    // PR #69 (`fix-bug/linux-plic-context-topology`) preserves QEMU's context
    // numbering in the virtual topology. Linux skips the inaccessible M-mode
    // context 0 and binds S-mode to the emulated context 1 register windows.
    fdt.property_cells(
        "interrupts-extended",
        &[
            CPU_INTC_PHANDLE,
            MACHINE_EXTERNAL_INTERRUPT,
            CPU_INTC_PHANDLE,
            SUPERVISOR_EXTERNAL_INTERRUPT,
        ],
    );
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

    let blob = fdt.finish(config.boot_hart_id);
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

    // PR #52 treats Linux-critical properties as a boot-time contract. A
    // builder regression fails in Hypocaust instead of becoming a silent Linux
    // hang before the console is available.
    let cpus = installed
        .find_node("/cpus")
        .expect("Guest FDT has no /cpus node");
    assert_eq!(
        cpus.property("timebase-frequency")
            .and_then(|value| value.as_usize()),
        Some(CLOCK_FREQ),
    );
    let cpu = installed
        .find_node("/cpus/cpu@0")
        .expect("Guest FDT has no CPU");
    assert_eq!(
        cpu.property("riscv,isa").and_then(|value| value.as_str()),
        Some("rv64imac_zicsr_zifencei"),
    );
    assert_eq!(
        cpu.property("mmu-type").and_then(|value| value.as_str()),
        Some("riscv,sv39"),
    );
    assert!(installed
        .find_node("/cpus/cpu@0/interrupt-controller")
        .is_some());
    let plic = installed
        .find_node("/soc/plic@c000000")
        .expect("Guest FDT has no PLIC");
    // PR #69 locks the QEMU-compatible context order into the serialized FDT:
    // M-external context 0 precedes S-external context 1 for Guest hart 0.
    let contexts = plic
        .property("interrupts-extended")
        .expect("Guest PLIC has no interrupt contexts");
    assert_eq!(
        contexts.value,
        &[
            0, 0, 0, CPU_INTC_PHANDLE as u8,
            0, 0, 0, MACHINE_EXTERNAL_INTERRUPT as u8,
            0, 0, 0, CPU_INTC_PHANDLE as u8,
            0, 0, 0, SUPERVISOR_EXTERNAL_INTERRUPT as u8,
        ],
    );
    let chosen = installed
        .find_node("/chosen")
        .expect("Guest FDT has no /chosen node");
    assert_eq!(
        chosen.property("bootargs").and_then(|value| value.as_str()),
        Some(config.bootargs),
    );
    if let Some(initrd) = config.initrd {
        // PR #62 verifies the exact half-open interval after serialization so
        // a cell-width or end-address regression cannot silently corrupt boot.
        assert_eq!(
            chosen
                .property("linux,initrd-start")
                .and_then(|value| value.as_usize()),
            Some(initrd.start_gpa),
        );
        assert_eq!(
            chosen
                .property("linux,initrd-end")
                .and_then(|value| value.as_usize()),
            Some(initrd.end_gpa),
        );
    }
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
