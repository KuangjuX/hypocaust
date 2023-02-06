//! RISC-V timer-related functionality

use crate::constants::layout::CLOCK_FREQ;
use riscv::register::time;
use crate::sbi::set_timer;

const TICKS_PER_SEC: usize = 100;
#[allow(unused)]
const MSEC_PER_SEC: usize = 1000;

pub fn get_default_timer() -> usize {
    CLOCK_FREQ / TICKS_PER_SEC
}

pub fn get_time() -> usize {
    time::read()
}

#[allow(unused)]
/// get current time in microseconds
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

#[allow(unused)]
pub fn set_next_trigger(stimer: usize) {
    set_timer(stimer);
} 

/// set the next timer interrupt
pub fn set_default_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}
