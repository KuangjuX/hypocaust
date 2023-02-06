use crate::constants::csr::sie::{SEIE, STIE, SSIE, STIE_BIT};
use crate::constants::csr::sip::SSIP;
use crate::constants::csr::status::STATUS_SIE_BIT;
use crate::debug::PageDebug;
use crate::page_table::{VirtAddr, PhysPageNum, PageTable, PageTableSv39};
use crate::mm::{MemorySet, MapPermission, KERNEL_SPACE};
use crate::trap::{TrapContext, trap_handler};
use crate::constants::layout::{TRAP_CONTEXT, kernel_stack_position, GUEST_KERNEL_VIRT_START};
use crate::constants::csr;


mod switch;
mod context;
mod pmap;
mod virtirq;
mod virtdevice;
mod gvm;

use context::TaskContext;
use alloc::vec::Vec;
use riscv::addr::BitField;
use switch::__switch;
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;
use virtdevice::VirtDevice;


pub use self::context::ShadowState;
pub use self::pmap::{ ShadowPageTables, PageTableRoot, gpa2hpa};



lazy_static! {
    pub static ref GUEST_KERNEL_MANAGER: GuestKernelManager<PageTableSv39> = GuestKernelManager::new();
}

pub struct GuestKernelManager<P: PageTable + PageDebug> {
    pub inner: UPSafeCell<GuestKernelManagerInner<P>>
}

pub struct GuestKernelManagerInner<P: PageTable + PageDebug> {
    pub kernels: Vec<GuestKernel<P>>,
    pub run_id: usize
}

impl<P> GuestKernelManager<P> where P: PageDebug + PageTable {
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

    pub fn push(&self, kernel: GuestKernel<P>) {
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


pub fn current_user_token() -> usize {
    let id = GUEST_KERNEL_MANAGER.inner.exclusive_access().run_id;
    GUEST_KERNEL_MANAGER.inner.exclusive_access().kernels[id].get_user_token()
}

pub fn current_trap_context_ppn() -> PhysPageNum {
    let id = GUEST_KERNEL_MANAGER.inner.exclusive_access().run_id;
    let kernel_memory = &GUEST_KERNEL_MANAGER.inner.exclusive_access().kernels[id].memory_set;
    let trap_context: VirtAddr = TRAP_CONTEXT.into();
    let trap_context_ppn= kernel_memory.translate(trap_context.floor()).unwrap().ppn();
    trap_context_ppn
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    let trap_context_ppn = current_trap_context_ppn();
    trap_context_ppn.get_mut() 
}




/// Guest Kernel 结构体
pub struct GuestKernel<P: PageTable + PageDebug> {
    pub memory_set: MemorySet<P>,
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: TaskContext,
    pub shadow_state: ShadowState<P>,
    pub index: usize,
    /// Guest OS 是否运行在 S mode
    pub smode: bool,
    /// Virtual emulated device in qemu
    pub virt_device: VirtDevice,
}

impl<P> GuestKernel<P> where P: PageDebug + PageTable {
    pub fn new(memory_set: MemorySet<P>, index: usize) -> Self {
        // 获取中断上下文的物理地址
        let trap_cx_ppn = memory_set
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
            memory_set,
            trap_cx_ppn,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            shadow_state: ShadowState::new(),
            index,
            smode: true,
            virt_device: VirtDevice::new(), 
        };
        // 设置 Guest OS `sstatus` 的 `SPP`
        let mut sstatus = riscv::register::sstatus::read();
        sstatus.set_spp(riscv::register::sstatus::SPP::Supervisor);
        guest_kernel.shadow_state.csrs.sstatus = sstatus.bits();
        // 获取中断上下文的地址
        let trap_cx : &mut TrapContext = guest_kernel.trap_cx_ppn.get_mut();
        *trap_cx = TrapContext::app_init_context(
            GUEST_KERNEL_VIRT_START,
            0,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        guest_kernel
    }

    pub fn get_user_token(&self) -> usize {
        match self.shadow() {
            PageTableRoot::GPA => self.memory_set.token(),
            PageTableRoot::GVA => self.shadow_state.shadow_page_tables.page_tables[1].unwrap(),
            PageTableRoot::UVA => self.shadow_state.shadow_page_tables.page_tables[2].unwrap()
        }
    }

    /// 用来检查应当使用哪一级的影子页表
    pub fn shadow(&self) -> PageTableRoot {
        if (self.shadow_state.csrs.satp >> 60) & 0xf == 0 {
            PageTableRoot::GPA
        }else if !self.shadow_state.smode() {
            PageTableRoot::UVA
        }else {
            PageTableRoot::GVA
        }
    }

    pub fn get_csr(&self, csr: usize) -> usize {
        let shadow_state = &self.shadow_state;
        match csr {
            csr::sstatus => shadow_state.csrs.sstatus,
            csr::stvec => shadow_state.csrs.stvec,
            csr::sie => shadow_state.csrs.sie,
            csr::sscratch => shadow_state.csrs.sscratch,
            csr::sepc => shadow_state.csrs.sepc,
            csr::scause => shadow_state.csrs.scause,
            csr::stval => shadow_state.csrs.stval,
            csr::satp => shadow_state.csrs.satp,
            _ => unreachable!(),
        }
    }

    pub fn set_csr(&mut self, csr: usize, val: usize) {
        let shadow_state = &mut self.shadow_state;
        match csr {
            csr::sstatus => { 
                if val.get_bit(STATUS_SIE_BIT) {
                    // Enabling interruots might casue one to happen right away
                    shadow_state.interrupt = true;
                }
                shadow_state.csrs.sstatus  = val
             }
            csr::stvec => shadow_state.csrs.stvec = val,
            csr::sie => { 
                let value = val & (SEIE | STIE | SSIE);
                if !shadow_state.csrs.sie & value != 0{
                    shadow_state.interrupt = true;
                }
                if value.get_bit(STIE_BIT) {
                    unsafe{ riscv::register::sie::set_stimer() };
                }
                shadow_state.csrs.sie = val;
            }
            csr::sip => {
                if val & SSIP != 0 {
                    shadow_state.interrupt = true;
                }
                shadow_state.csrs.sip = (shadow_state.csrs.sip & !SSIP) | (val & SSIP);
            }
            csr::sscratch => shadow_state.csrs.sscratch = val,
            csr::sepc => shadow_state.csrs.sepc = val,
            csr::scause => shadow_state.csrs.scause = val,
            csr::stval => shadow_state.csrs.stval = val,
            csr::satp => { 
                let satp = val;
                match (satp >> 60) & 0xf {
                    0 => shadow_state.csrs.satp = satp, 
                    8 => {
                        // 获取 guest kernel 
                        shadow_state.csrs.satp = satp;
                        self.make_shadow_page_table(satp);
                    }
                    _ => panic!("Install page table with unsupported mode?") 
                }
            }
            _ => unreachable!()
        }
    }
    

}

