use core::mem;
use core::ops::{Index, IndexMut};
pub struct MemoryRegion<T: Copy = usize>{
    ptr: *mut T,
    base_address: usize,
    length_bytes: usize
}

unsafe impl<T: Copy + Send> Send for MemoryRegion<T> {}

impl<T: Copy> MemoryRegion<T> {
    pub fn new(addr: usize, length: usize) -> Self {
        assert_eq!(length % mem::size_of::<T>(), 0);
        Self{
            ptr: addr as *mut T,
            base_address: addr,
            length_bytes: length
        }
    }

    pub fn base(&self) -> usize { self.base_address }

    pub fn len(&self) -> usize { self.length_bytes }

    pub fn in_region(&self, addr: usize) -> bool {
        addr >= self.base_address && addr < self.base_address + self.length_bytes
    }
}

impl<T: Copy> Index<usize> for MemoryRegion<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        assert_eq!(index % mem::size_of::<T>(), 0);
        assert!(index >= self.base_address);

        let offset = index - self.base_address;
        assert!(offset < self.length_bytes);
        unsafe{ &*(self.ptr.add(offset / mem::size_of::<T>())) }
    }
}

impl<T: Copy> IndexMut<usize> for MemoryRegion<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        assert_eq!(index % mem::size_of::<T>(), 0);
        assert!(index >= self.base_address);

        let offset = index - self.base_address;
        assert!(offset < self.length_bytes);
        unsafe{ &mut *(self.ptr.add(offset / mem::size_of::<T>())) }
    }
}