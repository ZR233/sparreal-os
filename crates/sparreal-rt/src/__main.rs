use core::{hint::spin_loop, ptr::NonNull};

use sparreal_kernel::{
    boot::{BootInfo, PlatformInfoKind},
    boot_preper,
    mem::boot_heap_init,
    start,
};

use crate::mem::{self, get_fdt};

use self::mem::{get_fdt_data, va_offset};

pub extern "C" fn __rust_main() -> ! {
    crate::debug::put(b'D');
    crate::debug::put(b'\n');

    let fdt = get_fdt_data().unwrap().as_ptr() as usize as _;
    crate::debug::put(b'e');
    crate::debug::put(b'\n');

    let info = BootInfo {
        va_offset: va_offset(),
        device_info_kind: PlatformInfoKind::DeviceTree {
            addr: NonNull::new(fdt).unwrap(),
        },
        stack: todo!(),
        kernel: todo!(),
    };

    boot_preper(info);

    start()
}
