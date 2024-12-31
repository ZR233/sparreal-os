use core::ptr::{NonNull, slice_from_raw_parts};

use fdt_parser::Fdt;

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
pub fn set_addr(addr: NonNull<u8>) {
    if let Ok(fdt) = Fdt::from_ptr(addr) {
        unsafe {
            let data = &*slice_from_raw_parts(addr.as_ptr(), fdt.total_size());
            FDT = Some(data);
        }
    }
}
