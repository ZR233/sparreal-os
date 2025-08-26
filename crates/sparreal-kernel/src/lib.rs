#![no_std]
#![feature(linkage)]
#![feature(fn_align)]
#![feature(allocator_api)]

extern crate alloc;
#[macro_use]
extern crate log;

pub use rdrive::module_driver;

#[macro_use]
mod logger;

pub mod __export;
pub mod boot;
pub mod globals;
pub mod io;

pub mod async_std;
pub mod driver;
pub mod hal_al;
pub mod irq;
mod lang_items;

pub mod mem;
pub mod platform;
// pub mod platform_if;
pub mod prelude;
pub mod task;
pub mod time;

pub use mem::Address;
