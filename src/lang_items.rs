//! The panic handler

use crate::sbi::shutdown;
use core::panic::PanicInfo;

#[panic_handler]
/// panic handler
fn panic(info: &PanicInfo) -> ! {
    // PR fix-bug/modern-rust-toolchain: PanicMessage now implements Display
    // directly, so reporting works without the removed Option-like API.
    if let Some(location) = info.location() {
        println!(
            "\x1b[1;31m[hypervisor] Panicked at {}:{} {}\x1b[0m",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("[kernel] Panicked: {}", info.message());
    }
    shutdown()
}
