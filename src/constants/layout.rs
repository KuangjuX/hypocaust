//! Constants used in rCore

pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 4;
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
pub const MEMORY_START: usize = 0x80000000;
pub const MEMORY_END: usize = 0x88000000;
pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_SIZE_BITS: usize = 0xc;

/// 每个内核拥有 128 M 的空间
pub const KERNEL_SPACE: usize = 128 * 1024 * 1024;

pub const SPT_PA_START_1: usize = 0x10000_0000;
pub const SPT_PA_END_1: usize = SPT_PA_START_1 + KERNEL_SPACE;

// 客户操作系统内存映射
pub const GUEST_KERNEL_PHY_START_1: usize = 0x8800_0000;
pub const GUEST_KERNEL_PHY_END_1: usize = 0x9000_0000;
pub const GUEST_KERNEL_VIRT_START: usize = 0x8000_0000;
pub const GUEST_KERNEL_VIRT_END: usize = 0x8800_0000;

pub const GUEST_KERNEL_PHY_START_2: usize = GUEST_KERNEL_PHY_START_1 + KERNEL_SPACE;
pub const GUEST_KERNEL_VIRT_START_2: usize = 0x8000_0000;

pub const GUEST_KERNEL_PHY_START_3: usize = GUEST_KERNEL_PHY_START_2 + KERNEL_SPACE;
pub const GUEST_KERNEL_VIRT_START_3: usize = 0x8000_0000;

pub const GUEST_KERNEL_OFFSET_1: usize = 0x800_0000;

/// 测试内核的跳板页和 Trap Context 的地址
pub const GUEST_MAX_VA: usize = 1 << (9 + 9 + 9 + 12 - 1);
pub const GUEST_TRAMPOLINE: usize = GUEST_MAX_VA - PAGE_SIZE;
pub const GUEST_TRAP_CONTEXT: usize = GUEST_TRAMPOLINE - PAGE_SIZE;


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
