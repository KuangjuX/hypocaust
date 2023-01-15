use alloc::collections::btree_map::BTreeMap;

use crate::mm::{VirtPageNum};
/// 影子页表是从 GVA 到 HVA 的直接映射
pub struct ShadowPageTable {
    pgt: BTreeMap<VirtPageNum, VirtPageNum>
}

impl ShadowPageTable {
    pub const fn new() -> Self {
        Self { 
            pgt: BTreeMap::new()
        }
    }
}