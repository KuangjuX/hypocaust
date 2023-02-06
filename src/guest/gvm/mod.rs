use crate::page_table::{PhysAddr, PhysPageNum, VirtPageNum,  PageTableSv39, PageTable};

use super::pmap::gpt2spt;

#[allow(unused)]
pub type GuestPhysAddr = PhysAddr;
#[allow(unused)]
pub type HostPhysAddr = PhysAddr;
#[allow(unused)]
pub type GuestPhysPageNum = PhysPageNum;
#[allow(unused)]
pub type HostPhysPageNum = PhysPageNum;
#[allow(unused)]
pub type GuestVirtPageNum = VirtPageNum;

#[allow(unused)]
pub trait ShadowPageTable: PageTable {
    fn gpt_for_spt(
        guest_root_page_table_vpn: GuestVirtPageNum,
        hart_id: usize 
    ) -> Self;
}

#[allow(unused)]
pub type ShadowPageTableSv39 = PageTableSv39;

impl ShadowPageTable for ShadowPageTableSv39 {
    fn gpt_for_spt(
        guest_root_page_table_vpn: GuestVirtPageNum,
        hart_id: usize     
    ) -> Self {
        let va = guest_root_page_table_vpn.0 << 12;
        let root_spt = gpt2spt(va, hart_id);
        Self::from_ppn(PhysPageNum::from(root_spt >> 12))
    }
}




pub trait GuestPhysMemorySetTrait: core::fmt::Debug + Send + Sync {

}