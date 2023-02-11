///! ref: https://github.com/mit-pdos/RVirt/blob/HEAD/src/fdt.rs

use arrayvec::ArrayVec;
use fdt::Fdt;

#[derive(Clone, Debug)]
pub struct Device {
    pub base_address: usize,
    pub size: usize
}

#[derive(Clone, Debug, Default)]
pub struct MachineMeta{
    pub physical_memory_offset: usize,
    pub physical_memory_size: usize,

    pub virtio: ArrayVec<Device, 16>
}

impl MachineMeta {
    pub fn parse(dtb: usize) -> Self {
        let fdt = unsafe{ Fdt::from_ptr(dtb as *const u8) }.unwrap();
        let memory = fdt.memory();
        let mut meta = MachineMeta::default();
        for region in memory.regions() {
            meta.physical_memory_offset = region.starting_address as usize;
            meta.physical_memory_size = region.size.unwrap();
        }
        // 发现 virtio mmio 设备
        for node in fdt.find_all_nodes("/soc/virtio_mmio") {
            if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
                let paddr = reg.starting_address as usize;
                let size = reg.size.unwrap();
                let vaddr = paddr;
                unsafe{
                    let header = vaddr as *const u32;
                    let device_id_addr = header.add(2);
                    let device_id = core::ptr::read_volatile(device_id_addr);
                    if device_id != 0 {
                        hdebug!("virtio mmio addr: {:#x}, size: {:#x}", paddr, size);
                        meta.virtio.push(
                            Device { base_address: paddr, size }
                        )
                    }
                }
            }
        }
        meta.virtio.sort_unstable_by_key(|v| v.base_address);
        meta
    }
}