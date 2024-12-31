#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod __export;
pub mod boot;
mod lang_items;
pub mod mem;
pub mod platform;
pub mod prelude;

use core::hint::spin_loop;

use boot::BootInfo;
use platform::PlatformImpl;

pub fn boot_preper(info: BootInfo) {}

pub fn start() -> ! {
    loop {
        PlatformImpl::wait_for_interrupt();
    }
}

#[macro_export]
macro_rules! bootdbg {
    ($($arg:tt)*) => {
        let s = alloc::format!($($arg)*);
        $crate::boot::debug::write_str(&s);
    }
}
