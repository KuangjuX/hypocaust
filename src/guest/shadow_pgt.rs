use alloc::collections::btree_map::BTreeMap;

use crate::mm::{VirtAddr, PhysAddr};
pub struct ShadowPageTable {
    pgt: BTreeMap<VirtAddr, PhysAddr>
}