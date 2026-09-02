//! Strong identities for the VM runtime.
//!
//! `feature/vm-vcpu-identities` prevents VM ownership, virtual CPU identity,
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
    /// `feature/vm-vcpu-identities` assigns IDs globally across all VMs so a
    /// Host kernel-stack slot never aliases a vCPU owned by another VM.
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
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
}
