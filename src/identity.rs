//! Strong identities for the VM runtime.
//!
//! PR #34 (`feature/vm-vcpu-identities`) prevents VM ownership, virtual CPU identity,
//! and the physical hart currently executing a vCPU from sharing one `usize`.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VmId(usize);

impl VmId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VcpuId(usize);

impl VcpuId {
    /// PR #34 (`feature/vm-vcpu-identities`) assigns IDs globally across all VMs so a
    /// Host kernel-stack slot never aliases a vCPU owned by another VM.
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }
}

/// PR #43 (`feature/vm-runtime-config`) identifies a hart inside one Guest.
/// Unlike [`VcpuId`], this value is local to a VM and selects architectural
/// interfaces such as the Guest boot `a0` value and virtual PLIC context.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct GuestHartId(usize);

impl GuestHartId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }
}

/// PR #38 (`feature/multivcpu-scheduler`) names a schedulable vCPU without
/// conflating its VM ownership with the Host hart currently running it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VcpuKey {
    pub vm_id: VmId,
    pub vcpu_id: VcpuId,
}

impl VcpuKey {
    pub const fn new(vm_id: VmId, vcpu_id: VcpuId) -> Self {
        Self { vm_id, vcpu_id }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HartId(usize);

impl HartId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }

    pub const fn is_boot(self) -> bool {
        self.0 == 0
    }

    /// PR #38 reads the Host hart identity restored by the trap entry path.
    pub fn current() -> Self {
        let index: usize;
        unsafe { core::arch::asm!("mv {}, tp", out(reg) index) };
        Self(index)
    }
}
