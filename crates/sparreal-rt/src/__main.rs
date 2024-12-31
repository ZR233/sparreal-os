use core::{hint::spin_loop, ptr::NonNull};

use sparreal_kernel::{
    boot::{self, BootInfo, PlatformInfoKind},
    start,
};

use crate::mem::{self, get_fdt};

use self::mem::{get_fdt_data, kernel_data, kernel_stack, va_offset};

pub extern "C" fn __rust_main() -> ! {
    crate::debug::put(b'D');
    crate::debug::put(b'\n');
    crate::debug::put(b'e');
    crate::debug::put(b'\n');

    let info = BootInfo {
        va_offset: va_offset(),
        device_info_kind: PlatformInfoKind::DeviceTree {
            addr: NonNull::new(get_fdt_data().unwrap().as_ptr() as usize as _).unwrap(),
        },
        stack: kernel_stack(),
        kernel: kernel_data(),
    };

    unsafe {
        boot::preper(info);

        start()
    }
}
