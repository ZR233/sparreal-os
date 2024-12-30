use core::{alloc::Layout, ptr::NonNull};

static mut VA_OFFSET: usize = 0;

impl page_table_generic::Access for super::Heap {
    fn va_offset(&self) -> usize {
        unsafe { VA_OFFSET }
    }

    unsafe fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        self.0.alloc(layout).ok()
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: core::alloc::Layout) {
        self.0.dealloc(ptr, layout);
    }
}
