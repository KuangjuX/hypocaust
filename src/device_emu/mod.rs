mod uart;
mod plic;
mod passthrough;
mod virtio;
use alloc::sync::Arc;

use crate::guest::GuestMemory;
use crate::constants::layout::PAGE_SIZE;
pub use uart::{Uart, UART_GPA, UART_IRQ, UART_SIZE};
pub(crate) use uart::self_test as console_self_test;
// PR #16 (fix-bug/modern-rust-toolchain): keep device types available to the
// API even when a particular guest configuration does not construct them.
#[allow(unused_imports)]
pub use plic::VirtualPlic;
pub use plic::{PLIC_GPA, PLIC_SIZE, VIRTIO_BLOCK_IRQ};
// PR #47 exposes the board-adapter contract before the QEMU board grows an
// IOMMU implementation; the mediated example intentionally does not use it.
#[allow(unused_imports)]
pub use passthrough::{
    DmaAperture, InterruptRemap, IommuDomainId, IommuPassthroughAdapter,
    PassthroughAssignment, PassthroughConfigError, PassthroughError,
    PassthroughManager, PhysicalDeviceId,
};
pub(crate) use passthrough::self_test as passthrough_self_test;
pub use virtio::VirtIO;

const QEMU_TEST_GPA: usize = 0x0010_0000;
const QEMU_TEST_HPA: usize = 0x0010_0000;
const QEMU_TEST_SIZE: usize = 0x1000;
const VIRTIO_BLOCK_GPA: usize = 0x1000_1000;
const VIRTIO_BLOCK_HPA: usize = 0x1000_1000;

/// PR #43 (`feature/vm-runtime-config`) describes one VM-visible MMIO window
/// and the explicitly assigned Host backend aperture that implements it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MmioAssignment {
    pub guest_base: usize,
    pub host_base: usize,
    pub size: usize,
}

impl MmioAssignment {
    pub const fn new(guest_base: usize, host_base: usize, size: usize) -> Self {
        Self {
            guest_base,
            host_base,
            size,
        }
    }

    fn guest_end(self) -> Option<usize> {
        self.guest_base.checked_add(self.size)
    }

    fn overlaps(self, other: Self) -> bool {
        let self_end = self.guest_end().expect("Guest MMIO range overflow");
        let other_end = other.guest_end().expect("Guest MMIO range overflow");
        self.guest_base < other_end && other.guest_base < self_end
    }
}

/// PR #43 makes physical backend assignment an explicit VM construction
/// decision. A later multi-Guest PR can bind the same Guest GPA to a different
/// Host VirtIO device without changing the Guest's device model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceBusConfig {
    pub virtio_block: MmioAssignment,
    pub qemu_test: Option<MmioAssignment>,
}

impl DeviceBusConfig {
    pub const fn new(
        virtio_block: MmioAssignment,
        qemu_test: Option<MmioAssignment>,
    ) -> Self {
        Self {
            virtio_block,
            qemu_test,
        }
    }

    /// Preserve the current single-VM QEMU layout through an explicit config.
    pub const fn qemu_default() -> Self {
        Self::new(
            MmioAssignment::new(VIRTIO_BLOCK_GPA, VIRTIO_BLOCK_HPA, 0x1000),
            Some(MmioAssignment::new(
                QEMU_TEST_GPA,
                QEMU_TEST_HPA,
                QEMU_TEST_SIZE,
            )),
        )
    }

    /// PR #45 (`feature/multi-guest-qemu`) gives one VM exclusive use of a
    /// discovered QEMU VirtIO backend while preserving the standard Guest GPA.
    pub const fn qemu_virtio_block(host_base: usize) -> Self {
        Self::new(
            MmioAssignment::new(VIRTIO_BLOCK_GPA, host_base, 0x1000),
            None,
        )
    }

    fn validate(self) {
        let plic = MmioAssignment::new(PLIC_GPA, PLIC_GPA, PLIC_SIZE);
        // PR #71 reserves the emulated UART aperture for every VM so a runtime
        // device assignment cannot silently shadow Linux's console registers.
        let uart = MmioAssignment::new(UART_GPA, UART_GPA, UART_SIZE);
        assert!(
            self.virtio_block.size >= 0x1000,
            "VirtIO MMIO assignment is smaller than one register aperture",
        );
        self.virtio_block
            .guest_end()
            .expect("VirtIO Guest MMIO range overflow");
        self.virtio_block
            .host_base
            .checked_add(self.virtio_block.size)
            .expect("VirtIO Host MMIO range overflow");
        assert!(
            !self.virtio_block.overlaps(plic),
            "VirtIO MMIO assignment overlaps the virtual PLIC",
        );
        assert!(
            !self.virtio_block.overlaps(uart),
            "VirtIO MMIO assignment overlaps the virtual UART",
        );
        if let Some(qemu_test) = self.qemu_test {
            assert!(qemu_test.size != 0, "QEMU test MMIO assignment is empty");
            qemu_test
                .guest_end()
                .expect("QEMU test Guest MMIO range overflow");
            qemu_test
                .host_base
                .checked_add(qemu_test.size)
                .expect("QEMU test Host MMIO range overflow");
            assert!(
                !self.virtio_block.overlaps(qemu_test),
                "VM device regions overlap",
            );
            assert!(
                !qemu_test.overlaps(plic),
                "QEMU test MMIO assignment overlaps the virtual PLIC",
            );
            assert!(
                !qemu_test.overlaps(uart),
                "QEMU test MMIO assignment overlaps the virtual UART",
            );
        }
    }
}

/// PR #39 (`feature/per-vm-device-bus`) is the VM-owned MMIO routing boundary.
/// All vCPUs in one VM therefore observe the same device and queue state.
pub struct DeviceBus {
    guest_memory: Arc<GuestMemory>,
    qemu_virt_tester: Option<qemu_virt::QemuVirtTester>,
    plic: VirtualPlic,
    virtio: VirtIO,
    pub uart: Uart,
}

impl DeviceBus {
    pub fn new(guest_memory: Arc<GuestMemory>, config: DeviceBusConfig) -> Self {
        config.validate();
        let vm_id = guest_memory.vm_id();
        let bus = Self {
            guest_memory,
            qemu_virt_tester: config.qemu_test.map(|assignment| {
                qemu_virt::QemuVirtTester::new(
                    assignment.guest_base,
                    assignment.host_base,
                    assignment.size,
                )
            }),
            // PR #41 gives each VM an independent PLIC register file and
            // pending/claim state instead of exposing the Host controller.
            plic: VirtualPlic::new(),
            // PR #43 consumes the explicit assignment instead of silently
            // sharing VM 0's physical device with every future VM.
            virtio: VirtIO::new(
                config.virtio_block.guest_base,
                config.virtio_block.host_base,
            ),
            uart: Uart::new(vm_id),
        };
        plic::self_test();
        bus
    }

    /// PR #39 centralizes the MMIO membership test instead of using global
    /// address predicates that cannot distinguish one VM's devices.
    pub fn contains(&self, guest_address: usize) -> bool {
        self.uart.contains(guest_address)
            || self.virtio.contains(guest_address)
            || self.plic.contains(guest_address)
            || self
                .qemu_virt_tester
                .as_ref()
                .is_some_and(|device| device.contains(guest_address))
    }

    /// PR #39 performs a 32-bit read only when this VM owns the address.
    pub fn read_u32(&mut self, guest_address: usize) -> Option<u32> {
        if self.virtio.contains(guest_address) {
            return Some(self.virtio.read(guest_address));
        }
        if let Some(device) = self
            .qemu_virt_tester
            .as_ref()
            .filter(|device| device.contains(guest_address))
        {
            return Some(device.read(guest_address));
        }
        if self.plic.contains(guest_address) {
            return self.plic.read_u32(guest_address);
        }
        None
    }

    /// PR #71 supplies byte MMIO for the NS16550A register file. Wider devices
    /// remain on the existing u32 bus contract so unsupported widths still fault.
    pub fn read_u8(&mut self, guest_address: usize) -> Option<u8> {
        if self.uart.contains(guest_address) {
            return self.uart.read_u8(guest_address);
        }
        None
    }

    /// PR #39 performs a 32-bit write only when this VM owns the address.
    pub fn write_u32(&mut self, guest_address: usize, value: u32) -> bool {
        if self.virtio.contains(guest_address) {
            let acknowledged = self.virtio.is_interrupt_ack(guest_address);
            self.virtio
                .write(guest_address, value, &self.guest_memory);
            if acknowledged {
                // PR #42 lowers only this VM's VirtIO PLIC line after the
                // physical backend has observed the Guest acknowledgement.
                self.plic.lower(VIRTIO_BLOCK_IRQ);
            }
            return true;
        }
        if let Some(device) = self
            .qemu_virt_tester
            .as_mut()
            .filter(|device| device.contains(guest_address))
        {
            device.write(guest_address, value);
            return true;
        }
        if self.plic.contains(guest_address) {
            return self.plic.write_u32(guest_address, value);
        }
        false
    }

    /// PR #71 keeps Guest serial writes in the VM-owned frontend and never
    /// grants direct access to the shared physical UART register page.
    pub fn write_u8(&mut self, guest_address: usize, value: u8) -> bool {
        if self.uart.contains(guest_address) {
            let handled = self.uart.write_u8(guest_address, value);
            if !self.uart.interrupt_pending() {
                self.plic.lower(UART_IRQ);
            }
            return handled;
        }
        false
    }

    /// PR #41 raises a source only in this VM's controller and reports whether
    /// the selected Guest context should receive a virtual external interrupt.
    pub fn raise_irq(&mut self, source: u32, context: usize) -> bool {
        self.plic.raise(source, context)
    }

    /// PR #41 deasserts a VM-local level-triggered device source.
    pub fn lower_irq(&mut self, source: u32) {
        self.plic.lower(source);
    }

    /// PR #41 exposes the PLIC context output that a later device backend maps
    /// to one target vCPU's virtual SEIP state.
    pub fn has_irq(&self, context: usize) -> bool {
        self.plic.has_interrupt(context)
    }

    /// PR #42 polls the asynchronous QEMU backend at a bounded Host timer
    /// boundary and raises this VM's VirtIO source on new used-ring entries.
    pub fn poll_async(&mut self, context: usize) -> bool {
        if self.virtio.poll_completions() {
            self.plic.raise(VIRTIO_BLOCK_IRQ, context);
            let (notifications, completions) = self.virtio.progress();
            // PR #42 samples powers of two so completion progress is visible
            // without turning normal block traffic into a serial-log flood.
            if completions.is_power_of_two() {
                // PR #45 labels progress by VM so concurrent backends remain
                // distinguishable even on the shared diagnostic console.
                htracking!(
                    "VM {} async VirtIO notifications={} completions={}",
                    self.guest_memory.vm_id().index(),
                    notifications,
                    completions,
                );
            }
        }
        // PR #71 multiplexes the focused Host input into this VM's receive FIFO
        // and exposes both receive-ready and transmit-empty through UART IRQ 10.
        self.uart.poll_input();
        if self.uart.interrupt_pending() {
            self.plic.raise(UART_IRQ, context);
        } else {
            self.plic.lower(UART_IRQ);
        }
        self.plic.has_interrupt(context)
    }

    pub fn virtio_progress(&self) -> (usize, usize) {
        self.virtio.progress()
    }

    /// PR #49 routes legacy SBI console output through this VM's buffered
    /// console frontend rather than writing directly to the Host UART.
    pub fn console_putchar(&mut self, value: usize) {
        self.uart.write_console_byte(value as u8);
    }

    /// PR #49 applies exclusive Host input focus at the VM-owned bus boundary.
    pub fn console_getchar(&mut self) -> usize {
        self.uart.read_console_byte()
    }

    /// PR #61 (`feature/sbi-dbcn-console`) validates the entire shared-memory
    /// range against this VM's RAM capability, then performs a bounded partial
    /// transfer as permitted by DBCN's non-blocking bulk operations.
    pub fn debug_console_write(
        &mut self,
        num_bytes: usize,
        base_addr_lo: usize,
        base_addr_hi: usize,
    ) -> Option<usize> {
        let (host_address, transfer_len) = self.debug_console_buffer(
            num_bytes,
            base_addr_lo,
            base_addr_hi,
        )?;
        let bytes = unsafe {
            core::slice::from_raw_parts(host_address as *const u8, transfer_len)
        };
        self.uart.write_console_bytes(bytes);
        Some(transfer_len)
    }

    /// PR #61 writes console input only into RAM owned by the calling VM. The
    /// returned byte count exposes an empty non-blocking read without using a
    /// sentinel value in Guest memory.
    pub fn debug_console_read(
        &mut self,
        num_bytes: usize,
        base_addr_lo: usize,
        base_addr_hi: usize,
    ) -> Option<usize> {
        let (host_address, transfer_len) = self.debug_console_buffer(
            num_bytes,
            base_addr_lo,
            base_addr_hi,
        )?;
        let bytes = unsafe {
            core::slice::from_raw_parts_mut(host_address as *mut u8, transfer_len)
        };
        Some(self.uart.read_console_bytes(bytes))
    }

    fn debug_console_buffer(
        &self,
        num_bytes: usize,
        base_addr_lo: usize,
        base_addr_hi: usize,
    ) -> Option<(usize, usize)> {
        // RV64 Linux supplies the physical address in the low XLEN word. A
        // nonzero high word cannot identify memory in Hypocaust's GPA space.
        if base_addr_hi != 0 {
            return None;
        }
        let host_address = self
            .guest_memory
            .translate_range(base_addr_lo, num_bytes)?;
        Some((host_address, num_bytes.min(PAGE_SIZE)))
    }
}



mod qemu_virt {
    use crate::mm::MemoryRegion;
    /// Software emulated qemu virt test
    pub struct QemuVirtTester {
        guest_base: usize,
        host_registers: MemoryRegion<u32>,
    }

    impl QemuVirtTester {
        pub fn new(guest_base: usize, host_base: usize, size: usize) -> Self {
            Self {
                guest_base,
                host_registers: MemoryRegion::new(host_base, size),
            }
        }

        pub fn contains(&self, addr: usize) -> bool {
            addr >= self.guest_base && addr - self.guest_base < self.host_registers.len()
        }

        pub fn read(&self, addr: usize) -> u32 {
            self.host_registers[self.host_address(addr)]
        }

        pub fn write(&mut self, addr: usize, value: u32) {
            let host_address = self.host_address(addr);
            self.host_registers[host_address] = value;
        }

        /// PR #39 translates the VM-visible register offset into the assigned
        /// Host MMIO aperture instead of treating both address spaces as one.
        fn host_address(&self, guest_address: usize) -> usize {
            assert!(self.contains(guest_address));
            self.host_registers.base() + (guest_address - self.guest_base)
        }
    }
}
