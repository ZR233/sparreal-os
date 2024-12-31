use core::ptr::NonNull;

use crate::{io, mem, platform::PlatformImpl, println};

pub mod debug;

pub struct BootInfo {
    pub va_offset: usize,
    pub device_info_kind: PlatformInfoKind,
    pub stack: &'static [u8],
    pub kernel: &'static [u8],
    pub heap: &'static [u8],
}

pub enum PlatformInfoKind {
    DeviceTree { addr: NonNull<u8> },
}

pub unsafe fn preper(info: BootInfo) {
    unsafe {
        io::print::stdout_use_debug();

        println!(
            "kernel @{:p} => {:#x}",
            info.kernel.as_ptr(),
            info.kernel.as_ptr() as usize - info.va_offset
        );

        println!(
            "stack @{:p} => {:#x}",
            info.stack.as_ptr(),
            info.stack.as_ptr() as usize - info.va_offset
        );

        println!(
            "heap @{:p} => {:#x}",
            info.heap.as_ptr(),
            info.heap.as_ptr() as usize - info.va_offset
        );

        mem::init(&info);
    }
}
