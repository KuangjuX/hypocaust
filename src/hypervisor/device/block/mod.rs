use core::any::Any;

pub use virtio_blk::{ initialize_virtio_blk, VirtIOBlock, virtio_blk_test };





mod virtio_blk;

pub trait BlockDevice: Send + Sync + Any {
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    fn write_block(&self, block_id: usize, buf: &[u8]);
}





