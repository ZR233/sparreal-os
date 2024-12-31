#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod __export;
mod addr;
pub mod boot;
pub mod io;
mod lang_items;
mod logger;
pub mod mem;
pub mod platform;
pub mod prelude;
pub mod time;

use core::hint::spin_loop;

pub use addr::Address;
use boot::BootInfo;
use platform::PlatformImpl;

pub fn start() -> ! {
    loop {
        PlatformImpl::wait_for_interrupt();
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::__export::print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*));
    };
}
