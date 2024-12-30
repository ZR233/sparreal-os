use core::{
    alloc::GlobalAlloc,
    ptr::{NonNull, null_mut},
};

use fdt_parser::Fdt;
use spin::Mutex;

mod mmu;

struct Heap(pub(crate) buddy_system_allocator::Heap<32>);

impl Heap {
    const fn empty() -> Self {
        Self(buddy_system_allocator::Heap::empty())
    }
}

#[global_allocator]
static ALLOCATOR: KAllocator = KAllocator {
    inner: Mutex::new(Heap::empty()),
};

pub struct KAllocator {
    inner: Mutex<Heap>,
}

impl KAllocator {
    pub fn reset(&self, memory: &mut [u8]) {
        let mut g = self.inner.lock();

        let mut h = Heap::empty();

        unsafe { h.0.init(memory.as_mut_ptr() as usize, memory.len()) };

        *g = h;
    }

    pub fn add_to_heap(&self, memory: &mut [u8]) {
        let mut g = self.inner.lock();

        unsafe { g.0.add_to_heap(memory.as_mut_ptr() as usize, memory.len()) };
    }
}

unsafe impl GlobalAlloc for KAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if let Ok(p) = self.inner.lock().0.alloc(layout) {
            p.as_ptr()
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.inner
            .lock()
            .0
            .dealloc(unsafe { NonNull::new_unchecked(ptr) }, layout);
    }
}
/// 重置堆
///
/// # Safty
/// 之前分配的内存将会失效
pub unsafe fn heap_reset(memory: &mut [u8]) {
    ALLOCATOR.reset(memory);
}

static mut VA_OFFSET: usize = 0;

pub(crate) unsafe fn set_va_offset(offset: usize) {
    unsafe { VA_OFFSET = offset };
}

static mut FDT: Option<&'static [u8]> = None;

pub fn set_dtb_data(dtb: &'static [u8]) {
    unsafe { FDT = Some(dtb) }
}

pub fn get_fdt<'a>() -> Option<Fdt<'a>> {
    let r = unsafe { FDT.map(|p| p) };

    if let Some(data) = r {
        Fdt::from_bytes(data).ok()
    } else {
        None
    }
}
