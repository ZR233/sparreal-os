use core::ops::Range;
use core::ptr::{NonNull, slice_from_raw_parts};
use core::sync::atomic::{AtomicUsize, Ordering};
use fdt_parser::Fdt;
use memory_addr::MemoryAddr;
use somehal::mem::page_size;
use somehal::{BootInfo, MemoryRegionKind};
use sparreal_kernel::mem::mmu::*;
use sparreal_kernel::mem::once::OnceStatic;
pub use sparreal_kernel::mem::*;

static BOOT_REGIONS: OnceStatic<heapless::Vec<BootRegion, 64>> =
    OnceStatic::new(heapless::Vec::new());

static mut VA_OFFSET: usize = 0;

static FDT_ADDR: AtomicUsize = AtomicUsize::new(0);
static FDT_LEN: AtomicUsize = AtomicUsize::new(0);

pub fn setup_boot_args(args: &BootInfo) {
    set_fdt_addr(args.fdt);
    unsafe {
        VA_OFFSET = args.kcode_offset();
    }
    let mut regions = heapless::Vec::<BootRegion, 64>::new();

    for region in args.memory_regions.iter() {
        let name;
        let kind;

        match region.kind {
            MemoryRegionKind::Ram => {
                name = c"ram";
                kind = BootMemoryKind::Ram;
            }
            MemoryRegionKind::Reserved => {
                name = c"reserved";
                kind = BootMemoryKind::Reserved;
            }
            MemoryRegionKind::Bootloader => {
                name = c"embedded loader";
                kind = BootMemoryKind::KImage;
            }
            MemoryRegionKind::UnknownUefi(_) => {
                name = c"uefi";
                kind = BootMemoryKind::Reserved;
            }
            MemoryRegionKind::UnknownBios(_) => {
                name = c"bios";
                kind = BootMemoryKind::Reserved;
            }
            _ => {
                name = c"reserved";
                kind = BootMemoryKind::Reserved;
            }
        }
        regions
            .push(BootRegion::new(
                Phys::from(region.start)..Phys::from(region.end),
                name,
                AccessSetting::ReadWriteExecute,
                CacheSetting::Normal,
                kind,
            ))
            .expect("boot regions overflow");
    }

    for region in this_boot_region() {
        regions.push(region).expect("boot regions overflow");
    }

    if let Some(debug) = &args.debug_console {
        let start = debug.base_phys.align_down(page_size());
        let end = (debug.base_phys + 0x1000).align_up(page_size());

        regions
            .push(BootRegion::new(
                start.into()..end.into(),
                c"debug",
                AccessSetting::ReadWrite,
                CacheSetting::Device,
                BootMemoryKind::Mmio,
            ))
            .expect("boot regions overflow");
    }

    regions.sort_by(|a, b| a.range.start.raw().cmp(&b.range.start.raw()));

    unsafe { OnceStatic::set(&BOOT_REGIONS, regions) };

    somehal::println!("boot regions:");

    for region in boot_regions() {
        somehal::println!(
            "  [{:<16}] {:?}\t{:?}\t{:?}\t{:?}",
            region.name(),
            region.range,
            region.kind,
            region.cache,
            region.access,
        );
    }
}

pub(crate) fn va_offset() -> usize {
    unsafe { VA_OFFSET }
}

pub(crate) fn boot_regions() -> &'static [BootRegion] {
    BOOT_REGIONS.as_slice()
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
            let start = $b as *const u8 as usize - va_offset();
            let end = $e as *const u8 as usize - va_offset();
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
    fn __bss_start();
    fn __bss_stop();
    fn __cpu0_stack_top();
    fn __cpu0_stack();
}

pub fn stack_cpu0() -> &'static [u8] {
    let start = __cpu0_stack as *const u8 as usize - va_offset();
    let end = __cpu0_stack_top as *const u8 as usize - va_offset();
    unsafe { &*slice_from_raw_parts(start as *mut u8, end - start) }
}

fn slice_to_phys_range(data: &[u8]) -> Range<PhysAddr> {
    let ptr_range = data.as_ptr_range();
    (ptr_range.start as usize).into()..(ptr_range.end as usize).into()
}

// pub fn fdt_addr() -> Option<PhysAddr> {
//     let len = FDT_LEN.load(Ordering::Relaxed);
//     if len != 0 {
//         let fdt_addr = FDT_ADDR.load(Ordering::Relaxed);
//         Some(fdt_addr.into())
//     } else {
//         None
//     }
// }

// fn fdt_addr_range() -> Option<Range<PhysAddr>> {
//     let len = FDT_LEN.load(Ordering::Relaxed);
//     if len != 0 {
//         let fdt_addr = FDT_ADDR.load(Ordering::Relaxed);
//         Some(fdt_addr.align_down_4k().into()..(fdt_addr + len.align_up_4k()).into())
//     } else {
//         None
//     }
// }

fn this_boot_region() -> impl Iterator<Item = BootRegion> {
    [
        BootRegion::new(
            section_phys!(_stext, _etext),
            c".text",
            AccessSetting::ReadExecute,
            CacheSetting::Normal,
            BootMemoryKind::KImage,
        ),
        BootRegion::new(
            section_phys!(_srodata, _erodata),
            c".rodata",
            AccessSetting::ReadExecute,
            CacheSetting::Normal,
            BootMemoryKind::KImage,
        ),
        BootRegion::new(
            section_phys!(_sdata, _edata),
            c".data",
            AccessSetting::ReadWriteExecute,
            CacheSetting::Normal,
            BootMemoryKind::KImage,
        ),
        BootRegion::new(
            section_phys!(__bss_start, __bss_stop),
            c".bss",
            AccessSetting::ReadWriteExecute,
            CacheSetting::Normal,
            BootMemoryKind::KImage,
        ),
        BootRegion::new(
            slice_to_phys_range(stack_cpu0()),
            c".stack0",
            AccessSetting::ReadWriteExecute,
            CacheSetting::PerCpu,
            BootMemoryKind::KImage,
        ),
    ]
    .into_iter()
}

// pub fn rsv_regions<const N: usize>() -> ArrayVec<BootRegion, N> {
//     let mut rsv_regions = ArrayVec::<BootRegion, N>::new();
//     rsv_regions.push(BootRegion::new(
//         section_phys!(_stext, _etext),
//         c".text",
//         AccessSetting::Read | AccessSetting::Execute,
//         CacheSetting::Normal,
//         RegionKind::KImage,
//     ));

//     rsv_regions.push(BootRegion::new(
//         section_phys!(_srodata, _erodata),
//         c".rodata",
//         AccessSetting::Read | AccessSetting::Execute,
//         CacheSetting::Normal,
//         RegionKind::KImage,
//     ));

//     rsv_regions.push(BootRegion::new(
//         section_phys!(_sdata, _edata),
//         c".data",
//         AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
//         CacheSetting::Normal,
//         RegionKind::KImage,
//     ));

//     rsv_regions.push(BootRegion::new(
//         section_phys!(__bss_start, __bss_stop),
//         c".bss",
//         AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
//         CacheSetting::Normal,
//         RegionKind::KImage,
//     ));

//     rsv_regions.push(BootRegion::new(
//         slice_to_phys_range(stack_cpu0()),
//         c".stack",
//         AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
//         CacheSetting::Normal,
//         RegionKind::Stack,
//     ));

//     if let Some(fdt) = fdt_addr_range() {
//         rsv_regions.push(BootRegion::new(
//             fdt,
//             c"fdt",
//             AccessSetting::Read,
//             CacheSetting::Normal,
//             RegionKind::Other,
//         ));
//     }

//     // unsafe {
//     //     if BOOT_RSV_START != 0 && BOOT_RSV_END != 0 {
//     //         rsv_regions.push(BootRegion::new(
//     //             PhysAddr::new(BOOT_RSV_START)..PhysAddr::new(BOOT_RSV_END),
//     //             c"boot_rsv",
//     //             AccessSetting::Read | AccessSetting::Write,
//     //             CacheSetting::Normal,
//     //             RegionKind::KImage,
//     //         ));
//     //     }
//     // }

//     rsv_regions
// }

pub fn driver_registers() -> &'static [u8] {
    unsafe extern "C" {
        fn _sdriver();
        fn _edriver();
    }

    unsafe { &*slice_from_raw_parts(_sdriver as *const u8, _edriver as usize - _sdriver as usize) }
}
