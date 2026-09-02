//! SBI call wrappers

use core::arch::asm;
use crate::identity::HartId;


const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;


#[inline(always)]
/// general sbi call
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
            "li x16, 0",
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x17") which,
        );
    }
    ret
}

/// use sbi call to putchar in console (qemu uart handler)
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

/// use sbi call to getchar from console (qemu uart handler)
pub fn console_getchar() -> usize {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

pub fn set_timer(stime: usize) {
    sbi_rt::set_timer(stime as u64);
}

/// PR #38 (`feature/multivcpu-scheduler`) starts a Host hart at Hypocaust's
/// physical entry point through the standard SBI HSM extension.
pub fn start_hart(hart_id: HartId, start_addr: usize, opaque: usize) -> bool {
    let result = sbi_rt::hart_start(hart_id.index(), start_addr, opaque);
    if result.error != 0 {
        hwarning!("failed to start hart {}: {:?}", hart_id.index(), result);
        return false;
    }
    true
}

/// PR #38 wakes one idle Host hart after a vCPU becomes runnable.
pub fn send_ipi(hart_id: HartId) {
    let result = sbi_rt::send_ipi(1, hart_id.index());
    assert_eq!(result.error, 0, "SBI failed to send scheduler IPI");
}

/// use sbi call to shutdown the kernel
pub fn shutdown() -> ! {
    sbi_rt::system_reset(sbi_rt::Shutdown, sbi_rt::SystemFailure);
    unreachable!()
}
