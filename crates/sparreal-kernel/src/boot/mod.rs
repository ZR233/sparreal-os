use core::ptr::NonNull;

use crate::mem;

pub mod debug;

pub struct BootInfo {
    pub va_offset: usize,
    pub device_info_kind: PlatformInfoKind,
    pub stack: &'static [u8],
    pub kernel: &'static [u8],
}

pub enum PlatformInfoKind {
    DeviceTree { addr: NonNull<u8> },
}

pub unsafe fn preper(info: BootInfo) {
    unsafe {
        mem::init(&info);
    }
}
