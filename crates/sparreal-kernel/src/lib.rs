#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod __export;
pub mod boot;
mod lang_items;
// mod logger;
pub mod mem;
pub mod platform;
pub mod prelude;

use core::hint::spin_loop;

use boot::BootInfo;
use platform::PlatformImpl;


pub fn start() -> ! {
    loop {
        PlatformImpl::wait_for_interrupt();
    }
}
