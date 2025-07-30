use core::ptr::NonNull;

use dma_api::Impl;

use crate::platform_if::{CacheOp, PlatformImpl};

use super::{PhysAddr, VirtAddr, mmu::RegionKind};

struct DMAImpl;

impl Impl for DMAImpl {
    fn map(addr: NonNull<u8>, _size: usize, _direction: dma_api::Direction) -> u64 {
        let vaddr = VirtAddr::from(addr);
        let paddr = PhysAddr::from(vaddr);
        paddr.raw() as _
    }

    fn unmap(_addr: NonNull<u8>, _size: usize) {}

    fn flush(addr: NonNull<u8>, size: usize) {
        PlatformImpl::dcache_range(CacheOp::Clean, addr.as_ptr() as _, size);
    }

    fn invalidate(addr: NonNull<u8>, size: usize) {
        PlatformImpl::dcache_range(CacheOp::Invalidate, addr.as_ptr() as _, size);
    }
}

dma_api::set_impl!(DMAImpl);
