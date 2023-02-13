// use core::ptr::NonNull;

// use alloc::vec;
// use virtio_drivers::transport::Transport;
// use virtio_drivers::transport::mmio::{VirtIOHeader, MmioTransport};
// use virtio_drivers::{Hal, device::blk::VirtIOBlk};
// use fdt::{ Fdt, node::FdtNode};

// use crate::hypervisor::HYPOCAUST;
// use crate::hypervisor::hyp_alloc::{frame_alloc, frame_dealloc};
// use crate::page_table::PhysPageNum;
// use crate::sync::UPSafeCell;
// use super::BlockDevice;


// pub struct VirtIOBlock<T: Transport>(UPSafeCell<VirtIOBlk<VirtioHal, T>>);


// impl<T: Transport> VirtIOBlock<T> {
//     pub fn new(transport: T) -> Self {
//         let virtio_blk = unsafe{
//             UPSafeCell::new(
//                 VirtIOBlk::new(transport).expect("failed to create vitio blk device")
//             )
//         };
//         Self(virtio_blk)
//     }
// } 

// unsafe impl<T: Transport> Sync for VirtIOBlock<T>{}
// unsafe impl<T: Transport> Send for VirtIOBlock<T>{}

// pub fn initialize_virtio_blk(dtb: usize) -> Option<MmioTransport> {
//     hdebug!("device tree @ {:#x}", dtb);
//     // Safe because the pointer is a valid pointer to unaliased memory.
//     let fdt = unsafe{ Fdt::from_ptr(dtb as *const u8).unwrap() };
//     for node in fdt.all_nodes() {
//         if let Some(compatible) = node.compatible() {
//             if compatible.all().any(|s| s == "virtio,mmio") {
//                 if let Some(transport) = virtio_probe(node) {
//                     return Some(transport)
//                 }
//             }
//         }
//     }
//     None
// }

// /// refs: https://github.com/rcore-os/virtio-drivers/blob/master/examples/riscv/src/main.rs
// fn virtio_probe(node: FdtNode) -> Option<MmioTransport> {
//     if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
//         let paddr = reg.starting_address as usize;
//         let vaddr = paddr;
//         let header = NonNull::new(vaddr as *mut VirtIOHeader).unwrap();
//         match unsafe{ MmioTransport::new(header) } {
//             Err(_) => { 
//                 // hwarning!("Error creating VirIO MMIO transport {}", e); 
//                 return None
//             },
//             Ok(transport) => {
//                 hdebug!(
//                     "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
//                     transport.vendor_id(),
//                     transport.device_type(),
//                     transport.version()
//                 );
//                 return Some(transport)
//             }
//         }
//     }
//     None
// }

// impl<T: Transport + 'static> BlockDevice for VirtIOBlock<T> {
//     fn read_block(&self, block_id: usize, buf: &mut [u8]) {
//         self.0
//             .exclusive_access()
//             .read_block(block_id, buf)
//             .expect("Error when reading VirtIOBlk");
//     }

//     fn write_block(&self, block_id: usize, buf: &[u8]) {
//         self.0
//             .exclusive_access()
//             .write_block(block_id, buf)
//             .expect("Error when writing VirtIOBlk");
//     }
// }

// pub struct VirtioHal;

// impl Hal for VirtioHal {
//     /// Allocates the given number of continguous physical pages of DMA memory for VirtIO use.
//     /// 
//     /// Returns both the physical address which the device can use to access the memory, and a pointer to the start of it 
//     /// which the driver can use to access it.
//     fn dma_alloc(pages: usize, _direction: virtio_drivers::BufferDirection) -> (virtio_drivers::PhysAddr, core::ptr::NonNull<u8>) {
//         let hypocaust = HYPOCAUST.lock();
//         let hypocaust = (&*hypocaust).as_ref().unwrap();
//         let mut queue = hypocaust.frame_queue.exclusive_access();
//         let mut ppn_base = PhysPageNum(0);
//         for i in 0..pages {
//             let frame = frame_alloc().unwrap();
//             if i == 0 {
//                 ppn_base = frame.ppn;
//             }
//             assert_eq!(frame.ppn.0, ppn_base.0 + i);
//             queue.push(frame);
//         }
//         let pa: virtio_drivers::PhysAddr  = ppn_base.0 << 12;
//         let va = NonNull::new(pa as _).unwrap();
//         (pa, va)
//     }

//     /// Reallocates the given contiguou physical DMA memory pages
//     fn dma_dealloc(paddr: virtio_drivers::PhysAddr, _vaddr: core::ptr::NonNull<u8>, pages: usize) -> i32 {
//         let mut ppn_base = PhysPageNum::from(paddr >> 12);
//         for _ in 0..pages {
//             frame_dealloc(ppn_base);
//             ppn_base.0 += 1;
//         }
//         0
//     }

//     /// Converts a physical address used for MMIO to a virtual address which the driver can access
//     /// 
//     /// This is only used for MMIO addressed within BARs read from the device, for the PCI transport. It may check
//     /// that the address range up to the given size is within the region expected for MMIO.
//     fn mmio_phys_to_virt(paddr: virtio_drivers::PhysAddr, _size: usize) -> core::ptr::NonNull<u8> {
//         NonNull::new(paddr as _).unwrap()
//     }

//     /// Shares the given memory range with the device, and return the physical address that the device can use to access it.
//     /// 
//     /// This may involve mapping the buffer into an IOMMU, giving the host permission to access the memory, or copying
//     /// it to a special region where it canbe accessed.
//     fn share(buffer: core::ptr::NonNull<[u8]>, _direction: virtio_drivers::BufferDirection) -> virtio_drivers::PhysAddr {
//         let vaddr = buffer.as_ptr() as *mut u8 as usize;
//         vaddr as virtio_drivers::PhysAddr
//     }

//     /// Unshares the given memory range from the device and (if necessary) copies it back to the original buffer.
//     fn unshare(_paddr: virtio_drivers::PhysAddr, _buffer: core::ptr::NonNull<[u8]>, _direction: virtio_drivers::BufferDirection) {
//     }
// }




// pub fn virtio_blk_test() {
//     let mut hypocaust = HYPOCAUST.lock();
//     let hypocaust = (&mut *hypocaust).as_mut().unwrap();
//     if let Some(blk) = hypocaust.virtio_blk.as_mut() {
//         let mut blk = blk.0.exclusive_access();
//         let mut input = vec![0xffu8; 512];
//         let mut output = vec![0; 512];
//         for i in 0..32 {
//             for x in input.iter_mut() {
//                 *x = i as u8;
//             }
//             blk.write_block(i, &input).expect("failed to write");
//             blk.read_block(i, &mut output).expect("failed to read");
//             assert_eq!(input, output);
//         }
//         hdebug!("virtio-blk test finished");
//     }else{
//         hwarning!("failed to find virtio blk device");
//     }

// }