use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::constants::csr::sie::{SEIE, STIE, SSIE, STIE_BIT};
use crate::constants::csr::sip::SSIP;
use crate::debug::PageDebug;
use crate::hypervisor::HYPERVISOR_MEMORY;
use crate::page_table::{VirtAddr, PhysPageNum, PageTable};
use crate::mm::{MemorySet, MapPermission};
use crate::hypervisor::trap::{TrapContext, trap_handler};
use crate::constants::layout::{
    GUEST_KERNEL_VIRT_START, MAX_HOST_HARTS, TRAP_CONTEXT, kernel_stack_position,
};
use crate::constants::csr;
use crate::device_emu::{DeviceBus, DeviceBusConfig};
use crate::identity::{GuestHartId, VcpuId, VmId};


pub mod switch;
pub mod context;
mod fdt;
mod pmap;
mod memory;
mod shadow_stats;
pub mod sbi;

use context::TaskContext;
use riscv::addr::BitField;

pub use self::context::{ShadowState, VirtualInterrupt};
pub(crate) use self::context::virtual_interrupt_self_test;
pub use self::memory::GuestMemory;
pub(crate) use self::fdt::install_guest_fdt;
// PR #52 (`feature/linux-guest-fdt`) exposes a policy-neutral FDT interface for
// the later Linux initramfs example while the current xv6 path keeps its simple
// compatibility wrapper above.
#[allow(unused_imports)]
pub(crate) use self::fdt::{
    install_configured_guest_fdt, GuestFdtConfig, GuestInitrdRange,
};
// PR #16 (fix-bug/modern-rust-toolchain): retain public translation helpers
// without weakening the crate-wide warning policy.
#[allow(unused_imports)]
pub use self::pmap::{ShadowPageTables, PageTableRoot};

/// PR #43 (`feature/vm-runtime-config`) collects the resources assigned to a
/// VM so construction never selects a physical device backend implicitly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmConfig {
    pub id: VmId,
    pub device_bus: DeviceBusConfig,
}

impl VmConfig {
    pub const fn new(id: VmId, device_bus: DeviceBusConfig) -> Self {
        Self { id, device_bus }
    }

    pub const fn qemu_default() -> Self {
        Self::new(VmId::new(0), DeviceBusConfig::qemu_default())
    }
}

/// PR #45 (`feature/multi-guest-qemu`) carries the RISC-V boot arguments that
/// belong to one vCPU rather than deriving them from Host scheduler identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VcpuBootConfig {
    pub guest_hart_id: GuestHartId,
    pub device_tree_gpa: usize,
}

impl VcpuBootConfig {
    pub const fn new(guest_hart_id: GuestHartId, device_tree_gpa: usize) -> Self {
        Self {
            guest_hart_id,
            device_tree_gpa,
        }
    }
}

/// PR #34 (`feature/vm-vcpu-identities`) makes a VM the ownership boundary for vCPUs.
/// PR #36 (`feature/vm-guest-memory`) moves Guest RAM here.
/// PR #39 (`feature/per-vm-device-bus`) makes devices part of the same VM
/// ownership boundary so all of its vCPUs share one coherent bus.
pub struct VirtualMachine<P: PageTable + PageDebug> {
    pub id: VmId,
    guest_memory: Arc<GuestMemory>,
    device_bus: DeviceBus,
    vcpus: Vec<Vcpu<P>>,
}

impl<P> VirtualMachine<P>
where
    P: PageDebug + PageTable,
{
    pub fn new(config: VmConfig) -> Self {
        let id = config.id;
        let guest_memory = Arc::new(
            GuestMemory::for_vm(id).expect("VM has no Guest memory slot"),
        );
        Self {
            id,
            // PR #36 (`feature/vm-guest-memory`) makes the VM own the RAM
            // capability shared by its vCPUs and checked translation clients.
            guest_memory: Arc::clone(&guest_memory),
            // PR #39 (`feature/per-vm-device-bus`) gives every vCPU in this VM
            // one shared MMIO namespace and coherent device state.
            device_bus: DeviceBus::new(guest_memory, config.device_bus),
            vcpus: Vec::new(),
        }
    }

    pub fn add_vcpu(
        &mut self,
        memory_set: MemorySet<P>,
        id: VcpuId,
        boot: VcpuBootConfig,
    ) {
        assert!(
            self.vcpus.iter().all(|existing| existing.id != id),
            "duplicate vCPU ID",
        );
        assert!(
            self.vcpus
                .iter()
                .all(|existing| existing.guest_hart_id != boot.guest_hart_id),
            "duplicate Guest hart ID",
        );
        // PR #43 bounds the VM-local ID by the virtual PLIC's context capacity
        // before any device completion attempts to address that context.
        assert!(
            boot.guest_hart_id.index() < MAX_HOST_HARTS,
            "Guest hart ID exceeds virtual PLIC capacity",
        );
        self.vcpus.push(Vcpu::new(
            memory_set,
            id,
            boot,
            Arc::clone(&self.guest_memory),
        ));
    }

    pub fn guest_memory(&self) -> &GuestMemory {
        &self.guest_memory
    }

    /// PR #39 returns disjoint mutable references to VM-owned CPU and device
    /// state for one emulated MMIO instruction.
    pub fn vcpu_and_device_bus_mut(
        &mut self,
        id: VcpuId,
    ) -> Option<(&mut Vcpu<P>, &mut DeviceBus)> {
        let vcpu = self.vcpus.iter_mut().find(|vcpu| vcpu.id == id)?;
        Some((vcpu, &mut self.device_bus))
    }

    pub fn vcpu(&self, id: VcpuId) -> Option<&Vcpu<P>> {
        self.vcpus.iter().find(|vcpu| vcpu.id == id)
    }

    pub fn vcpu_mut(&mut self, id: VcpuId) -> Option<&mut Vcpu<P>> {
        self.vcpus.iter_mut().find(|vcpu| vcpu.id == id)
    }

    pub fn vcpu_ids(&self) -> impl Iterator<Item = VcpuId> + '_ {
        self.vcpus.iter().map(|vcpu| vcpu.id)
    }
}

/// PR #34 (`feature/vm-vcpu-identities`) keeps one virtual CPU's architectural state
/// separate from both its owning VM and the physical hart that executes it.
pub struct Vcpu<P: PageTable + PageDebug> {
    pub memory_set: MemorySet<P>,
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: TaskContext,
    pub shadow_state: ShadowState<P>,
    pub vm_id: VmId,
    pub id: VcpuId,
    /// PR #43 separates the VM-local architectural hart number from the global
    /// ID used for scheduler registration and Host kernel-stack selection.
    pub guest_hart_id: GuestHartId,
    /// PR #36 (`feature/vm-guest-memory`) shares the VM-owned immutable address
    /// translation capability without copying or reconstructing memory slots.
    pub guest_memory: Arc<GuestMemory>,
    /// PR #18 (fix-bug/smode-interrupt-forwarding): current virtual privilege mode.
    /// This is separate from sstatus.SPP, which records the mode before a trap.
    pub smode: bool,
}

impl<P> Vcpu<P> where P: PageDebug + PageTable {
    fn new(
        memory_set: MemorySet<P>,
        id: VcpuId,
        boot: VcpuBootConfig,
        guest_memory: Arc<GuestMemory>,
    ) -> Self {
        let vm_id = guest_memory.vm_id();
        let guest_hart_id = boot.guest_hart_id;
        // 获取中断上下文的物理地址
        let mut hypervisor_memory = HYPERVISOR_MEMORY.exclusive_access();
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 获取内核栈地址
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(id.index());
        // 将内核栈地址进行映射
        hypervisor_memory.insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let mut vcpu = Self {
            memory_set,
            trap_cx_ppn,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            shadow_state: ShadowState::new(),
            vm_id,
            id,
            guest_hart_id,
            guest_memory,
            smode: true,
        };
        // 设置 Guest OS `sstatus` 的 `SPP`
        let mut sstatus = riscv::register::sstatus::read();
        sstatus.set_spp(riscv::register::sstatus::SPP::Supervisor);
        vcpu.shadow_state.csrs.sstatus = sstatus.bits();
        // 获取中断上下文的地址
        let trap_cx : &mut TrapContext = vcpu.trap_cx_ppn.get_mut();
        *trap_cx = TrapContext::app_init_context(
            GUEST_KERNEL_VIRT_START,
            0,
            hypervisor_memory.token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        // PR #43 passes the VM-local hart identity through the standard RISC-V
        // boot ABI. Multiple VMs may therefore each boot a hart numbered zero.
        trap_cx.x[10] = guest_hart_id.index();
        // PR #45 supplies each VM's own synthesized DTB through the standard
        // RISC-V `a1` boot argument instead of leaking the Host device tree.
        trap_cx.x[11] = boot.device_tree_gpa;
        vcpu
    }

    /// PR #26 (`feature/shadow-page-table-asid`) selects the next guest token
    /// and consumes any pending ASID-scoped flush for that shadow root.
    pub fn prepare_user_token(&mut self) -> (usize, Option<usize>) {
        let guest_satp = self.shadow_state.csrs.satp;
        let (token, flush_asid) = match self.shadow() {
            // PR #26 (`feature/shadow-page-table-asid`) keeps bare mappings on
            // ASID 0, so switching from the Host root must flush that namespace.
            PageTableRoot::GPA => (self.memory_set.token(), Some(0)),
            PageTableRoot::GVA => if self.shadow_state.csrs.satp == self.shadow_state.shadow_page_tables.guest_satp.unwrap() 
                                    { (self.shadow_state.shadow_page_tables.page_tables[1].unwrap(), self.shadow_state.shadow_page_tables.take_tlb_flush(guest_satp)) }
                                    else{ (self.shadow_state.shadow_page_tables.page_tables[2].unwrap(), self.shadow_state.shadow_page_tables.take_tlb_flush(guest_satp)) },
            PageTableRoot::UVA => (self.shadow_state.shadow_page_tables.page_tables[2].unwrap(), self.shadow_state.shadow_page_tables.take_tlb_flush(guest_satp)),
        };
        self.shadow_state.shadow_paging_stats.record_tlb_decision(flush_asid.is_some());
        (token, flush_asid)
    }

    /// 用来检查应当使用哪一级的影子页表
    pub fn shadow(&self) -> PageTableRoot {
        if (self.shadow_state.csrs.satp >> 60) & 0xf == 0 {
            PageTableRoot::GPA
        }else if !self.smode {
            PageTableRoot::UVA
        }else {
            PageTableRoot::GVA
        }
    }

    /// PR #48 returns `None` for a CSR that Hypocaust does not virtualize so
    /// the illegal instruction can be injected into the Guest instead of
    /// reaching a Host `unreachable!()`.
    pub fn get_csr(&self, csr: usize) -> Option<usize> {
        let shadow_state = &self.shadow_state;
        match csr {
            csr::sstatus => Some(shadow_state.csrs.sstatus),
            csr::stvec => Some(shadow_state.csrs.stvec),
            csr::sie => Some(shadow_state.csrs.sie),
            csr::sscratch => Some(shadow_state.csrs.sscratch),
            csr::sepc => Some(shadow_state.csrs.sepc),
            csr::scause => Some(shadow_state.csrs.scause),
            csr::stval => Some(shadow_state.csrs.stval),
            csr::satp => Some(shadow_state.csrs.satp),
            _ => None,
        }
    }

    /// PR #40 queues an interrupt in this vCPU only. Device backends select
    /// their destination with a `VcpuKey` before calling this method.
    pub fn inject_virtual_interrupt(&mut self, interrupt: VirtualInterrupt) {
        self.shadow_state.csrs.inject_interrupt(interrupt);
    }

    /// PR #40 deasserts one virtual interrupt source without disturbing other
    /// pending sources owned by this vCPU.
    pub fn clear_virtual_interrupt(&mut self, interrupt: VirtualInterrupt) {
        self.shadow_state.csrs.clear_interrupt(interrupt);
    }

    /// PR #48 reports unsupported CSR writes to the instruction emulator.
    pub fn set_csr(&mut self, csr: usize, val: usize) -> bool {
        let shadow_state = &mut self.shadow_state;
        match csr {
            csr::sstatus => { 
                shadow_state.csrs.sstatus  = val
             }
            csr::stvec => shadow_state.csrs.stvec = val,
            csr::sie => { 
                let value = val & (SEIE | STIE | SSIE);
                if value.get_bit(STIE_BIT) {
                    unsafe{ riscv::register::sie::set_stimer() };
                }
                shadow_state.csrs.sie = val;
            }
            csr::sip => {
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
                    _ => return false,
                }
            }
            _ => return false,
        }
        true
    }
    

}
