use arrayvec::ArrayVec;
use core::ops::Range;
use core::ptr::{NonNull, slice_from_raw_parts, slice_from_raw_parts_mut};
use core::sync::atomic::{AtomicUsize, Ordering};
use memory_addr::MemoryAddr;
use sparreal_kernel::mem::mmu::*;
pub use sparreal_kernel::mem::*;
use sparreal_kernel::platform_if::BootRegion;

static FDT_ADDR: AtomicUsize = AtomicUsize::new(0);
static FDT_LEN: AtomicUsize = AtomicUsize::new(0);

macro_rules! pa_of {
    ($name:ident) => {{
        unsafe extern "C" {
            static $name: u8;
        }
        let mut pa = &raw const $name as *const u8 as usize;

        if crate::arch::is_mmu_enabled() {
            pa -= get_text_va_offset()
        }

        PhysAddr::new(pa)
    }};
}

pub(crate) unsafe fn save_fdt(ptr: *mut u8, free_start: usize) -> Option<NonNull<u8>> {
    let fdt_addr = free_start.align_up_4k();
    let fdt = fdt_parser::Fdt::from_ptr(NonNull::new(ptr)?).ok()?;
    let len = fdt.total_size();

    unsafe {
        let dst = &mut *slice_from_raw_parts_mut(fdt_addr as _, len);
        let src = &*slice_from_raw_parts(ptr, len);
        dst.copy_from_slice(src);

        FDT_ADDR.store(fdt_addr, Ordering::SeqCst);
        FDT_LEN.store(len, Ordering::SeqCst);
    }

    NonNull::new(FDT_ADDR.load(Ordering::SeqCst) as _)
}

unsafe extern "C" {
    fn _stext();
    fn _etext();
    fn _srodata();
    fn _erodata();
    fn _sdata();
    fn _edata();
    fn _sbss();
    fn _ebss();
    fn _stack_bottom();
    fn _stack_top();
}

macro_rules! fn_ld_range {
    ($name:ident) => {
        pub fn $name() -> &'static [u8] {
            let start = concat_idents!(_s, $name) as *const u8 as usize;
            let end = concat_idents!(_e, $name) as *const u8 as usize;
            unsafe { &*slice_from_raw_parts(start as *mut u8, end - start) }
        }
    };
}

fn_ld_range!(rodata);
fn_ld_range!(data);
fn_ld_range!(bss);

pub fn stack_cpu0() -> &'static [u8] {
    let start = _stack_bottom as *const u8 as usize;
    let end = _stack_top as *const u8 as usize;
    unsafe { &*slice_from_raw_parts(start as *mut u8, end - start) }
}

pub fn clean_bss() {
    let start = _sbss as *const u8 as usize;
    let end = _ebss as *const u8 as usize;
    let bss = unsafe { &mut *slice_from_raw_parts_mut(start as *mut u8, end - start) };
    bss.fill(0);
}

fn slice_to_phys_range(data: &[u8]) -> Range<PhysAddr> {
    let ptr_range = data.as_ptr_range();
    (ptr_range.start as usize).into()..(ptr_range.end as usize).into()
}

fn fdt_addr_range() -> Option<Range<PhysAddr>> {
    let len = FDT_LEN.load(Ordering::Relaxed);
    if len != 0 {
        let fdt_addr = FDT_ADDR.load(Ordering::Relaxed);
        Some(fdt_addr.into()..(fdt_addr + len.align_up_4k()).into())
    } else {
        None
    }
}

pub fn fdt_addr() -> Option<PhysAddr> {
    let len = FDT_LEN.load(Ordering::Relaxed);
    if len != 0 {
        let fdt_addr = FDT_ADDR.load(Ordering::Relaxed);
        Some(fdt_addr.into())
    } else {
        None
    }
}

pub fn rsv_regions<const N: usize>() -> ArrayVec<BootRegion, N> {
    let mut rsv_regions = ArrayVec::<BootRegion, N>::new();
    rsv_regions.push(BootRegion::new(
        pa_of!(_stext)..pa_of!(_etext),
        c".text",
        AccessSetting::Read | AccessSetting::Execute,
        CacheSetting::Normal,
        RegionKind::KImage,
    ));

    rsv_regions.push(BootRegion::new(
        slice_to_phys_range(rodata()),
        c".rodata",
        AccessSetting::Read | AccessSetting::Execute,
        CacheSetting::Normal,
        RegionKind::KImage,
    ));

    rsv_regions.push(BootRegion::new(
        slice_to_phys_range(data()),
        c".data",
        AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
        CacheSetting::Normal,
        RegionKind::KImage,
    ));

    rsv_regions.push(BootRegion::new(
        slice_to_phys_range(bss()),
        c".bss",
        AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
        CacheSetting::Normal,
        RegionKind::KImage,
    ));

    rsv_regions.push(BootRegion::new(
        slice_to_phys_range(stack_cpu0()),
        c".stack",
        AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
        CacheSetting::Normal,
        RegionKind::Stack,
    ));

    if let Some(fdt) = fdt_addr_range() {
        rsv_regions.push(BootRegion::new(
            fdt,
            c"fdt",
            AccessSetting::Read,
            CacheSetting::Normal,
            RegionKind::Other,
        ));
    }

    rsv_regions
}

pub fn driver_registers() -> &'static [u8] {
    unsafe extern "C" {
        fn _sdriver();
        fn _edriver();
    }

    unsafe { &*slice_from_raw_parts(_sdriver as *const u8, _edriver as usize - _sdriver as usize) }
}
