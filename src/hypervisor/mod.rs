use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;


use crate::constants::layout::TRAP_CONTEXT;
use crate::guest::GuestKernel;
use crate::sync::UPSafeCell;
use crate::mm::MemorySet;
use crate::page_table::{PageTable, PageTableSv39, VirtPageNum};
use crate::debug::PageDebug;
use crate::guest::context::TaskContext;
use crate::guest::switch::__switch;

pub use self::hyp_alloc::FrameTracker;
pub use self::fdt::MachineMeta;
use self::trap::TrapContext;



pub mod hyp_alloc;
pub mod trap;
pub mod fdt;

pub struct Hypervisor<P: PageTable + PageDebug> {
    pub hyper_space: Arc<UPSafeCell<MemorySet<P>>>,
    pub frame_queue: UPSafeCell<Vec<FrameTracker>>,
    pub meta: MachineMeta,
    pub guests: Vec<GuestKernel<P>>,
    pub guest_run_id: usize
}


pub static HYPOCAUST: Mutex<Option<Hypervisor<PageTableSv39>>> = Mutex::new(None);

impl<P: PageTable + PageDebug> Hypervisor<P> {
    pub fn run_guest_kernel(&self, guest_id: usize) -> ! {
        let guest_kernel = &self.guests[guest_id];
        let task_cx_ptr = &guest_kernel.task_cx as *const TaskContext;
        let mut _unused = TaskContext::zero_init();
        hdebug!("run guest kernel {}......", guest_id);
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    pub fn add_guest(&mut self, guest: GuestKernel<P>) {
        self.guests.push(guest);
    }

    pub fn current_user_token(&self) -> usize {
        let guest = &self.guests[self.guest_run_id];
        guest.get_user_token()
    }

    pub fn current_trap_cx(&mut self) -> &'static mut TrapContext {
        let guest = &mut self.guests[self.guest_run_id];
        guest.memory_set.translate(VirtPageNum::from(TRAP_CONTEXT >> 12)).unwrap().ppn().get_mut()
    }

    pub fn current_guest(&mut self) -> &mut GuestKernel<P> {
        &mut self.guests[self.guest_run_id]
    }
}



pub fn initialize_vmm(meta: MachineMeta) {
    unsafe{ HYPOCAUST.force_unlock(); }
    let old = HYPOCAUST.lock().replace(
        Hypervisor{
            hyper_space: Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) }),
            frame_queue: unsafe{ UPSafeCell::new(Vec::new()) },
            meta,
            guests: Vec::new(),
            guest_run_id: 0
        }
    );
    core::mem::forget(old);
}

