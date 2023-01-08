//! Constants used in rCore

pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
pub const MEMORY_END: usize = 0x80800000;
pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_SIZE_BITS: usize = 0xc;

// 客户操作系统内存映射
pub const GUEST_KERNEL_PHY_START: usize = 0xC0000000;
pub const GUEST_KERNEL_VIRT_STRAT: usize = 0xFFFFFFFFC0000000;
pub const MAX_GUEST_KERNEL_PHY_END: usize = 0xEFFFFFFF;
pub const MAX_GUEST_KERNEL_VIRT_END: usize = 0xFFFFFFFFEFFFFFFF;

/// 虚拟地址最高页为跳板页
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
/// 中断切换上下文
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;
/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

pub use crate::board::{CLOCK_FREQ, MMIO};
