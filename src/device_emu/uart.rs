use arrayvec::ArrayVec;
use crate::identity::VmId;

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
}
