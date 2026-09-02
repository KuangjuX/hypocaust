//! SBI console driver, for text output

use crate::sbi::console_putchar;
use crate::identity::VmId;
use core::fmt::{self, Write};
use spin::Mutex;

/// PR #49 (`feature/per-vm-console`) serializes complete Host log records and
/// buffered Guest lines across Host harts.
static CONSOLE_OUTPUT: Mutex<()> = Mutex::new(());

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    let _guard = CONSOLE_OUTPUT.lock();
    Stdout.write_fmt(args).unwrap();
}

/// PR #49 emits one VM-labelled Guest record while holding the same lock used
/// by Hypervisor logging, preventing byte-level output interleaving.
pub fn write_guest_record(vm_id: VmId, bytes: &[u8], newline: bool) {
    let _guard = CONSOLE_OUTPUT.lock();
    let mut stdout = Stdout;
    write!(stdout, "[Guest VM {}] ", vm_id.index()).unwrap();
    for byte in bytes {
        console_putchar(*byte as usize);
    }
    if newline {
        console_putchar(b'\n' as usize);
    }
}

#[macro_export]
/// print string macro
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
/// println string macro
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! hdebug {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!("[Hypervisor] ", $fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! hwarning {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!("[Warning] ", $fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! htracking {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!("[Tracking] ", $fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! herror {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[1;31m[Error] ", $fmt, "\x1b[0m\n") $(, $($arg)+)?));
    }
}
