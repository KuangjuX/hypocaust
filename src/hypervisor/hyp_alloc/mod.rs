mod frame_allocator;
mod heap_allocator;

// PR #16 (fix-bug/modern-rust-toolchain): frame_dealloc stays in this module's
// allocator interface although production code normally reaches it via Drop.
#[allow(unused_imports)]
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameTracker};

/// initiate heap allocator, frame allocator and kernel space
pub fn heap_init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
}
