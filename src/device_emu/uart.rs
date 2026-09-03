use arrayvec::ArrayVec;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::identity::VmId;
use crate::sbi::console_getchar;

const NO_INPUT: usize = usize::MAX;

/// PR #71 (`feature/linux-virtual-uart-console`) exposes a QEMU-compatible
/// NS16550A frontend while keeping the physical Host UART owned by Hypocaust.
pub const UART_GPA: usize = 0x1000_0000;
pub const UART_SIZE: usize = 0x100;
pub const UART_IRQ: u32 = 10;

const UART_RBR_THR_DLL: usize = 0;
const UART_IER_DLM: usize = 1;
const UART_IIR_FCR: usize = 2;
const UART_LCR: usize = 3;
const UART_MCR: usize = 4;
const UART_LSR: usize = 5;
const UART_MSR: usize = 6;
const UART_SCR: usize = 7;
const UART_LCR_DLAB: u8 = 1 << 7;
const UART_IER_RX: u8 = 1 << 0;
const UART_IER_TX: u8 = 1 << 1;
const UART_LSR_DATA_READY: u8 = 1 << 0;
const UART_LSR_THR_EMPTY: u8 = 1 << 5;
const UART_LSR_TRANSMITTER_EMPTY: u8 = 1 << 6;

/// PR #49 gives one VM exclusive ownership of the physical interactive input
/// stream. Output remains visible for every VM through labelled records.
static CONSOLE_FOCUS_VM: AtomicUsize = AtomicUsize::new(0);

pub struct Uart {
    pub dlab: bool,

    pub divisor_latch: u16,
    pub interrupt_enable: u8,

    line_control: u8,
    modem_control: u8,
    scratch: u8,
    tx_interrupt_pending: bool,

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
            line_control: 0,
            modem_control: 0,
            scratch: 0,
            tx_interrupt_pending: false,
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

    /// PR #71 routes the Guest UART aperture to this VM-local device instead of
    /// mapping the Host UART page into every Guest shadow page table.
    pub fn contains(&self, guest_address: usize) -> bool {
        guest_address >= UART_GPA && guest_address - UART_GPA < UART_SIZE
    }

    /// PR #71 implements the byte registers used by Linux's 8250 driver. The
    /// transmit holding register is always ready because output is synchronously
    /// copied into Hypocaust's per-VM line buffer.
    pub fn read_u8(&mut self, guest_address: usize) -> Option<u8> {
        let offset = self.offset(guest_address)?;
        match offset {
            UART_RBR_THR_DLL if self.dlab => Some(self.divisor_latch as u8),
            UART_RBR_THR_DLL => Some(self.pop_input().unwrap_or(0)),
            UART_IER_DLM if self.dlab => Some((self.divisor_latch >> 8) as u8),
            UART_IER_DLM => Some(self.interrupt_enable),
            UART_IIR_FCR => Some(self.read_interrupt_identification()),
            UART_LCR => Some(self.line_control),
            UART_MCR => Some(self.modem_control),
            UART_LSR => Some(UART_LSR_THR_EMPTY | UART_LSR_TRANSMITTER_EMPTY
                | if self.input_bytes_ready != 0 { UART_LSR_DATA_READY } else { 0 }),
            UART_MSR => Some(0),
            UART_SCR => Some(self.scratch),
            _ => None,
        }
    }

    /// PR #71 accepts the minimal NS16550A programming sequence used by Linux:
    /// divisor setup, line/FIFO configuration, interrupt enables, and TX bytes.
    pub fn write_u8(&mut self, guest_address: usize, value: u8) -> bool {
        let Some(offset) = self.offset(guest_address) else {
            return false;
        };
        match offset {
            UART_RBR_THR_DLL if self.dlab => {
                self.divisor_latch = (self.divisor_latch & 0xff00) | value as u16;
            }
            UART_RBR_THR_DLL => {
                self.write_console_byte(value);
                // PR #71 models the THR becoming empty after the synchronous
                // Host copy. IIR acknowledgement prevents a permanent IRQ storm.
                self.tx_interrupt_pending = self.interrupt_enable & UART_IER_TX != 0;
            }
            UART_IER_DLM if self.dlab => {
                self.divisor_latch = (self.divisor_latch & 0x00ff) | ((value as u16) << 8);
            }
            UART_IER_DLM => {
                let previous = self.interrupt_enable;
                self.interrupt_enable = value & 0x0f;
                if previous & UART_IER_TX == 0 && self.interrupt_enable & UART_IER_TX != 0 {
                    self.tx_interrupt_pending = true;
                } else if self.interrupt_enable & UART_IER_TX == 0 {
                    self.tx_interrupt_pending = false;
                }
            }
            UART_IIR_FCR => {
                // Bits 1 and 2 reset the receive and transmit FIFOs. TX has no
                // queued state in this synchronous virtual frontend.
                if value & (1 << 1) != 0 {
                    self.input_bytes_ready = 0;
                }
            }
            UART_LCR => {
                self.line_control = value;
                self.dlab = value & UART_LCR_DLAB != 0;
            }
            UART_MCR => self.modem_control = value,
            UART_SCR => self.scratch = value,
            UART_LSR | UART_MSR => {}
            _ => return false,
        }
        true
    }

    /// PR #71 samples the one focused physical input stream at the existing
    /// bounded Host timer boundary and queues it only in the owning VM's FIFO.
    pub fn poll_input(&mut self) {
        if self.interrupt_enable & UART_IER_RX == 0
            || self.input_bytes_ready == self.input_fifo.len()
            || CONSOLE_FOCUS_VM.load(Ordering::Acquire) != self.vm_id.index()
        {
            return;
        }
        let byte = console_getchar();
        if byte != NO_INPUT {
            self.input_fifo[self.input_bytes_ready] = byte as u8;
            self.input_bytes_ready += 1;
        }
    }

    /// PR #71 presents RX and immediately-empty TX as level-triggered UART
    /// conditions. The virtual PLIC converts this device-local level to SEIP.
    pub fn interrupt_pending(&self) -> bool {
        (self.interrupt_enable & UART_IER_RX != 0 && self.input_bytes_ready != 0)
            || self.tx_interrupt_pending
    }

    fn read_interrupt_identification(&mut self) -> u8 {
        if self.interrupt_enable & UART_IER_RX != 0 && self.input_bytes_ready != 0 {
            0x04
        } else if self.tx_interrupt_pending {
            // PR #71 follows the 16550 acknowledgement rule: reporting THRE in
            // IIR clears that latch until a new byte is transmitted.
            self.tx_interrupt_pending = false;
            0x02
        } else {
            0x01
        }
    }

    fn pop_input(&mut self) -> Option<u8> {
        if self.input_bytes_ready == 0 {
            return None;
        }
        let byte = self.input_fifo[0];
        self.input_fifo.copy_within(1..self.input_bytes_ready, 0);
        self.input_bytes_ready -= 1;
        Some(byte)
    }

    fn offset(&self, guest_address: usize) -> Option<usize> {
        self.contains(guest_address).then_some(guest_address - UART_GPA)
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

    // PR #71 locks down the register behavior Linux relies on without emitting
    // an extra Host console record during Hypocaust startup.
    assert_eq!(uart.read_u8(UART_GPA + UART_LSR), Some(0x60));
    assert!(uart.write_u8(UART_GPA + UART_LCR, UART_LCR_DLAB));
    assert!(uart.write_u8(UART_GPA + UART_RBR_THR_DLL, 2));
    assert!(uart.write_u8(UART_GPA + UART_IER_DLM, 0));
    assert_eq!(uart.divisor_latch, 2);
    assert!(uart.write_u8(UART_GPA + UART_LCR, 3));
    assert!(uart.write_u8(UART_GPA + UART_IER_DLM, UART_IER_TX));
    assert!(uart.interrupt_pending());
    assert_eq!(uart.read_u8(UART_GPA + UART_IIR_FCR), Some(0x02));
    assert!(!uart.interrupt_pending());
}
