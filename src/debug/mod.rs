mod backtrace;
mod pagedebug;

pub use backtrace::print_guest_backtrace;

use crate::mm::{ PageTable, VirtPageNum };
use crate::constants::layout::GUEST_TRAP_CONTEXT;
use crate::trap::TrapContext;


impl PageTable {
    /// Print guest trap context content for debug
    pub fn print_trap_context(&self) {
        let trap_ctx_ppn = self.translate(VirtPageNum::from(GUEST_TRAP_CONTEXT >> 12)).unwrap().ppn().0;
        hdebug!("trap ctx ppn: {:#x}", trap_ctx_ppn);
        unsafe{
            let trap_ctx = &*((trap_ctx_ppn << 12) as *const TrapContext);
            for i in 0..trap_ctx.x.len() {
                hdebug!("x{} -> {:#x}", i, trap_ctx.x[i]);
            }
            hdebug!("sepc -> {:#x}", trap_ctx.sepc);
            hdebug!("sstatus -> {:#x}", trap_ctx.sstatus.bits());
        }
    }
}