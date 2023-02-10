use crate::mm::MemoryRegion;



/// ref: https://github.com/mit-pdos/RVirt/blob/HEAD/src/context.rs
/// hypervisor emulated plic for guest
pub struct HostPlic {
    pub claim_clear: MemoryRegion<u32>
}

impl HostPlic {
    pub fn claim_and_clear(&mut self) -> u32 {
        let claim = self.claim_clear[0];
        // 设置内存屏障
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        self.claim_clear[0] = claim;
        claim
    }
}