use riscv::addr::BitField;


use crate::hypervisor::trap::trap_return;
use crate::constants::csr::sip::{SEIP, SSIP, STIP};
use crate::constants::csr::status::{STATUS_SIE_BIT, STATUS_SPIE_BIT};
use crate::page_table::PageTable;
use crate::debug::PageDebug;
use super::pmap::ShadowPageTables;
use super::shadow_stats::ShadowPagingStats;


pub struct ControlRegisters {
    // sedeleg: usize, -- Hard-wired to zero
    // sideleg: usize, -- Hard-wired to zero
    pub sstatus: usize,
    /// 中断使能寄存器
    pub sie: usize,
    /// 中断代理寄存器
    pub sip: usize,
    pub stvec: usize,
    /// PR #64 (`feature/vcpu-scounteren`) stores the Guest-visible counter
    /// permissions independently for every vCPU.
    pub scounteren: usize,
    pub sscratch: usize,
    pub sepc: usize,
    pub scause: usize,
    pub stval: usize,
    pub satp: usize,
    /// 用于设置 Guest OS 时钟中断
    pub mtimecmp: usize
}

/// PR #40 (`feature/per-vcpu-virtual-interrupts`) names the three Supervisor
/// interrupt classes that Hypocaust can inject into one selected vCPU.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VirtualInterrupt {
    External,
    Software,
    Timer,
}

impl VirtualInterrupt {
    pub const fn pending_bit(self) -> usize {
        match self {
            Self::External => SEIP,
            Self::Software => SSIP,
            Self::Timer => STIP,
        }
    }

    pub const fn cause(self) -> usize {
        match self {
            Self::External => 9,
            Self::Software => 1,
            Self::Timer => 5,
        }
    }
}

impl ControlRegisters {
    /// Hypocaust exposes only the architectural cycle, time, and instret
    /// counters; hardware-performance-monitor counters remain unavailable.
    const STANDARD_COUNTER_MASK: usize = 0b111;

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
            scounteren: 0,
            // PR #16 (fix-bug/modern-rust-toolchain): use the current associated
            // constant while preserving the disabled-until-programmed timer.
            mtimecmp: usize::MAX
        }
    }

    /// PR #64 applies the WARL mask advertised by Hypocaust when Linux writes
    /// `scounteren`, rather than leaking unsupported hardware counter bits.
    pub fn set_scounteren(&mut self, value: usize) {
        self.scounteren = value & Self::STANDARD_COUNTER_MASK;
    }

    /// PR #64 converts virtual privilege into the physical U-mode permission
    /// needed by deprivileged Guests. Virtual S-mode may always read counters;
    /// virtual U-mode is governed by that vCPU's `scounteren` value.
    pub fn effective_scounteren(&self, guest_smode: bool) -> usize {
        if guest_smode {
            Self::STANDARD_COUNTER_MASK
        } else {
            self.scounteren
        }
    }

    /// PR #40 records a virtual interrupt only in this vCPU's shadow `sip`.
    pub fn inject_interrupt(&mut self, interrupt: VirtualInterrupt) {
        self.sip |= interrupt.pending_bit();
    }

    /// PR #40 lets a virtual device or timer deassert its own interrupt line.
    pub fn clear_interrupt(&mut self, interrupt: VirtualInterrupt) {
        self.sip &= !interrupt.pending_bit();
    }

    /// PR #40 applies the architectural SEI > SSI > STI priority order after
    /// masking pending sources with this vCPU's shadow `sie` register.
    pub fn next_enabled_interrupt(&self) -> Option<VirtualInterrupt> {
        let deliverable = self.sip & self.sie;
        [
            VirtualInterrupt::External,
            VirtualInterrupt::Software,
            VirtualInterrupt::Timer,
        ]
        .into_iter()
        .find(|interrupt| deliverable & interrupt.pending_bit() != 0)
    }
}

/// PR #40 checks per-vCPU pending-bit isolation operations and the mandated
/// priority order without relying on a standard-library test harness.
pub fn virtual_interrupt_self_test() {
    let mut registers = ControlRegisters::new();
    assert_eq!(registers.effective_scounteren(true), 0b111);
    assert_eq!(registers.effective_scounteren(false), 0);
    registers.set_scounteren(usize::MAX);
    assert_eq!(registers.scounteren, 0b111);
    assert_eq!(registers.effective_scounteren(false), 0b111);
    registers.sie = SEIP | SSIP | STIP;
    registers.inject_interrupt(VirtualInterrupt::Timer);
    registers.inject_interrupt(VirtualInterrupt::Software);
    registers.inject_interrupt(VirtualInterrupt::External);
    assert_eq!(
        registers.next_enabled_interrupt(),
        Some(VirtualInterrupt::External),
    );
    registers.clear_interrupt(VirtualInterrupt::External);
    assert_eq!(
        registers.next_enabled_interrupt(),
        Some(VirtualInterrupt::Software),
    );
    registers.clear_interrupt(VirtualInterrupt::Software);
    assert_eq!(
        registers.next_enabled_interrupt(),
        Some(VirtualInterrupt::Timer),
    );
}

pub struct ShadowState<P: PageTable + PageDebug> {
    pub csrs: ControlRegisters,
    /// 影子页表
    pub shadow_page_tables: ShadowPageTables<P>,
    /// 连续切换页表次数
    pub conseutive_satp_switch_count: usize,
    /// PR #24 (`feature/shadow-paging-profile`) records the work performed
    /// while maintaining shadow page tables so later optimization PRs have a baseline.
    pub shadow_paging_stats: ShadowPagingStats,
}

impl<P> ShadowState<P> where P: PageTable + PageDebug {
    pub const fn new() -> Self {
        Self {
            csrs: ControlRegisters::new(),
            shadow_page_tables: ShadowPageTables::new(),
            conseutive_satp_switch_count: 0,
            shadow_paging_stats: ShadowPagingStats::new(),
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
        self.csrs.sstatus.set_bit(STATUS_SIE_BIT, self.csrs.sstatus.get_bit(STATUS_SPIE_BIT));
        self.csrs.sstatus.set_bit(STATUS_SPIE_BIT, true);
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
