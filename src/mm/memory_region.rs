use core::mem;
pub struct MemoryRegion<T: Copy = usize>{
    ptr: *mut T,
    base_address: usize,
    length_bytes: usize
}

unsafe impl<T: Copy + Send> Send for MemoryRegion<T> {}

impl<T: Copy> MemoryRegion<T> {
    pub unsafe fn new(addr: usize, length: usize) -> Self {
        assert_eq!(length % mem::size_of::<T>(), 0);
        Self{
            ptr: addr as *mut T,
            base_address: addr,
            length_bytes: length
        }
    }

    pub fn in_region(&self, addr: usize) -> bool {
        addr >= self.base_address && addr < self.base_address + self.length_bytes
    }
}