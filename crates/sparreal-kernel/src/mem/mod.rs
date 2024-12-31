use core::{
    alloc::GlobalAlloc,
    cell::UnsafeCell,
    ptr::{NonNull, null_mut, slice_from_raw_parts_mut},
};

use buddy_system_allocator::Heap;
use fdt_parser::Fdt;
use memory_addr::MemoryAddr;
use spin::Mutex;

use crate::{boot::BootInfo, platform};

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

    match info.device_info_kind {
        crate::boot::PlatformInfoKind::DeviceTree { addr } => {
            platform::fdt::set_addr(addr);

            let fdt = platform::fdt::get_fdt().unwrap();

            for memory in fdt.memory() {
                for region in memory.regions() {
                    add_to_heap(info, region.address as _, region.size);
                }
            }
        }
    }
}

fn add_to_heap(info: &BootInfo, mut start: usize, size: usize) {
    let mut start = start + va_offset();

    let memory_range = start..start + size;
    let half = memory_range.start + size / 2;
    let kernel_end = info.kernel.as_ptr_range().end as usize;
    let stack_bottom = info.stack.as_ptr_range().start as usize;
    let stack_top = info.stack.as_ptr_range().end as usize;

    let mut end = start + size;

    if memory_range.contains(&kernel_end) {
        start = kernel_end;
    }

    if memory_range.contains(&stack_bottom) {
        if stack_bottom > half {
            end = stack_bottom.min(end);
        } else {
            start = (stack_top + 0x16).max(start);
        }
    }

    start = start.align_up_4k();
    end = end.align_down_4k();

    ALLOCATOR.add_to_heap(unsafe { &mut *slice_from_raw_parts_mut(start as *mut u8, end - start) });
}
