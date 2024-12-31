use core::{
    alloc::GlobalAlloc,
    cell::UnsafeCell,
    ptr::{NonNull, null_mut, slice_from_raw_parts_mut},
};

use buddy_system_allocator::Heap;
use fdt_parser::Fdt;
use memory_addr::MemoryAddr;
use spin::Mutex;

use crate::{
    boot::BootInfo,
    platform::{self, PlatformImpl},
    println,
};

#[cfg(feature = "mmu")]
pub mod mmu;

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

static mut VA_OFFSET: usize = 0;

fn set_va_offset(offset: usize) {
    unsafe { VA_OFFSET = offset };
}

fn va_offset() -> usize {
    unsafe { VA_OFFSET }
}

pub(crate) fn init(info: &BootInfo) {
    set_va_offset(info.va_offset);

    let heap = info.heap.as_ptr_range();

    let start = (heap.start as usize).align_up_4k();
    let end = (heap.end as usize).align_down_4k();

    println!("heap add memory {:#x} - {:#x}", start, end);
    ALLOCATOR.add_to_heap(unsafe { &mut *slice_from_raw_parts_mut(start as *mut u8, end - start) });

    println!("heap initialized");
}
