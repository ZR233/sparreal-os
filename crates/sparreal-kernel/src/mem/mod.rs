use core::{
    alloc::GlobalAlloc,
    cell::UnsafeCell,
    ptr::{NonNull, null_mut},
};

use buddy_system_allocator::Heap;
use fdt_parser::Fdt;
use spin::Mutex;

#[cfg(feature="mmu")]
pub mod mmu;

// #[global_allocator]
// static ALLOCATOR: KAllocator = KAllocator {
//     inner: Mutex::new(Heap::empty()),
// };

#[global_allocator]
static ALLOCATOR: BootHeap = BootHeap::empty();

struct BootHeap(UnsafeCell<Heap<32>>);

unsafe impl Send for BootHeap {}
unsafe impl Sync for BootHeap {}

impl BootHeap {
    const fn empty() -> Self {
        Self(UnsafeCell::new(Heap::empty()))
    }

    unsafe fn init(&self, start: usize, size: usize) {
        unsafe { (&mut *self.0.get()).init(start, size) };
    }
}

unsafe impl GlobalAlloc for BootHeap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if let Ok(p) = (unsafe { &mut *self.0.get() }).alloc(layout) {
            p.as_ptr()
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        (unsafe { &mut *self.0.get() }).dealloc(unsafe { NonNull::new_unchecked(ptr) }, layout);
    }
}

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

        unsafe { g.add_to_heap(memory.as_mut_ptr() as usize, memory.len()) };
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
/// 重置堆
///
/// # Safty
/// 之前分配的内存将会失效
// pub unsafe fn heap_reset(memory: &mut [u8]) {
//     ALLOCATOR.reset(memory);
// }

pub unsafe fn boot_heap_init(memory: &mut [u8]) {
    let start = memory.len() / 2;
    let memory = &mut memory[start..];

    unsafe { ALLOCATOR.init(memory.as_ptr() as usize, memory.len()) };
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
