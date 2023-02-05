use riscv::addr::BitField;


use crate::trap::trap_return;
use crate::constants::csr::status::{STATUS_SIE_BIT, STATUS_SPIE_BIT, STATUS_SPP_BIT};
use crate::page_table::PageTable;
use crate::debug::PageDebug;
use super::pmap::ShadowPageTables;


pub struct ControlRegisters {
    // sedeleg: usize, -- Hard-wired to zero
    // sideleg: usize, -- Hard-wired to zero
    pub sstatus: usize,
    /// 中断使能寄存器
    pub sie: usize,
    /// 中断代理寄存器
    pub sip: usize,
    pub stvec: usize,
    // scounteren: usize, -- Hard-wired to zero
    pub sscratch: usize,
    pub sepc: usize,
    pub scause: usize,
    pub stval: usize,
    pub satp: usize,
    /// 用于设置 Guest OS 时钟中断
    pub mtimecmp: usize
}

impl ControlRegisters {
    pub const fn new() -> Self {
        Self {
            sstatus: 0,
            stvec: 0,
            sie: 0,
            sip: 0,
            sscratch: 0,
            sepc: 0,
            scause: 0,
            stval: 0,
            satp: 0,
            mtimecmp: core::usize::MAX
        }
    }
}

pub struct ShadowState<P: PageTable + PageDebug> {
    pub csrs: ControlRegisters,
    /// 影子页表
    pub shadow_page_tables: ShadowPageTables<P>,
    /// 是否发生中断
    pub interrupt: bool,
    /// 连续切换页表次数
    pub conseutive_satp_switch_count: usize
}

impl<P> ShadowState<P> where P: PageTable + PageDebug {
    pub const fn new() -> Self {
        Self {
            csrs: ControlRegisters::new(),
            shadow_page_tables: ShadowPageTables::new(),
            interrupt: false,
            conseutive_satp_switch_count: 0
        }
    }


    

    /// ref: riscv-privileged
    /// The `SPIE` bit indicates whether supervisor interrupts were enabled prior to
    /// trapping into supervisor mode. When a trap is taken into supervisor mode, `SPIE` is set 
    /// to 0. When an `SRET` instruction is executed, `SIE` is set to `SPIE`, then `SPIE` is set to 1.
    pub fn push_sie(&mut self) {
        self.csrs.sstatus.set_bit(STATUS_SPIE_BIT, self.csrs.sstatus.get_bit(STATUS_SIE_BIT));
        self.csrs.sstatus.set_bit(STATUS_SIE_BIT, false);
    }

    /// ref: riscv-privileged
    /// The `SPIE` bit indicates whether supervisor interrupts were enabled prior to
    /// trapping into supervisor mode. When a trap is taken into supervisor mode, `SPIE` is set 
    /// to 0. When an `SRET` instruction is executed, `SIE` is set to `SPIE`, then `SPIE` is set to 1.
    pub fn pop_sie(&mut self) {
        if !self.csrs.sstatus.get_bit(STATUS_SIE_BIT) && self.csrs.sstatus.get_bit(STATUS_SPIE_BIT) {
            self.interrupt = true;
        }
        self.csrs.sstatus.set_bit(STATUS_SIE_BIT, self.csrs.sstatus.get_bit(STATUS_SPIE_BIT));
        self.csrs.sstatus.set_bit(STATUS_SPIE_BIT, true);
    }

    pub fn smode(&self) -> bool { 
        self.csrs.sstatus.get_bit(STATUS_SPP_BIT)    
    } 
    // 是否开启分页
    pub fn paged(&self) -> bool { self.csrs.satp != 0 }


}






#[repr(C)]
/// task context structure containing some registers
pub struct TaskContext {
    /// return address ( e.g. __restore ) of __switch ASM function
    ra: usize,
    /// kernel stack pointer of app
    sp: usize,
    /// callee saved registers:  s 0..11
    s: [usize; 12],
}

impl TaskContext {
    /// init task context
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    /// set Task Context{__restore ASM funciton: trap_return, sp: kstack_ptr, s: s_0..12}
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
