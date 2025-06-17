use core::{
    alloc::Layout,
    ffi::CStr,
    ops::Range,
    ptr::NonNull,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use super::{
    Phys, PhysAddr, PhysCRange, STACK_BOTTOM, Virt, once::OnceStatic, region::boot_regions,
};
pub use arrayvec::ArrayVec;
use buddy_system_allocator::Heap;
use page_table_generic::err::PagingError;
pub use page_table_generic::*;

use crate::{
    globals::{self, cpu_inited, global_val},
    io::print::*,
    platform,
    platform_if::{MMUImpl, PlatformImpl},
    println,
};

mod paging;

pub use paging::init_table;
pub use paging::iomap;

pub const LINER_OFFSET: usize = 0xffff_f000_0000_0000;
static TEXT_OFFSET: OnceStatic<usize> = OnceStatic::new(0);
static IS_MMU_ENABLED: OnceStatic<bool> = OnceStatic::new(false);

pub fn set_mmu_enabled() {
    unsafe { IS_MMU_ENABLED.set(true) };
}

pub fn is_mmu_enabled() -> bool {
    *IS_MMU_ENABLED.get_ref()
}

/// 设置内核段偏移.
///
/// # Safety
///
/// 应在 cpu0 入口处执行
pub unsafe fn set_text_va_offset(offset: usize) {
    unsafe {
        IS_MMU_ENABLED.set(false);
        TEXT_OFFSET.set(offset);
    }
}
pub fn get_text_va_offset() -> usize {
    *TEXT_OFFSET.get_ref()
}

struct PageHeap(Heap<32>);

impl page_table_generic::Access for PageHeap {
    fn va_offset(&self) -> usize {
        0
    }

    unsafe fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        self.0.alloc(layout).ok()
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: core::alloc::Layout) {
        self.0.dealloc(ptr, layout);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BootRegion {
    // 链接地址
    pub range: PhysCRange,
    pub name: *const u8,
    pub access: AccessSetting,
    pub cache: CacheSetting,
    pub kind: RegionKind,
}

impl BootRegion {
    pub fn new(
        range: Range<PhysAddr>,
        name: &'static CStr,
        access: AccessSetting,
        cache: CacheSetting,
        kind: RegionKind,
    ) -> Self {
        Self {
            range: range.into(),
            name: name.as_ptr() as _,
            access,
            cache,
            kind,
        }
    }

    pub fn new_with_len(
        start: PhysAddr,
        len: usize,
        name: &'static CStr,
        access: AccessSetting,
        cache: CacheSetting,
        kind: RegionKind,
    ) -> Self {
        Self::new(start..start + len, name, access, cache, kind)
    }

    pub fn name(&self) -> &'static str {
        unsafe { CStr::from_ptr(self.name as _).to_str().unwrap() }
    }

    pub fn va_offset(&self) -> usize {
        match self.kind {
            RegionKind::Stack => {
                if cpu_inited() {
                    self.kind.va_offset()
                } else {
                    // cpu0
                    STACK_BOTTOM - self.range.start.raw()
                }
            }
            _ => self.kind.va_offset(),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum RegionKind {
    KImage,
    Stack,
    Other,
}

impl RegionKind {
    pub fn va_offset(&self) -> usize {
        match self {
            RegionKind::KImage => get_text_va_offset(),
            RegionKind::Stack => STACK_BOTTOM - globals::cpu_global().stack.start.raw(),
            RegionKind::Other => LINER_OFFSET,
        }
    }
}

impl<T> From<Virt<T>> for Phys<T> {
    fn from(value: Virt<T>) -> Self {
        let v = value.raw();
        todo!()
    }
}
const MB: usize = 1024 * 1024;
pub fn new_boot_table() -> Result<usize, &'static str> {
    let mut access = PageHeap(Heap::empty());
    let main_mem = global_val().main_memory.clone();

    let tmp_end = main_mem.end;
    let tmp_size = tmp_end - main_mem.start.align_up(MB);
    let tmp_pt = (main_mem.end - tmp_size / 2).raw();

    println!("page table allocator {:#x}, {:#x}", tmp_pt, tmp_end.raw());
    unsafe { access.0.add_to_heap(tmp_pt, tmp_end.raw()) };

    let mut table =
        PageTableRef::create_empty(&mut access).map_err(|_| "page table allocator no memory")?;

    for memory in platform::phys_memorys() {
        let region = BootRegion::new(
            memory,
            c"memory",
            AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
            CacheSetting::Normal,
            RegionKind::Other,
        );
        map_region(&mut table, 0, &region, &mut access);
    }

    for region in boot_regions() {
        map_region(&mut table, region.va_offset(), region, &mut access);
    }

    let main_memory = BootRegion::new(
        global_val().main_memory.clone(),
        c"main memory",
        AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
        CacheSetting::Normal,
        RegionKind::Other,
    );

    map_region(
        &mut table,
        main_memory.va_offset(),
        &main_memory,
        &mut access,
    );

    let table_addr = table.paddr();

    println!("Table: {table_addr:#x}");

    Ok(table_addr)
}

fn map_region(
    table: &mut paging::PageTableRef<'_>,
    va_offset: usize,
    region: &BootRegion,
    access: &mut PageHeap,
) {
    let addr = region.range.start;
    let size = region.range.end.raw() - region.range.start.raw();

    // let addr = align_down_1g(addr);
    // let size = align_up_1g(size);
    let vaddr = addr.raw() + va_offset;

    const NAME_LEN: usize = 12;

    let name_right = if region.name().len() < NAME_LEN {
        NAME_LEN - region.name().len()
    } else {
        0
    };

    println!(
        "map region [{:<12}] [{:#x}, {:#x}) -> [{:#x}, {:#x})",
        region.name(),
        vaddr,
        vaddr + size,
        addr.raw(),
        addr.raw() + size
    );

    unsafe {
        if let Err(e) = table.map_region(
            MapConfig::new(vaddr as _, addr.raw(), region.access, region.cache),
            size,
            true,
            access,
        ) {
            // early_handle_err(e);
        }
    }
}

fn early_handle_err(e: PagingError) {
    match e {
        PagingError::NoMemory => println!("no memory"),
        PagingError::NotAligned(e) => {
            println!("not aligned: {e}");
        }
        PagingError::NotMapped => println!("not mapped"),
        PagingError::AlreadyMapped => {}
    }
    panic!()
}

pub fn set_kernel_table(addr: usize) {
    MMUImpl::set_kernel_table(addr);
}

pub fn set_user_table(addr: usize) {
    MMUImpl::set_user_table(addr);
}
pub fn get_user_table() -> usize {
    MMUImpl::get_user_table()
}

#[allow(unused)]
pub(crate) fn flush_tlb(addr: *const u8) {
    unsafe { MMUImpl::flush_tlb(addr) };
}
pub fn flush_tlb_all() {
    MMUImpl::flush_tlb_all();
}
pub fn page_size() -> usize {
    MMUImpl::page_size()
}
pub fn table_level() -> usize {
    MMUImpl::table_level()
}
