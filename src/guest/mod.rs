use crate::page_table::{MemorySet, VirtAddr, KERNEL_SPACE, MapPermission, PhysPageNum};
use crate::trap::{TrapContext, trap_handler};
use crate::constants::layout::{TRAP_CONTEXT, kernel_stack_position, GUEST_KERNEL_VIRT_START_1};
use crate::constants::csr;


mod switch;
mod context;
mod pmap;
mod virtirq;
mod virtdevice;

use context::TaskContext;
use alloc::vec::Vec;
use switch::__switch;
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;
use virtdevice::VirtDevice;


pub use self::context::ShadowState;
pub use self::pmap::{ ShadowPageTables, ShadowPageTable };
use self::pmap::PageTableRoot;



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
    hdebug!("run guest kernel......");
    // before this, we should drop local variables that must be dropped manually
    unsafe {
        __switch(&mut _unused as *mut _, task_cx_ptr);
    }
    panic!("unreachable in run_first_task!");
}

pub fn current_run_id() -> usize {
    let id = GUEST_KERNEL_MANAGER.inner.exclusive_access().run_id;
    id
}

pub fn current_user_token() -> usize {
    let id = GUEST_KERNEL_MANAGER.inner.exclusive_access().run_id;
    GUEST_KERNEL_MANAGER.inner.exclusive_access().kernels[id].get_user_token()
}

pub fn current_trap_context_ppn() -> PhysPageNum {
    let id = GUEST_KERNEL_MANAGER.inner.exclusive_access().run_id;
    let kernel_memory = &GUEST_KERNEL_MANAGER.inner.exclusive_access().kernels[id].memory;
    let trap_context: VirtAddr = TRAP_CONTEXT.into();
    let trap_context_ppn= kernel_memory.translate(trap_context.floor()).unwrap().ppn();
    trap_context_ppn
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    let trap_context_ppn = current_trap_context_ppn();
    trap_context_ppn.get_mut() 
}




/// Guest Kernel 结构体
pub struct GuestKernel {
    /// guest kernel 内存映射，从 GPA -> HVA 转换
    pub memory: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: TaskContext,
    pub shadow_state: ShadowState,
    pub index: usize,
    /// Guest OS 是否运行在 S mode
    pub smode: bool,
    /// Virtual emulated device in qemu
    pub virt_device: VirtDevice
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
        let mut guest_kernel = Self { 
            memory,
            trap_cx_ppn,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            shadow_state: ShadowState::new(),
            index,
            smode: true,
            virt_device: VirtDevice::new()
        };
        // 设置 Guest OS `sstatus` 的 `SPP`
        let mut sstatus = riscv::register::sstatus::read();
        sstatus.set_spp(riscv::register::sstatus::SPP::Supervisor);
        guest_kernel.shadow_state.write_sstatus(sstatus.bits());
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
        match self.shadow() {
            PageTableRoot::GPA => { self.memory.token() }
            PageTableRoot::GVA | PageTableRoot::UVA => { 
                if let Some(spt) = self.shadow_state.shadow_page_tables.find_shadow_page_table(self.shadow_state.get_satp()) {
                    return spt.page_table.token()
                }
                panic!()
            }
        }
    }

    /// 用来检查应当使用哪一级的影子页表
    pub fn shadow(&self) -> PageTableRoot {
        if (self.shadow_state.get_satp() >> 60) & 0xf == 0 {
            PageTableRoot::GPA
        }else if !self.shadow_state.smode() {
            PageTableRoot::UVA
        }else {
            PageTableRoot::GVA
        }
    }

    /// STEP:
    /// 1. VMM intercepts guest OS setting the virtual satp
    /// 2. VMM iterates over the guest page table, constructs a corresponding shadow page table
    /// 3. In shadow PT, every guest physical address is translated into host virtual address(machine address)
    /// 4. Finally, VMM sets the real satp to point to the shadow page table
    pub fn satp_handler(&mut self, satp: usize, sepc: usize) {
        if satp == 0 { panic!("sepc -> {:#x}", sepc); }
        match (satp >> 60) & 0xf {
            0 => { self.write_shadow_csr(csr::satp, satp)}
            8 => {
                // 获取 guest kernel 
                self.make_shadow_page_table(satp);
                self.write_shadow_csr(csr::satp, satp);
            }
            _ => { panic!("Atttempted to install page table with unsupported mode") }
        } 
    }

    pub fn read_shadow_csr(&self, csr: usize) -> usize {
        let shadow_state = &self.shadow_state;
        match csr {
            csr::sstatus => { shadow_state.get_sstatus() }
            csr::stvec => { shadow_state.get_stvec() }
            csr::sie => { shadow_state.get_sie() }
            csr::sscratch => { shadow_state.get_sscratch() }
            csr::sepc => { shadow_state.get_sepc() }
            csr::scause => { shadow_state.get_scause() }
            csr::stval => { shadow_state.get_stval() }
            csr::satp => { shadow_state.get_satp() }
            _ => { panic!("[hypervisor] Unrecognized") }
        }
    }

    pub fn write_shadow_csr(&mut self, csr: usize, val: usize) {
        let shadow_state = &mut self.shadow_state;
        match csr {
            csr::sstatus => { shadow_state.write_sstatus(val) }
            csr::stvec => { shadow_state.write_stvec(val) }
            csr::sie => { shadow_state.write_sie(val) }
            csr::sscratch => { shadow_state.write_sscratch(val) }
            csr::sepc => { shadow_state.write_sepc(val);}
            csr::scause => { shadow_state.write_scause(val) }
            csr::stval => { shadow_state.write_stval(val) }
            csr::satp => { shadow_state.write_satp(val) }
            _ => { panic!("[hypervisor] Unrecognized") }
        }
    }
    

}

