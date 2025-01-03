use alloc::string::{String, ToString};

use buddy_system_allocator::Heap;
use core::{
    alloc::{GlobalAlloc, Layout},
    ops::Range,
    ptr::NonNull,
};
use page_table_generic::{AccessSetting, CacheSetting, MapConfig};
use spin::MutexGuard;

mod paging;

use super::{ALLOCATOR, KAllocator, VA_OFFSET};
use paging::{PTEImpl, PageTableRef};

const G: usize = 1 << 30;

struct PageHeap(Heap<32>);

impl page_table_generic::Access for PageHeap {
    fn va_offset(&self) -> usize {
        0
    }

    unsafe fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        unsafe { self.0.alloc(layout).ok() }
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: core::alloc::Layout) {
        unsafe { self.0.dealloc(ptr, layout) };
    }
}

pub fn new_boot_table(
    va_offset: usize,
    main_memory: Range<usize>,
    debug_reg: usize,
) -> Result<usize, String> {
    let mut access = PageHeap(Heap::empty());
    let mut start = main_memory.start;
    let size = (main_memory.end - main_memory.start) / 2;
    start += size;

    unsafe { access.0.add_to_heap(start, main_memory.end) };

    let mut table = PageTableRef::create_empty(&mut access).map_err(|_| "no memory".to_string())?;

    let main_addr = align_down_1g(main_memory.start);
    let main_size = align_up_1g(main_memory.end - main_memory.start);

    let debug_addr = align_down_1g(debug_reg);
    let debug_size = G;

    unsafe {
        let _ = table.map_region(
            MapConfig::new(
                main_addr as _,
                main_addr,
                AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
                CacheSetting::Normal,
            ),
            main_size,
            true,
            &mut access,
        );

        let _ = table.map_region(
            MapConfig::new(
                (main_addr + va_offset) as _,
                main_addr,
                AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
                CacheSetting::Normal,
            ),
            main_size,
            true,
            &mut access,
        );

        let _ = table.map_region(
            MapConfig::new(
                debug_addr as _,
                debug_addr,
                AccessSetting::Read | AccessSetting::Write,
                CacheSetting::Device,
            ),
            debug_size,
            true,
            &mut access,
        );

        let _ = table.map_region(
            MapConfig::new(
                (debug_addr + va_offset) as _,
                debug_addr,
                AccessSetting::Read | AccessSetting::Write,
                CacheSetting::Device,
            ),
            debug_size,
            true,
            &mut access,
        );
    }

    Ok(table.paddr())
}

fn align_down_1g(val: usize) -> usize {
    val & !(G - 1)
}
fn align_up_1g(val: usize) -> usize {
    (val + G - 1) & !(G - 1)
}
