use core::{
    alloc::Layout,
    ptr::{NonNull, slice_from_raw_parts, slice_from_raw_parts_mut},
};

use buddy_system_allocator::Heap;
use fdt_parser::Fdt;

#[unsafe(no_mangle)]
static mut VA_OFFSET: usize = 0;

pub(crate) fn set_va_offset(va_offset: usize) {
    unsafe { VA_OFFSET = va_offset };
}

pub fn va_offset() -> usize {
    unsafe { VA_OFFSET }
}

pub fn stack_top() -> usize {
    unsafe extern "C" {
        fn _stack_top();
    }

    _stack_top as usize
}

pub unsafe fn clear_bss() {
    unsafe {
        unsafe extern "C" {
            fn _sbss();
            fn _ebss();
        }
        let bss = &mut *slice_from_raw_parts_mut(_sbss as *mut u8, _ebss as usize - _sbss as usize);
        bss.fill(0);
    }
}

static mut FDT: Option<&'static [u8]> = None;

pub unsafe fn move_dtb(src: *const u8) -> Option<&'static [u8]> {
    let mut dst = unsafe { NonNull::new(_stack_top as *mut u8)? };

    let fdt = Fdt::from_ptr(unsafe { NonNull::new_unchecked(src as usize as _) }).ok()?;
    let size = fdt.total_size();
    let dest = unsafe { &mut *slice_from_raw_parts_mut(dst.as_mut(), size) };
    let src = unsafe { &*slice_from_raw_parts(src, size) };
    dest.copy_from_slice(src);
    unsafe { FDT = Some(dest) }
    Some(dest)
}

pub fn get_fdt_data() -> Option<&'static [u8]> {
    unsafe { FDT.map(|p| p) }
}

pub fn get_fdt<'a>() -> Option<Fdt<'a>> {
    if let Some(data) = get_fdt_data() {
        Fdt::from_bytes(data).ok()
    } else {
        None
    }
}

unsafe extern "C" {
    fn _skernel();
    fn _ekernel();
    fn _stack_bottom();
    fn _stack_top();
}

pub fn kernel_data() -> &'static [u8] {
    unsafe {
        core::slice::from_raw_parts(
            _skernel as usize as *const u8,
            _ekernel as usize - _skernel as usize,
        )
    }
}

pub fn kernel_stack() -> &'static [u8] {
    unsafe {
        core::slice::from_raw_parts(
            _stack_bottom as usize as *const u8,
            _stack_top as usize - _stack_bottom as usize,
        )
    }
}

pub fn boot_heap() -> &'static [u8] {
    let main_memory = get_fdt()
        .unwrap()
        .memory()
        .next()
        .unwrap()
        .regions()
        .next()
        .unwrap();

    unsafe {
        let start = kernel_stack().as_ptr_range().end as usize;
        let end = main_memory.address as usize + main_memory.size + va_offset();

        let len = end - start;

        core::slice::from_raw_parts(start as _, len)
    }
}

#[allow(unused)]
pub struct PageAllocator {
    heap: Heap<32>,
    va_offset: usize,
}

impl PageAllocator {
    pub unsafe fn new(start: usize, size: usize, va_offset: usize) -> Self {
        let mut heap = Heap::new();
        unsafe { heap.init(start, size) };
        Self { heap, va_offset }
    }
}

impl page_table_generic::Access for PageAllocator {
    fn va_offset(&self) -> usize {
        self.va_offset
    }

    unsafe fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        self.heap.alloc(layout).ok()
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: core::alloc::Layout) {
        self.heap.dealloc(ptr, layout);
    }
}
