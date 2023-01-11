use crate::{mm::{MemorySet, VirtAddr, KERNEL_SPACE, MapPermission, PhysPageNum},  trap::{TrapContext, trap_handler}, config::{TRAP_CONTEXT, kernel_stack_position, GUEST_KERNEL_VIRT_START_1}, task::TaskContext};


mod switch;
use switch::__switch;
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

    pub fn run(&self) -> ! {
        let next_task_cx_ptr = &self.task_cx as *const TaskContext;
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr, self.get_user_token());
        }
        panic!("unreachable in run_first_task!");
    }
}

