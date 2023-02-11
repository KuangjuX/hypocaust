//! refs: https://github.com/mit-pdos/RVirt/blob/HEAD/src/virtio.rs

use arrayvec::ArrayVec;

use crate::mm::MemoryRegion;


pub const MAX_QUEUES: usize = 4;
pub const MAX_DEVICES: usize = 4;
pub const MAX_PAGES: usize = MAX_DEVICES * MAX_QUEUES;

pub struct VirtIO {
    pub devices: ArrayVec<Device, MAX_DEVICES>,
    pub queue_guest_pages: ArrayVec<usize, MAX_PAGES>
}

#[derive(Copy, Clone)]
pub struct Queue {
    /// Address guest thinks queue is mapped at
    guest_pa: usize, 
    /// Address queue is actually mapped at
    host_pa: usize,
    /// Number of entries in queue
    size: usize
}

pub enum Device {
    Passthrough {
        /// Virtual Queue Index, offset=0x30
        queue_sel: u32,
        queues: [Queue; MAX_QUEUES],
        device_registers: MemoryRegion<u32>
    },
    Unmapped
}

impl Device {
    pub unsafe fn new(host_base_address: usize) -> Self {
        Device::Passthrough { 
            queue_sel: 0,
            queues: [Queue { guest_pa: 0, host_pa: 0, size: 0}; MAX_QUEUES],
            device_registers: MemoryRegion::new(host_base_address, 0x1000)
        }
    }
}

