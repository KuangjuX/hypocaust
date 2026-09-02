use alloc::vec::Vec;
use spin::Mutex;


use crate::constants::layout::TRAP_CONTEXT;
use crate::guest::{Vcpu, VirtualMachine};
use crate::identity::{VcpuId, VmId};
use crate::page_table::{PageTable, PageTableSv39, VirtPageNum};
use crate::debug::PageDebug;
use crate::guest::context::TaskContext;
use crate::guest::switch::__switch;

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

pub struct Hypervisor<P: PageTable + PageDebug> {
    pub meta: MachineMeta,
    /// `feature/vm-vcpu-identities` stores VM ownership explicitly instead of
    /// treating a physical hart number as a Guest index.
    pub vms: Vec<VirtualMachine<P>>,
    pub current_vm_id: VmId,
    pub current_vcpu_id: VcpuId,
}


pub static HYPOCAUST: Mutex<Option<Hypervisor<PageTableSv39>>> = Mutex::new(None);

impl<P: PageTable + PageDebug> Hypervisor<P> {
    pub fn run_vcpu(&mut self, vm_id: VmId, vcpu_id: VcpuId) -> ! {
        self.current_vm_id = vm_id;
        self.current_vcpu_id = vcpu_id;
        let vcpu = self
            .vm(vm_id)
            .and_then(|vm| vm.vcpu(vcpu_id))
            .expect("selected vCPU does not exist");
        let task_cx_ptr = &vcpu.task_cx as *const TaskContext;
        let mut _unused = TaskContext::zero_init();
        hdebug!(
            "run VM {} vCPU {}......",
            vm_id.index(),
            vcpu_id.index(),
        );
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    pub fn add_vm(&mut self, vm: VirtualMachine<P>) {
        assert!(
            self.vms.iter().all(|existing| existing.id != vm.id),
            "duplicate VM ID",
        );
        // `feature/vm-vcpu-identities` uses globally unique vCPU IDs because
        // each ID currently selects one Host kernel-stack slot.
        for vcpu_id in vm.vcpu_ids() {
            assert!(
                self.vms
                    .iter()
                    .all(|existing| existing.vcpu(vcpu_id).is_none()),
                "duplicate global vCPU ID",
            );
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
    pub fn prepare_current_user_token(&mut self) -> (usize, Option<usize>) {
        self.current_vcpu().prepare_user_token()
    }

    pub fn current_trap_cx(&mut self) -> &'static mut TrapContext {
        self.current_vcpu()
            .memory_set
            .translate(VirtPageNum::from(TRAP_CONTEXT >> 12))
            .unwrap()
            .ppn()
            .get_mut()
    }

    pub fn current_vcpu(&mut self) -> &mut Vcpu<P> {
        let vm_id = self.current_vm_id;
        let vcpu_id = self.current_vcpu_id;
        self.vm_mut(vm_id)
            .and_then(|vm| vm.vcpu_mut(vcpu_id))
            .expect("current vCPU does not exist")
    }
}



pub fn initialize_vmm(meta: MachineMeta) {
    unsafe{ HYPOCAUST.force_unlock(); }
    let old = HYPOCAUST.lock().replace(
        Hypervisor{
            meta,
            vms: Vec::new(),
            current_vm_id: VmId::new(0),
            current_vcpu_id: VcpuId::new(0),
        }
    );
    core::mem::forget(old);
}
