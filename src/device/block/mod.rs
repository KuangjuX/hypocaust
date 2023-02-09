use core::any::Any;
use spin::Mutex;
use virtio_drivers::{device::blk::VirtIOBlk, transport::Transport};

use self::virtio_blk::HalImpl;

mod virtio_blk;

pub trait BlockDevice: Send + Sync + Any {
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    fn write_block(&self, block_id: usize, buf: &[u8]);
    fn handle_irq(&self);
}

pub struct VirtIOBlock<T: Transport> {
    virtio_blk: Mutex<VirtIOBlk<HalImpl, T>>
}