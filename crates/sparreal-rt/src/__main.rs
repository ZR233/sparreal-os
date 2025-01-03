use core::{hint::spin_loop, ptr::NonNull};

use sparreal_kernel::{
    boot::{self, BootInfo, PlatformInfoKind},
    start,
};

use crate::{
    debug::put,
    mem::{self, get_fdt},
};

use self::mem::{boot_heap, get_fdt_data, kernel_data, kernel_stack, va_offset};

pub extern "C" fn __rust_main() -> ! {
    let info = BootInfo {
        va_offset: va_offset(),
        device_info_kind: PlatformInfoKind::DeviceTree {
            addr: NonNull::new(get_fdt_data().unwrap().as_ptr() as usize as _).unwrap(),
        },
        stack: kernel_stack(),
        kernel: kernel_data(),
        heap: boot_heap(),
    };

    unsafe {
        boot::preper(info);

        println!("Hello, world!");

        start()
    }
}
