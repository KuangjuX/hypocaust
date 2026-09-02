mod uart;
mod plic;
mod virtio;
use alloc::sync::Arc;

use crate::guest::GuestMemory;
use crate::identity::VmId;
pub use uart::Uart;
// PR #16 (fix-bug/modern-rust-toolchain): keep device types available to the
// API even when a particular guest configuration does not construct them.
#[allow(unused_imports)]
pub use plic::VirtualPlic;
pub use plic::VIRTIO_BLOCK_IRQ;
pub use virtio::VirtIO;

const QEMU_TEST_GPA: usize = 0x0010_0000;
const QEMU_TEST_HPA: usize = 0x0010_0000;
const QEMU_TEST_SIZE: usize = 0x1000;
const VIRTIO_BLOCK_GPA: usize = 0x1000_1000;
const VIRTIO_BLOCK_HPA: usize = 0x1000_1000;

/// PR #39 (`feature/per-vm-device-bus`) is the VM-owned MMIO routing boundary.
/// All vCPUs in one VM therefore observe the same device and queue state.
pub struct DeviceBus {
    guest_memory: Arc<GuestMemory>,
    qemu_virt_tester: qemu_virt::QemuVirtTester,
    plic: VirtualPlic,
    virtio: VirtIO,
    pub uart: Uart,
}

impl DeviceBus {
    pub fn new(vm_id: VmId, guest_memory: Arc<GuestMemory>) -> Self {
        // PR #39 deliberately rejects implicit sharing of the one physical
        // QEMU device set. Later VM configurations must choose emulation or an
        // explicitly assigned Host backend instead of aliasing passthrough.
        assert_eq!(
            vm_id,
            VmId::new(0),
            "the default QEMU passthrough bus is reserved for VM 0",
        );
        let bus = Self {
            guest_memory,
            qemu_virt_tester: qemu_virt::QemuVirtTester::new(
                QEMU_TEST_GPA,
                QEMU_TEST_HPA,
                QEMU_TEST_SIZE,
            ),
            // PR #41 gives each VM an independent PLIC register file and
            // pending/claim state instead of exposing the Host controller.
            plic: VirtualPlic::new(),
            // PR #39 records Guest and Host addresses separately. VM 0 keeps
            // today's identity mapping until configurable backends are added.
            virtio: VirtIO::new(VIRTIO_BLOCK_GPA, VIRTIO_BLOCK_HPA),
            uart: Uart::new(vm_id),
        };
        assert!(
            !bus.virtio.contains(QEMU_TEST_GPA)
                && !bus.qemu_virt_tester.contains(VIRTIO_BLOCK_GPA),
            "VM device regions overlap",
        );
        plic::self_test();
        bus
    }

    /// PR #39 centralizes the MMIO membership test instead of using global
    /// address predicates that cannot distinguish one VM's devices.
    pub fn contains(&self, guest_address: usize) -> bool {
        self.virtio.contains(guest_address)
            || self.plic.contains(guest_address)
            || self.qemu_virt_tester.contains(guest_address)
    }

    /// PR #39 performs a 32-bit read only when this VM owns the address.
    pub fn read_u32(&mut self, guest_address: usize) -> Option<u32> {
        if self.virtio.contains(guest_address) {
            return Some(self.virtio.read(guest_address));
        }
        if self.qemu_virt_tester.contains(guest_address) {
            return Some(self.qemu_virt_tester.read(guest_address));
        }
        if self.plic.contains(guest_address) {
            return self.plic.read_u32(guest_address);
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
        if self.qemu_virt_tester.contains(guest_address) {
            self.qemu_virt_tester.write(guest_address, value);
            return true;
        }
        if self.plic.contains(guest_address) {
            return self.plic.write_u32(guest_address, value);
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
                htracking!(
                    "async VirtIO notifications={} completions={}",
                    notifications,
                    completions,
                );
            }
        }
        self.plic.has_interrupt(context)
    }

    pub fn virtio_progress(&self) -> (usize, usize) {
        self.virtio.progress()
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
