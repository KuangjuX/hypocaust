pub struct ControlRegisters {
    // sedeleg: usize, -- Hard-wired to zero
    // sideleg: usize, -- Hard-wired to zero
    sstatus: usize,
    /// 中断使能寄存器
    sie: usize,
    /// 中断代理寄存器
    pub sip: usize,
    stvec: usize,
    // scounteren: usize, -- Hard-wired to zero
    sscratch: usize,
    sepc: usize,
    scause: usize,
    stval: usize,
    satp: usize,
    /// 用于设置 Guest OS 时钟中断
    mtimecmp: usize
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
            mtimecmp: 0
        }
    }
}

pub struct ShadowState {
    pub csrs: ControlRegisters,
    /// 影子页表
    pub shadow_page_tables: ShadowPageTables,
    /// 是否发生中断
    pub interrupt: bool
}

impl ShadowState {
    pub const fn new() -> Self {
        Self {
            csrs: ControlRegisters::new(),
            shadow_page_tables: ShadowPageTables::new(),
            interrupt: false
        }
    }

    pub fn get_sstatus(&self) -> usize { self.csrs.sstatus }
    pub fn get_stvec(&self) -> usize { self.csrs.stvec }
    pub fn get_sie(&self) -> usize { self.csrs.sie }
    pub fn get_sscratch(&self) -> usize { self.csrs.sscratch }
    pub fn get_sepc(&self) -> usize { self.csrs.sepc }
    pub fn get_scause(&self) -> usize { self.csrs.scause }
    pub fn get_stval(&self) -> usize { self.csrs.stval }
    pub fn get_satp(&self) -> usize { self.csrs.satp }
    pub fn get_mtimecmp(&self) -> usize { self.csrs.mtimecmp }

    pub fn write_sstatus(&mut self, val: usize) { self.csrs.sstatus  = val}
    pub fn write_stvec(&mut self, val: usize) { self.csrs.stvec = val }
    pub fn write_sie(&mut self, val: usize) { self.csrs.sie = val}
    pub fn write_sscratch(&mut self, val: usize) { self.csrs.sscratch = val }
    pub fn write_sepc(&mut self, val: usize) { self.csrs.sepc = val }
    pub fn write_scause(&mut self, val: usize)  { self.csrs.scause = val }
    pub fn write_stval(&mut self, val: usize) { self.csrs.stval  = val }
    pub fn write_satp(&mut self, val: usize) { self.csrs.satp = val }
    pub fn write_mtimecmp(&mut self, val: usize) { self.csrs.mtimecmp = val }

    pub fn smode(&self) -> bool { 
        self.csrs.sstatus.get_bit(8)    
    } 
    // 是否开启分页
    pub fn paged(&self) -> bool { self.csrs.satp != 0 }


}

use riscv::addr::BitField;

use crate::trap::trap_return;

use super::pmap::ShadowPageTables;


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
