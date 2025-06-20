use arrayvec::ArrayVec;
use core::ops::Range;
use core::ptr::{NonNull, slice_from_raw_parts, slice_from_raw_parts_mut};
use core::sync::atomic::{AtomicUsize, Ordering};
use fdt_parser::Fdt;
use memory_addr::MemoryAddr;
use pie_boot::{BootInfo, MemoryRegionKind};
use sparreal_kernel::mem::mmu::*;
pub use sparreal_kernel::mem::*;
use sparreal_kernel::platform_if::BootRegion;

static FDT_ADDR: AtomicUsize = AtomicUsize::new(0);
static FDT_LEN: AtomicUsize = AtomicUsize::new(0);

static mut BOOT_RSV_START: usize = 0;
static mut BOOT_RSV_END: usize = 0;

pub fn setup_boot_args(args: &BootInfo) {
    set_fdt_addr(args.fdt);
    let rsv = args
        .memory_regions
        .iter()
        .find(|o| matches!(o.kind, MemoryRegionKind::Bootloader))
        .unwrap();
    unsafe {
        BOOT_RSV_START = rsv.start;
        BOOT_RSV_END = rsv.end.align_up_4k();
    }
}

fn set_fdt_addr(ptr: Option<NonNull<u8>>) {
    let ptr = match ptr {
        Some(v) => v,
        None => {
            return;
        }
    };

    let fdt = Fdt::from_ptr(ptr).unwrap();
    let len = fdt.total_size();
    FDT_ADDR.store(ptr.as_ptr() as _, Ordering::Relaxed);
    FDT_LEN.store(len, Ordering::Relaxed);
}

macro_rules! section_phys {
    ($b:ident,$e:ident) => {
        {
            unsafe extern "C" {
                fn $b();
                fn $e();
            }
            let start = $b as *const u8 as usize - get_text_va_offset();
            let end = $e as *const u8 as usize - get_text_va_offset();
            PhysAddr::new(start)..PhysAddr::new(end)
        }
    };
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

pub fn stack_cpu0() -> &'static [u8] {
    let start = _stack_bottom as *const u8 as usize - get_text_va_offset();
    let end = _stack_top as *const u8 as usize - get_text_va_offset();
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

pub fn fdt_addr() -> Option<PhysAddr> {
    let len = FDT_LEN.load(Ordering::Relaxed);
    if len != 0 {
        let fdt_addr = FDT_ADDR.load(Ordering::Relaxed);
        Some(fdt_addr.into())
    } else {
        None
    }
}

fn fdt_addr_range() -> Option<Range<PhysAddr>> {
    let len = FDT_LEN.load(Ordering::Relaxed);
    if len != 0 {
        let fdt_addr = FDT_ADDR.load(Ordering::Relaxed);
        Some(fdt_addr.align_down_4k().into()..(fdt_addr + len.align_up_4k()).into())
    } else {
        None
    }
}

pub fn rsv_regions<const N: usize>() -> ArrayVec<BootRegion, N> {
    let mut rsv_regions = ArrayVec::<BootRegion, N>::new();
    rsv_regions.push(BootRegion::new(
        section_phys!(_stext, _etext),
        c".text",
        AccessSetting::Read | AccessSetting::Execute,
        CacheSetting::Normal,
        RegionKind::KImage,
    ));

    rsv_regions.push(BootRegion::new(
        section_phys!(_srodata, _erodata),
        c".rodata",
        AccessSetting::Read | AccessSetting::Execute,
        CacheSetting::Normal,
        RegionKind::KImage,
    ));

    rsv_regions.push(BootRegion::new(
        section_phys!(_sdata, _edata),
        c".data",
        AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
        CacheSetting::Normal,
        RegionKind::KImage,
    ));

    rsv_regions.push(BootRegion::new(
        section_phys!(_sbss, _ebss),
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

    // unsafe {
    //     if BOOT_RSV_START != 0 && BOOT_RSV_END != 0 {
    //         rsv_regions.push(BootRegion::new(
    //             PhysAddr::new(BOOT_RSV_START)..PhysAddr::new(BOOT_RSV_END),
    //             c"boot_rsv",
    //             AccessSetting::Read | AccessSetting::Write,
    //             CacheSetting::Normal,
    //             RegionKind::KImage,
    //         ));
    //     }
    // }

    rsv_regions
}

pub fn driver_registers() -> &'static [u8] {
    unsafe extern "C" {
        fn _sdriver();
        fn _edriver();
    }

    unsafe { &*slice_from_raw_parts(_sdriver as *const u8, _edriver as usize - _sdriver as usize) }
}
