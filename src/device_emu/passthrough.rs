//! IOMMU-protected physical-device assignment policy.
//!
//! PR #47 (`feature/iommu-passthrough-policy`) keeps real passthrough separate
//! from Hypocaust's mediated QEMU VirtIO backend. The types in this module are
//! the fail-closed boundary a hardware-specific IOMMU adapter must implement.

use arrayvec::ArrayVec;

use crate::constants::layout::{MAX_HOST_HARTS, PAGE_SIZE};
use crate::guest::GuestMemory;
use crate::identity::VmId;

use super::{MmioAssignment, PLIC_GPA, PLIC_SIZE, UART_GPA, UART_IRQ, UART_SIZE};

const MAX_PASSTHROUGH_DEVICES: usize = 16;

/// PR #47 identifies a physical requester independently from its MMIO base.
/// On a PCI platform this value can encode a segment and requester ID; a
/// platform without PCI can use its firmware-assigned device identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalDeviceId(u64);

impl PhysicalDeviceId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// PR #47 is an opaque handle to one hardware IOMMU translation domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IommuDomainId(usize);

impl IommuDomainId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }
}

/// PR #47 limits device DMA to a contiguous Guest-physical RAM aperture.
/// The platform adapter derives the Host address through `GuestMemory`; the
/// caller cannot inject an arbitrary Host physical DMA target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaAperture {
    pub guest_base: usize,
    pub size: usize,
}

impl DmaAperture {
    pub const fn new(guest_base: usize, size: usize) -> Self {
        Self { guest_base, size }
    }
}

/// PR #47 describes the interrupt-remapping entry required by passthrough.
/// A Host interrupt never becomes a Guest interrupt without an explicit
/// virtual PLIC source and VM-local vCPU context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterruptRemap {
    pub host_irq: u32,
    pub guest_source: u32,
    pub guest_context: usize,
}

impl InterruptRemap {
    pub const fn new(host_irq: u32, guest_source: u32, guest_context: usize) -> Self {
        Self {
            host_irq,
            guest_source,
            guest_context,
        }
    }
}

/// PR #47 collects every capability needed to assign one physical device.
/// Fields are immutable so ownership, DMA, and IRQ policy cannot diverge after
/// the platform adapter has activated the assignment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PassthroughAssignment {
    pub owner: VmId,
    pub device: PhysicalDeviceId,
    pub domain: IommuDomainId,
    pub mmio: MmioAssignment,
    pub dma: DmaAperture,
    pub interrupt: InterruptRemap,
}

impl PassthroughAssignment {
    pub const fn new(
        owner: VmId,
        device: PhysicalDeviceId,
        domain: IommuDomainId,
        mmio: MmioAssignment,
        dma: DmaAperture,
        interrupt: InterruptRemap,
    ) -> Self {
        Self {
            owner,
            device,
            domain,
            mmio,
            dma,
            interrupt,
        }
    }

    fn validate(self, guest_memory: &GuestMemory) -> Result<(), PassthroughConfigError> {
        if self.owner != guest_memory.vm_id() {
            return Err(PassthroughConfigError::WrongVm);
        }
        if self.mmio.size == 0 {
            return Err(PassthroughConfigError::EmptyMmio);
        }
        if self.mmio.guest_base.checked_add(self.mmio.size).is_none()
            || self.mmio.host_base.checked_add(self.mmio.size).is_none()
        {
            return Err(PassthroughConfigError::MmioOverflow);
        }
        if self.mmio.guest_base % PAGE_SIZE != 0
            || self.mmio.host_base % PAGE_SIZE != 0
            || self.mmio.size % PAGE_SIZE != 0
        {
            return Err(PassthroughConfigError::UnalignedMmio);
        }
        let virtual_plic = MmioAssignment::new(PLIC_GPA, PLIC_GPA, PLIC_SIZE);
        if self.mmio.overlaps(virtual_plic) {
            return Err(PassthroughConfigError::VirtualPlicOverlap);
        }
        // PR #71 keeps passthrough devices from replacing either half of the
        // VM-local UART contract: its MMIO aperture and virtual PLIC source.
        let virtual_uart = MmioAssignment::new(UART_GPA, UART_GPA, UART_SIZE);
        if self.mmio.overlaps(virtual_uart) {
            return Err(PassthroughConfigError::VirtualUartOverlap);
        }
        let mmio_end = self.mmio.guest_base + self.mmio.size;
        if self.mmio.guest_base < guest_memory.guest_end()
            && guest_memory.guest_base() < mmio_end
        {
            return Err(PassthroughConfigError::GuestRamOverlap);
        }
        if self.dma.size == 0 {
            return Err(PassthroughConfigError::EmptyDmaAperture);
        }
        if self.dma.guest_base % PAGE_SIZE != 0 || self.dma.size % PAGE_SIZE != 0 {
            return Err(PassthroughConfigError::UnalignedDmaAperture);
        }
        if guest_memory
            .translate_range(self.dma.guest_base, self.dma.size)
            .is_none()
        {
            return Err(PassthroughConfigError::DmaOutsideGuestRam);
        }
        if self.interrupt.host_irq == 0 || self.interrupt.guest_source == 0 {
            return Err(PassthroughConfigError::InvalidInterrupt);
        }
        if self.interrupt.guest_source == UART_IRQ {
            return Err(PassthroughConfigError::VirtualUartInterrupt);
        }
        if self.interrupt.guest_context >= MAX_HOST_HARTS {
            return Err(PassthroughConfigError::InvalidGuestContext);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PassthroughConfigError {
    WrongVm,
    EmptyMmio,
    MmioOverflow,
    UnalignedMmio,
    VirtualPlicOverlap,
    VirtualUartOverlap,
    GuestRamOverlap,
    EmptyDmaAperture,
    UnalignedDmaAperture,
    DmaOutsideGuestRam,
    InvalidInterrupt,
    VirtualUartInterrupt,
    InvalidGuestContext,
}

/// PR #47 requires a board-specific adapter to activate all isolation pieces
/// atomically. `activate` must leave the device quiesced and unassigned on an
/// error. `deactivate` must stop bus mastering and interrupt delivery before
/// removing IOMMU mappings; on error the assignment remains owned and active.
pub trait IommuPassthroughAdapter {
    type Error;

    fn activate(
        &mut self,
        assignment: &PassthroughAssignment,
        guest_memory: &GuestMemory,
    ) -> Result<(), Self::Error>;

    fn deactivate(&mut self, assignment: &PassthroughAssignment) -> Result<(), Self::Error>;
}

#[derive(Debug, Eq, PartialEq)]
pub enum PassthroughError<E> {
    Invalid(PassthroughConfigError),
    DeviceAlreadyAssigned,
    DomainOwnedByAnotherVm,
    HostMmioAlreadyAssigned,
    GuestMmioAlreadyAssigned,
    HostIrqAlreadyAssigned,
    GuestIrqAlreadyAssigned,
    CapacityExceeded,
    Adapter(E),
}

/// PR #47 is the ownership gate in front of a platform IOMMU adapter. It
/// prevents one physical requester, Host MMIO aperture, or IOMMU domain from
/// becoming an accidental cross-VM sharing channel.
pub struct PassthroughManager<A: IommuPassthroughAdapter> {
    adapter: A,
    active: ArrayVec<PassthroughAssignment, MAX_PASSTHROUGH_DEVICES>,
}

impl<A: IommuPassthroughAdapter> PassthroughManager<A> {
    pub fn new(adapter: A) -> Self {
        Self {
            adapter,
            active: ArrayVec::new(),
        }
    }

    pub fn assign(
        &mut self,
        assignment: PassthroughAssignment,
        guest_memory: &GuestMemory,
    ) -> Result<(), PassthroughError<A::Error>> {
        assignment
            .validate(guest_memory)
            .map_err(PassthroughError::Invalid)?;
        if self.active.len() == self.active.capacity() {
            return Err(PassthroughError::CapacityExceeded);
        }
        if self.active.iter().any(|active| active.device == assignment.device) {
            return Err(PassthroughError::DeviceAlreadyAssigned);
        }
        if self.active.iter().any(|active| {
            active.domain == assignment.domain && active.owner != assignment.owner
        }) {
            return Err(PassthroughError::DomainOwnedByAnotherVm);
        }
        if self
            .active
            .iter()
            .any(|active| host_mmio_overlaps(active.mmio, assignment.mmio))
        {
            return Err(PassthroughError::HostMmioAlreadyAssigned);
        }
        if self.active.iter().any(|active| {
            active.owner == assignment.owner && active.mmio.overlaps(assignment.mmio)
        }) {
            return Err(PassthroughError::GuestMmioAlreadyAssigned);
        }
        if self
            .active
            .iter()
            .any(|active| active.interrupt.host_irq == assignment.interrupt.host_irq)
        {
            return Err(PassthroughError::HostIrqAlreadyAssigned);
        }
        if self.active.iter().any(|active| {
            active.owner == assignment.owner
                && active.interrupt.guest_source == assignment.interrupt.guest_source
        }) {
            return Err(PassthroughError::GuestIrqAlreadyAssigned);
        }
        self.adapter
            .activate(&assignment, guest_memory)
            .map_err(PassthroughError::Adapter)?;
        self.active.push(assignment);
        Ok(())
    }

    pub fn revoke(
        &mut self,
        owner: VmId,
        device: PhysicalDeviceId,
    ) -> Result<bool, PassthroughError<A::Error>> {
        let Some(index) = self
            .active
            .iter()
            .position(|active| active.owner == owner && active.device == device)
        else {
            return Ok(false);
        };
        self.adapter
            .deactivate(&self.active[index])
            .map_err(PassthroughError::Adapter)?;
        self.active.swap_remove(index);
        Ok(true)
    }

    pub fn active_assignments(&self) -> &[PassthroughAssignment] {
        &self.active
    }
}

fn host_mmio_overlaps(left: MmioAssignment, right: MmioAssignment) -> bool {
    let left_end = left.host_base + left.size;
    let right_end = right.host_base + right.size;
    left.host_base < right_end && right.host_base < left_end
}

struct SelfTestAdapter {
    active: usize,
}

impl IommuPassthroughAdapter for SelfTestAdapter {
    type Error = ();

    fn activate(
        &mut self,
        _assignment: &PassthroughAssignment,
        _guest_memory: &GuestMemory,
    ) -> Result<(), Self::Error> {
        self.active += 1;
        Ok(())
    }

    fn deactivate(&mut self, _assignment: &PassthroughAssignment) -> Result<(), Self::Error> {
        self.active -= 1;
        Ok(())
    }
}

/// PR #47 checks ownership and IOMMU-domain isolation before VM construction.
pub(crate) fn self_test() {
    let vm0 = GuestMemory::for_vm(VmId::new(0)).expect("missing VM 0 memory");
    let vm1 = GuestMemory::for_vm(VmId::new(1)).expect("missing VM 1 memory");
    let assignment = PassthroughAssignment::new(
        VmId::new(0),
        PhysicalDeviceId::new(1),
        IommuDomainId::new(1),
        MmioAssignment::new(0x1000_1000, 0x2000_0000, 0x1000),
        DmaAperture::new(vm0.guest_base(), 0x1000),
        InterruptRemap::new(32, 1, 0),
    );
    // PR #71 verifies that neither the virtual UART page nor IRQ 10 can be
    // reassigned to a physical device by a later board adapter.
    let uart_mmio = PassthroughAssignment {
        mmio: MmioAssignment::new(UART_GPA, 0x2000_2000, PAGE_SIZE),
        ..assignment
    };
    assert_eq!(
        uart_mmio.validate(&vm0),
        Err(PassthroughConfigError::VirtualUartOverlap),
    );
    let uart_irq = PassthroughAssignment {
        interrupt: InterruptRemap::new(32, UART_IRQ, 0),
        ..assignment
    };
    assert_eq!(
        uart_irq.validate(&vm0),
        Err(PassthroughConfigError::VirtualUartInterrupt),
    );
    let mut manager = PassthroughManager::new(SelfTestAdapter { active: 0 });
    assert_eq!(manager.assign(assignment, &vm0), Ok(()));
    assert!(matches!(
        manager.assign(assignment, &vm0),
        Err(PassthroughError::DeviceAlreadyAssigned),
    ));

    let cross_vm_domain = PassthroughAssignment::new(
        VmId::new(1),
        PhysicalDeviceId::new(2),
        assignment.domain,
        MmioAssignment::new(0x1000_1000, 0x2000_1000, 0x1000),
        DmaAperture::new(vm1.guest_base(), 0x1000),
        InterruptRemap::new(33, 1, 0),
    );
    assert!(matches!(
        manager.assign(cross_vm_domain, &vm1),
        Err(PassthroughError::DomainOwnedByAnotherVm),
    ));
    assert_eq!(manager.active_assignments(), &[assignment]);
    assert_eq!(manager.revoke(VmId::new(0), assignment.device), Ok(true));
    assert!(manager.active_assignments().is_empty());
}
