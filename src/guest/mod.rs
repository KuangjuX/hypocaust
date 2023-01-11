use crate::{mm::{MemorySet, VirtAddr, KERNEL_SPACE, MapPermission, PhysPageNum},  trap::{TrapContext, trap_handler}, config::{TRAP_CONTEXT, kernel_stack_position, GUEST_KERNEL_VIRT_START_1}, task::TaskContext};


mod switch;
mod context;
use alloc::vec::Vec;
use switch::__switch;
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;


lazy_static! {
    pub static ref GUEST_KERNEL_MANAGER: GuestKernelManager = GuestKernelManager::new();
}

pub struct GuestKernelManager {
    pub inner: UPSafeCell<GuestKernelManagerInner>
}

pub struct GuestKernelManagerInner {
    pub kernels: Vec<GuestKernel>,
    pub run_id: usize
}

impl GuestKernelManager {
    pub fn new() -> Self {
        Self {
           inner: unsafe{
            UPSafeCell::new(
                GuestKernelManagerInner{
                    kernels: Vec::new(),
                    run_id: 0
                }
            )
           }
        }
    }

    pub fn push(&self, kernel: GuestKernel) {
        self.inner.exclusive_access().kernels.push(kernel)
    }
}

pub fn run_guest_kernel() -> ! {
    let inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
    let id = inner.run_id;
    let guest_kernel = &inner.kernels[id];
    let task_cx_ptr = &guest_kernel.task_cx as *const TaskContext;
    drop(inner);
    let mut _unused = TaskContext::zero_init();
    // before this, we should drop local variables that must be dropped manually
    unsafe {
        __switch(&mut _unused as *mut _, task_cx_ptr);
    }
    panic!("unreachable in run_first_task!");
}

pub fn current_user_token() -> usize {
    let id = GUEST_KERNEL_MANAGER.inner.exclusive_access().run_id;
    GUEST_KERNEL_MANAGER.inner.exclusive_access().kernels[id].get_user_token()
}

/// Guest Kernel 结构体
pub struct GuestKernel {
    pub memory: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: TaskContext
}

impl GuestKernel {
    pub fn new(memory: MemorySet, index: usize) -> Self {
        // 获取中断上下文的物理地址
        let trap_cx_ppn = memory
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 获取内核栈地址
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(index);
        // 将内核栈地址进行映射
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let guest_kernel = Self { 
            memory,
            trap_cx_ppn,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top)
        };
        // 获取中断上下文的地址
        let trap_cx : &mut TrapContext = guest_kernel.trap_cx_ppn.get_mut();
        *trap_cx = TrapContext::app_init_context(
            GUEST_KERNEL_VIRT_START_1,
            0,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        guest_kernel
    }

    pub fn get_user_token(&self) -> usize {
        self.memory.token()
    }

}

