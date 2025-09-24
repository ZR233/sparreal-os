use core::ptr::NonNull;

use dma_api::Osal;

use crate::platform::{self, CacheOp};

use super::{PhysAddr, VirtAddr};

struct DMAImpl;

impl Osal for DMAImpl {
    fn map(&self, addr: NonNull<u8>, _size: usize, _direction: dma_api::Direction) -> u64 {
        let vaddr = VirtAddr::from(addr);
        let paddr = PhysAddr::from(vaddr);
        paddr.raw() as _
    }

    fn unmap(&self, _addr: NonNull<u8>, _size: usize) {}
}

pub fn init() {
    unsafe {
        dma_api::init(&DMAImpl);
    }
}
