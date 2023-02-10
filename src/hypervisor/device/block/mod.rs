use core::{any::Any, ptr::NonNull};
use spin::Mutex;
use virtio_drivers::{device::blk::VirtIOBlk, transport::{Transport, mmio::{VirtIOHeader, MmioTransport}}};

use fdt::{node::FdtNode, standard_nodes::Compatible, Fdt};

use self::virtio_blk::VirtioHal;


mod virtio_blk;

pub trait BlockDevice: Send + Sync + Any {
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    fn write_block(&self, block_id: usize, buf: &[u8]);
}

pub struct VirtIOBlock<T: Transport> {
    virtio_blk: Mutex<VirtIOBlk<VirtioHal, T>>
}

/// refs: https://github.com/rcore-os/virtio-drivers/blob/master/examples/riscv/src/main.rs
pub fn init_dt(dtb: usize) {
    hdebug!("device tree @ {:#x}", dtb);
    // Safe because the pointer is a valid pointer to unaliased memory.
    let fdt = unsafe{ Fdt::from_ptr(dtb as *const u8).unwrap() };
    walk_dt(fdt);
}

fn walk_dt(fdt: Fdt) {
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            if compatible.all().any(|s| s == "virtio,mmio") {
                virtio_probe(node);
            }
        }
    }
}

fn virtio_probe(node: FdtNode) {
    if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
        let paddr = reg.starting_address as usize;
        let size = reg.size.unwrap();
        let vaddr = paddr;
        hdebug!("walk dt addr={:#x}, size={:#x}", paddr, size);
        hdebug!(
            "Device tree node {}: {:?}",
            node.name,
            node.compatible().map(Compatible::first)
        );
        let header = NonNull::new(vaddr as *mut VirtIOHeader).unwrap();
        match unsafe{ MmioTransport::new(header) } {
            Err(e) => { hwarning!("Error creating VirIO MMIO transport {}", e); },
            Ok(transport) => {
                hdebug!(
                    "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
                    transport.vendor_id(),
                    transport.device_type(),
                    transport.version()
                );
            }
        }
    }
}

