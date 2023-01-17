use crate::{mm::{PageTable, VirtPageNum, VirtAddr}, guest::translate_guest_paddr};

pub struct ShadowState {
    // sedeleg: usize, -- Hard-wired to zero
    // sideleg: usize, -- Hard-wired to zero

    sstatus: usize,
    sie: usize,
    // sip: usize, -- checked dynamically on read
    stvec: usize,
    // scounteren: usize, -- Hard-wired to zero
    sscratch: usize,
    sepc: usize,
    scause: usize,
    stval: usize,
    satp: usize,

    // Whether the guest is in S-Mode.
    smode: bool,

    // 根目录页表
    root_page_table: Option<PageTable>,

    /// 影子页表
    pub shadow_pgt: ShadowPageTable
}

impl ShadowState {
    pub const fn new() -> Self {
        Self {
            sstatus: 0,
            stvec: 0,
            sie: 0,
            sscratch: 0,
            sepc: 0,
            scause: 0,
            stval: 0,
            satp: 0,

            smode: true,

            root_page_table: None,
            shadow_pgt: ShadowPageTable::new()
        }
    }

    pub fn get_sstatus(&self) -> usize { self.sstatus }
    pub fn get_stvec(&self) -> usize { self.stvec }
    pub fn get_sie(&self) -> usize { self.sie }
    pub fn get_sscratch(&self) -> usize { self.sscratch }
    pub fn get_sepc(&self) -> usize { self.sepc }
    pub fn get_scause(&self) -> usize { self.scause }
    pub fn get_stval(&self) -> usize { self.stval }
    pub fn get_satp(&self) -> usize { self.satp }

    pub fn write_sstatus(&mut self, val: usize) { self.sstatus  = val}
    pub fn write_stvec(&mut self, val: usize) { self.stvec = val }
    pub fn write_sie(&mut self, val: usize) { self.sie = val}
    pub fn write_sscratch(&mut self, val: usize) { self.sscratch = val }
    pub fn write_sepc(&mut self, val: usize) { self.sepc = val }
    pub fn write_scause(&mut self, val: usize)  { self.scause = val }
    pub fn write_stval(&mut self, val: usize) { self.stval  = val }
    pub fn write_satp(&mut self, val: usize) { self.satp = val }

    pub fn smode(&self) -> bool { self.smode } 
    // 是否开启分页
    pub fn paged(&self) -> bool { self.satp != 0 }

    /// 将 guest 虚拟地址翻译成 guest 物理地址(即 host 虚拟地址)
    pub fn translate_guest_vaddr(&self, guest_vaddr: usize) -> usize {
        if let Some(shadow_pg) = &self.root_page_table {
            let offset = guest_vaddr & 0xfff;
            let guest_vaddr = VirtAddr(guest_vaddr);
            let guest_vpn: VirtPageNum = guest_vaddr.floor();
            hdebug!("guest_vpn: {:?}, guest_vaddr: {:?}", guest_vpn, guest_vaddr);
            let guest_ppn = shadow_pg.translate(guest_vpn).unwrap().ppn();
            let guest_paddr: usize = guest_ppn.into();
            guest_paddr + offset
        }else{
            guest_vaddr
        }
    }
}

use crate::trap::trap_return;

use super::shadow_pgt::ShadowPageTable;

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
