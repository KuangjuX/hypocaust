use alloc::vec::Vec;
use crate::mm::{PageTable, VirtPageNum, PageTableEntry};
struct UserShadowPageTable {
    process_id: usize,
    pgt: PageTable
}
/// 影子页表是从 GVA 到 HVA 的直接映射
pub struct ShadowPageTable {
    guest_shadow_pgt: Option<PageTable>,
    user_shadow_pgt: Option<Vec<PageTable>>
}

impl ShadowPageTable {
    pub const fn new() -> Self {
        Self { 
            guest_shadow_pgt: None, 
            user_shadow_pgt: None
         }
    }

    pub fn guest_shadow_pgt(&self) -> Option<&PageTable> {
        if let Some(pgt) = &self.guest_shadow_pgt {
            Some(pgt)
        }else{ None }
    }

    pub fn replace_guest_pgt(&mut self, pgt: PageTable) -> Option<PageTable> {
        self.guest_shadow_pgt.replace(pgt)
    }


}