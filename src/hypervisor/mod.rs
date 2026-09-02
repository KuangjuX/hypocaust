use alloc::vec::Vec;
use core::arch::asm;
use spin::Mutex;

use crate::debug::PageDebug;
use crate::device_emu::DeviceBus;
use crate::guest::context::TaskContext;
use crate::guest::switch::__switch;
use crate::guest::{Vcpu, VirtualInterrupt, VirtualMachine};
use crate::identity::{HartId, VcpuKey, VmId};
use crate::page_table::{PageTable, PageTableSv39};

// PR #16 (fix-bug/modern-rust-toolchain): preserve the allocator type re-export
// while keeping deny(warnings) useful for all other imports.
#[allow(unused_imports)]
pub use self::hyp_alloc::FrameTracker;
pub use self::fdt::MachineMeta;
pub use self::shared::HYPERVISOR_MEMORY;
use self::trap::TrapContext;



pub mod hyp_alloc;
pub mod trap;
pub mod fdt;
pub mod shared;
pub mod scheduler;

use self::scheduler::Scheduler;

pub struct Hypervisor<P: PageTable + PageDebug> {
    pub meta: MachineMeta,
    /// PR #34 (`feature/vm-vcpu-identities`) stores VM ownership explicitly instead of
    /// treating a physical hart number as a Guest index.
    pub vms: Vec<VirtualMachine<P>>,
    /// PR #38 (`feature/multivcpu-scheduler`) owns the runnable queue and one
    /// independent current-vCPU slot per Host hart.
    scheduler: Scheduler,
}


pub static HYPOCAUST: Mutex<Option<Hypervisor<PageTableSv39>>> = Mutex::new(None);

impl<P: PageTable + PageDebug> Hypervisor<P> {
    pub fn add_vm(&mut self, vm: VirtualMachine<P>) {
        assert!(
            self.vms.iter().all(|existing| existing.id != vm.id),
            "duplicate VM ID",
        );
        // PR #34 (`feature/vm-vcpu-identities`) uses globally unique vCPU IDs because
        // each ID currently selects one Host kernel-stack slot.
        for vcpu_id in vm.vcpu_ids() {
            assert!(
                self.vms
                    .iter()
                    .all(|existing| existing.vcpu(vcpu_id).is_none()),
                "duplicate global vCPU ID",
            );
        }
        for vcpu_id in vm.vcpu_ids() {
            self.scheduler.register(VcpuKey::new(vm.id, vcpu_id));
        }
        self.vms.push(vm);
    }

    pub fn vm(&self, id: VmId) -> Option<&VirtualMachine<P>> {
        self.vms.iter().find(|vm| vm.id == id)
    }

    pub fn vm_mut(&mut self, id: VmId) -> Option<&mut VirtualMachine<P>> {
        self.vms.iter_mut().find(|vm| vm.id == id)
    }

    /// PR #26 (`feature/shadow-page-table-asid`) returns both the selected
    /// shadow token and an optional destination ASID that must be fenced.
    pub fn prepare_current_user_token(
        &mut self,
        hart_id: HartId,
    ) -> (usize, Option<usize>) {
        self.current_vcpu(hart_id).prepare_user_token()
    }

    pub fn current_trap_cx(&mut self, hart_id: HartId) -> &'static mut TrapContext {
        self.current_vcpu(hart_id).trap_cx_ppn.get_mut()
    }

    pub fn current_vcpu(&mut self, hart_id: HartId) -> &mut Vcpu<P> {
        let key = self.scheduler.current(hart_id)
            .expect("Host hart has no current vCPU");
        self.vcpu_mut(key)
    }

    /// PR #39 resolves MMIO through the current vCPU's owning VM and returns
    /// its shared device bus instead of vCPU-local device state.
    pub fn current_vcpu_and_device_bus(
        &mut self,
        hart_id: HartId,
    ) -> (&mut Vcpu<P>, &mut DeviceBus) {
        let key = self
            .scheduler
            .current(hart_id)
            .expect("Host hart has no current vCPU");
        self.vm_mut(key.vm_id)
            .and_then(|vm| vm.vcpu_and_device_bus_mut(key.vcpu_id))
            .expect("current vCPU or its device bus does not exist")
    }

    pub fn schedule(&mut self, hart_id: HartId) -> Option<VcpuKey> {
        let key = self.scheduler.schedule(hart_id)?;
        self.bind_vcpu_to_hart(key, hart_id);
        Some(key)
    }

    pub fn preempt(&mut self, hart_id: HartId) -> Option<VcpuKey> {
        let key = self.scheduler.preempt(hart_id)?;
        self.bind_vcpu_to_hart(key, hart_id);
        Some(key)
    }

    pub fn block_current(&mut self, hart_id: HartId) -> Option<VcpuKey> {
        let key = self.scheduler.block_current(hart_id)?;
        self.bind_vcpu_to_hart(key, hart_id);
        Some(key)
    }

    /// PR #40 updates only the selected vCPU's pending bits and asks the
    /// scheduler which running or idle Host hart must be kicked.
    pub fn inject_virtual_interrupt(
        &mut self,
        key: VcpuKey,
        interrupt: VirtualInterrupt,
    ) -> Option<HartId> {
        self.vcpu_mut(key).inject_virtual_interrupt(interrupt);
        self.scheduler.interrupt_target(key)
    }

    fn vcpu_mut(&mut self, key: VcpuKey) -> &mut Vcpu<P> {
        self.vm_mut(key.vm_id)
            .and_then(|vm| vm.vcpu_mut(key.vcpu_id))
            .expect("current vCPU does not exist")
    }

    fn bind_vcpu_to_hart(&mut self, key: VcpuKey, hart_id: HartId) {
        // The assembly trap entry reads this after saving the Guest's tp.
        self.vcpu_mut(key).trap_cx_ppn.get_mut::<TrapContext>().host_hart_id =
            hart_id.index();
    }
}

/// PR #38 (`feature/multivcpu-scheduler`) drops the global lock before entering
/// a Guest. Trap handlers can therefore acquire it without `force_unlock`.
pub fn run_scheduler(hart_id: HartId) -> ! {
    {
        let mut guard = HYPOCAUST.lock();
        guard
            .as_mut()
            .unwrap()
            .scheduler
            .mark_hart_online(hart_id);
    }
    // PR #40 keeps Host software interrupts enabled during Guest execution so
    // a targeted IPI can force prompt virtual-interrupt arbitration.
    trap::enable_software_interrupt();
    loop {
        let next = {
            let mut guard = HYPOCAUST.lock();
            let hypervisor = guard.as_mut().unwrap();
            hypervisor.schedule(hart_id).map(|key| {
                let task_cx_ptr = &hypervisor.vcpu_mut(key).task_cx as *const TaskContext;
                (key, task_cx_ptr)
            })
        };
        if let Some((key, task_cx_ptr)) = next {
            hdebug!(
                "hart {} runs VM {} vCPU {}",
                hart_id.index(),
                key.vm_id.index(),
                key.vcpu_id.index(),
            );
            trap::enable_timer_interrupt();
            crate::timer::set_default_next_trigger();
            let mut scheduler_cx = TaskContext::zero_init();
            unsafe { __switch(&mut scheduler_cx as *mut _, task_cx_ptr) };
            unreachable!();
        }

        // PR #38 lets idle harts accept only Host software interrupts. A
        // wakeup returns through the kernel trap frame and rechecks the queue.
        unsafe {
            asm!(
                "csrsi sie, 2",
                "csrsi sstatus, 2",
                "wfi",
                "csrci sstatus, 2",
            )
        };
    }
}

/// PR #38 makes a blocked vCPU runnable and kicks an already-online idle hart.
/// The IPI is sent after releasing the global lock so the receiver can run.
pub fn wake_vcpu(key: VcpuKey) {
    let target = {
        let mut guard = HYPOCAUST.lock();
        guard.as_mut().unwrap().scheduler.wake(key)
    };
    if let Some(hart_id) = target {
        crate::sbi::send_ipi(hart_id);
    }
}

/// PR #40 is the Host/device-backend entry point for targeted virtual IRQs.
/// Sending the physical IPI outside the lock avoids blocking the destination.
pub fn inject_virtual_interrupt(key: VcpuKey, interrupt: VirtualInterrupt) {
    let target = {
        let mut guard = HYPOCAUST.lock();
        guard
            .as_mut()
            .unwrap()
            .inject_virtual_interrupt(key, interrupt)
    };
    if let Some(hart_id) = target {
        crate::sbi::send_ipi(hart_id);
    }
}



pub fn initialize_vmm(meta: MachineMeta) {
    // PR #38 validates Ready/Running/Blocked transitions before any Guest can
    // enter the scheduler and make a failure harder to diagnose.
    scheduler::self_test();
    crate::guest::virtual_interrupt_self_test();
    crate::guest::payload_self_test();
    // PR #54 validates BASE probing and TIME action decoding before Linux can
    // depend on the modern Guest SBI contract.
    crate::guest::sbi_self_test();
    crate::page_table::translation_self_test();
    crate::device_emu::passthrough_self_test();
    crate::device_emu::console_self_test();
    trap::exception_routing_self_test();
    // PR #45 (`feature/multi-guest-qemu`) maps every active VirtIO aperture
    // reported by the Host DTB before a VM backend dereferences its registers.
    // These are Host mappings; per-VM DeviceBus config controls Guest access.
    for device in &meta.virtio {
        crate::mm::map_host_mmio(device.base_address, device.size);
    }
    let old = HYPOCAUST.lock().replace(
        Hypervisor{
            meta,
            vms: Vec::new(),
            scheduler: Scheduler::new(),
        }
    );
    core::mem::forget(old);
}
