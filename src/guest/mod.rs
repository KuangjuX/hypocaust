use crate::constants::csr::sie::{SEIE, STIE, SSIE, STIE_BIT};
use crate::constants::csr::sip::SSIP;
use crate::constants::csr::status::STATUS_SIE_BIT;
use crate::debug::PageDebug;
use crate::hypervisor::HYPERVISOR_MEMORY;
use crate::page_table::{VirtAddr, PhysPageNum, PageTable};
use crate::mm::{MemorySet, MapPermission};
use crate::hypervisor::trap::{TrapContext, trap_handler};
use crate::constants::layout::{TRAP_CONTEXT, kernel_stack_position, GUEST_KERNEL_VIRT_START};
use crate::constants::csr;
use crate::device_emu::VirtDevice;


pub mod switch;
pub mod context;
mod pmap;
pub mod sbi;

use context::TaskContext;
use riscv::addr::BitField;

pub use self::context::ShadowState;
pub use self::pmap::{ ShadowPageTables, PageTableRoot, gpa2hpa, hpa2gpa };

/// Guest Kernel 结构体
pub struct GuestKernel<P: PageTable + PageDebug> {
    pub memory_set: MemorySet<P>,
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: TaskContext,
    pub shadow_state: ShadowState<P>,
    pub guest_id: usize,
    /// Guest OS 是否运行在 S mode
    pub smode: bool,
    /// Virtual emulated device in qemu
    pub virt_device: VirtDevice,
}

impl<P> GuestKernel<P> where P: PageDebug + PageTable {
    pub fn new(memory_set: MemorySet<P>, guest_id: usize) -> Self {
        // 获取中断上下文的物理地址
        let mut hypervisor_memory = HYPERVISOR_MEMORY.exclusive_access();
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 获取内核栈地址
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(guest_id);
        // 将内核栈地址进行映射
        hypervisor_memory.insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let mut guest_kernel = Self { 
            memory_set,
            trap_cx_ppn,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            shadow_state: ShadowState::new(),
            guest_id,
            smode: true,
            virt_device: VirtDevice::new(guest_id), 
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
            hypervisor_memory.token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        guest_kernel
    }

    /// 根据 `PageTableRoot` mode 来获取对应的 shadow page table token
    pub fn get_user_token(&self) -> usize {
        match self.shadow() {
            PageTableRoot::GPA => self.memory_set.token(), 
            PageTableRoot::GVA => if self.shadow_state.csrs.satp == self.shadow_state.shadow_page_tables.guest_satp.unwrap() 
                                    { self.shadow_state.shadow_page_tables.page_tables[1].unwrap() }
                                    else{ self.shadow_state.shadow_page_tables.page_tables[2].unwrap() },
            PageTableRoot::UVA => self.shadow_state.shadow_page_tables.page_tables[2].unwrap(),  
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

