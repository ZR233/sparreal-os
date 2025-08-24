#![allow(unused)]

use core::{
    alloc::GlobalAlloc,
    ptr::{NonNull, null_mut, slice_from_raw_parts_mut},
    sync::atomic::{AtomicUsize, Ordering},
};

use buddy_system_allocator::Heap;
use log::debug;
use mmu::RegionKind;
// use page_table_generic::{AccessSetting, CacheSetting};
use spin::Mutex;

use crate::{
    globals::global_val,
    mem::mmu::{BootMemoryKind, BootRegion},
    platform::{self, kstack_size},
    println,
};

mod addr;
mod cache;
// #[cfg(feature = "mmu")]
pub mod mmu;
pub mod once;
pub mod region;
pub use addr::*;

#[global_allocator]
static ALLOCATOR: KAllocator = KAllocator {
    inner: Mutex::new(Heap::empty()),
};

pub struct KAllocator {
    pub(crate) inner: Mutex<Heap<32>>,
}

impl KAllocator {
    pub fn reset(&self, memory: &mut [u8]) {
        let mut g = self.inner.lock();

        let mut h = Heap::empty();

        unsafe { h.init(memory.as_mut_ptr() as usize, memory.len()) };

        *g = h;
    }

    pub fn add_to_heap(&self, memory: &mut [u8]) {
        let mut g = self.inner.lock();
        let range = memory.as_mut_ptr_range();

        unsafe { g.add_to_heap(range.start as usize, range.end as usize) };
    }
}

unsafe impl GlobalAlloc for KAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if let Ok(p) = self.inner.lock().alloc(layout) {
            p.as_ptr()
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.inner
            .lock()
            .dealloc(unsafe { NonNull::new_unchecked(ptr) }, layout);
    }
}

const STACK_BOTTOM: usize = 0xffff_e100_0000_0000;
pub fn stack_bottom() -> usize {
    STACK_BOTTOM
}
pub fn stack_top() -> usize {
    STACK_BOTTOM + kstack_size()
}

pub(crate) fn init_heap() {
    let main = find_main_memory().expect("no main memory found");

    let main = global_val().main_memory.clone();
    let mut start = VirtAddr::from(main.start.raw() + RegionKind::Other.va_offset());
    let mut end = VirtAddr::from(main.end.raw() + RegionKind::Other.va_offset());

    println!("heap add memory [{}, {})", start, end);
    ALLOCATOR.add_to_heap(unsafe { &mut *slice_from_raw_parts_mut(start.into(), end - start) });

    println!("heap initialized");
}

fn find_main_memory() -> Option<BootRegion> {
    let mut ram = heapless::Vec::<_, 32>::new();

    println!("find main memory");

    for r in platform::boot_regions() {
        println!(
            "  region {:?} {} {:?} {:?} {:?}",
            r.range,
            r.name(),
            r.access,
            r.cache,
            r.kind
        );
    }

    let mut main = None;

    for region in platform::boot_regions() {
        if matches!(region.kind, BootMemoryKind::Ram) {
            ram.push(region).ok()?;
        }
    }

    main
}

// pub(crate) fn init_page_and_memory() {
//     #[cfg(feature = "mmu")]
//     mmu::init_table();

//     let main = global_val().main_memory.clone();

//     for memory in global_val().platform_info.memorys() {
//         if memory.contains(&main.start) {
//             continue;
//         }
//         // let start = VirtAddr::from(memory.start);
//         // let end = VirtAddr::from(memory.end);
//         // let len = memory.end - memory.start;

//         // debug!("Heap add memory [{}, {})", start, end);
//         // ALLOCATOR.add_to_heap(unsafe { &mut *slice_from_raw_parts_mut(start.as_mut_ptr(), len) });
//     }
// }

// #[repr(C)]
// #[derive(Debug, Clone, Copy)]
// pub struct CMemRange {
//     pub start: usize,
//     pub end: usize,
// }

// impl CMemRange {
//     pub fn as_slice(&self) -> &'static [u8] {
//         unsafe { core::slice::from_raw_parts(self.start as *const u8, self.end - self.start) }
//     }
// }

// #[repr(C)]
// #[derive(Debug, Clone, Copy)]
// pub struct KernelRegions {
//     pub text: CMemRange,
//     pub rodata: CMemRange,
//     pub data: CMemRange,
//     pub bss: CMemRange,
// }

pub fn iomap(paddr: PhysAddr, _size: usize) -> NonNull<u8> {
    unimplemented!();
    // #[cfg(feature = "mmu")]
    // {
    //     mmu::iomap(paddr, _size)
    // }

    // #[cfg(not(feature = "mmu"))]
    // unsafe {
    //     NonNull::new_unchecked(paddr.as_usize() as *mut u8)
    // }
}
