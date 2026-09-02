use crate::constants::layout::{
    GUEST_KERNEL_VIRT_START, MAX_VM_MEMORY_SLOTS, SHADOW_PAGE_TABLES_HOST_START,
    VM_MEMORY_HOST_START, VM_MEMORY_SLOT_SIZE,
};
use crate::identity::VmId;

/// PR #36 (`feature/vm-guest-memory`) is the checked translation boundary for
/// one VM's RAM. Callers cannot derive a Host address from a physical hart ID.
#[derive(Debug)]
pub struct GuestMemory {
    vm_id: VmId,
    guest_base: usize,
    host_base: usize,
    shadow_base: usize,
    len: usize,
}

impl GuestMemory {
    pub fn for_vm(vm_id: VmId) -> Option<Self> {
        if vm_id.index() >= MAX_VM_MEMORY_SLOTS {
            return None;
        }
        let slot_offset = vm_id.index().checked_mul(VM_MEMORY_SLOT_SIZE)?;
        Some(Self {
            vm_id,
            guest_base: GUEST_KERNEL_VIRT_START,
            host_base: VM_MEMORY_HOST_START.checked_add(slot_offset)?,
            shadow_base: SHADOW_PAGE_TABLES_HOST_START.checked_add(slot_offset)?,
            len: VM_MEMORY_SLOT_SIZE,
        })
    }

    pub const fn vm_id(&self) -> VmId {
        self.vm_id
    }

    pub const fn guest_base(&self) -> usize {
        self.guest_base
    }

    pub const fn guest_end(&self) -> usize {
        self.guest_base + self.len
    }

    pub const fn host_base(&self) -> usize {
        self.host_base
    }

    pub const fn host_end(&self) -> usize {
        self.host_base + self.len
    }

    /// Translate one GPA only when it belongs to this VM's RAM slot.
    pub fn gpa_to_hpa(&self, gpa: usize) -> Option<usize> {
        self.translate_range(gpa, 1)
    }

    /// PR #36 (`feature/vm-guest-memory`) validates the complete DMA or page
    /// table range, including integer overflow, before exposing a Host address.
    pub fn translate_range(&self, gpa: usize, len: usize) -> Option<usize> {
        let offset = gpa.checked_sub(self.guest_base)?;
        let end = offset.checked_add(len)?;
        if end > self.len {
            return None;
        }
        self.host_base.checked_add(offset)
    }

    pub fn hpa_to_gpa(&self, hpa: usize) -> Option<usize> {
        let offset = hpa.checked_sub(self.host_base)?;
        if offset >= self.len {
            return None;
        }
        self.guest_base.checked_add(offset)
    }

    /// Translate a Guest page-table GPA into the VM's private shadow slot.
    pub fn gpa_to_shadow_hpa(&self, gpa: usize) -> Option<usize> {
        self.translate_shadow_range(gpa, 1)
    }

    pub fn translate_shadow_range(&self, gpa: usize, len: usize) -> Option<usize> {
        let offset = gpa.checked_sub(self.guest_base)?;
        let end = offset.checked_add(len)?;
        if end > self.len {
            return None;
        }
        self.shadow_base.checked_add(offset)
    }

    pub fn contains_gpa(&self, gpa: usize) -> bool {
        gpa >= self.guest_base && gpa < self.guest_end()
    }
}
