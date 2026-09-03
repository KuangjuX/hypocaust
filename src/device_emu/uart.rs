use arrayvec::ArrayVec;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::identity::VmId;
use crate::sbi::console_getchar;

const NO_INPUT: usize = usize::MAX;

/// PR #49 gives one VM exclusive ownership of the physical interactive input
/// stream. Output remains visible for every VM through labelled records.
static CONSOLE_FOCUS_VM: AtomicUsize = AtomicUsize::new(0);

pub struct Uart {
    pub dlab: bool,

    pub divisor_latch: u16,
    pub interrupt_enable: u8,

    pub next_interrupt_time: usize,

    pub input_fifo: [u8; 16],
    pub input_bytes_ready: usize,

    pub line_buffer: ArrayVec<u8, 256>,
    pub vm_id: VmId
}

impl Uart {
    pub const fn new(vm_id: VmId) -> Self {
        Self{
            dlab: false,
            interrupt_enable: 0,
            divisor_latch: 1,
            next_interrupt_time: 0,
            input_fifo: [0; 16],
            input_bytes_ready: 0,
            line_buffer: ArrayVec::new_const(),
            vm_id
        }
    }

    /// PR #49 buffers one Guest's SBI console bytes until a record boundary.
    /// This state belongs to its VM's `DeviceBus`, so another Guest cannot
    /// splice bytes into the middle of the record.
    pub fn write_console_byte(&mut self, byte: u8) {
        if byte == b'\n' {
            self.flush_console(true);
            return;
        }
        if self.line_buffer.is_full() {
            self.flush_console(false);
        }
        self.line_buffer.push(byte);
    }

    /// PR #61 (`feature/sbi-dbcn-console`) preserves per-VM line labelling
    /// while accepting one complete DBCN transfer from the Guest.
    pub fn write_console_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write_console_byte(*byte);
        }
    }

    /// PR #49 flushes a partial prompt before polling for input. Only the VM
    /// selected by the Host console focus can consume physical console bytes.
    pub fn read_console_byte(&mut self) -> usize {
        if !self.line_buffer.is_empty() {
            self.flush_console(false);
        }
        if CONSOLE_FOCUS_VM.load(Ordering::Acquire) == self.vm_id.index() {
            console_getchar()
        } else {
            NO_INPUT
        }
    }

    /// PR #61 implements DBCN's non-blocking bulk read. A VM without console
    /// focus, or a focused VM with no pending input, returns a short transfer.
    pub fn read_console_bytes(&mut self, bytes: &mut [u8]) -> usize {
        if !self.line_buffer.is_empty() {
            self.flush_console(false);
        }
        if CONSOLE_FOCUS_VM.load(Ordering::Acquire) != self.vm_id.index() {
            return 0;
        }
        let mut read = 0;
        while read < bytes.len() {
            let byte = console_getchar();
            if byte == NO_INPUT {
                break;
            }
            bytes[read] = byte as u8;
            read += 1;
        }
        read
    }

    fn flush_console(&mut self, newline: bool) {
        crate::console::write_guest_record(self.vm_id, &self.line_buffer, newline);
        self.line_buffer.clear();
    }

    /// PR #49 is the Host management-plane hook for selecting the one Guest
    /// that receives the shared physical console input stream.
    pub fn set_console_focus(vm_id: VmId) {
        CONSOLE_FOCUS_VM.store(vm_id.index(), Ordering::Release);
    }

    pub fn console_focus() -> VmId {
        VmId::new(CONSOLE_FOCUS_VM.load(Ordering::Acquire))
    }
}

/// PR #49 validates VM-local buffering and the exclusive input-focus control
/// without consuming input or emitting a synthetic line on the Host console.
pub(crate) fn self_test() {
    let original_focus = Uart::console_focus();
    Uart::set_console_focus(VmId::new(1));
    assert_eq!(Uart::console_focus(), VmId::new(1));
    Uart::set_console_focus(original_focus);

    let mut uart = Uart::new(VmId::new(0));
    uart.write_console_byte(b'O');
    uart.write_console_byte(b'K');
    uart.write_console_bytes(b" bulk");
    assert_eq!(uart.line_buffer.as_slice(), b"OK bulk");
}
