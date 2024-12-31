#![no_std]
#![no_main]
#![feature(naked_functions)]

extern crate alloc;

#[macro_use]
extern crate sparreal_kernel;

pub(crate) mod __main;
#[cfg_attr(target_arch = "aarch64", path = "arch/aarch64/mod.rs")]
pub mod arch;
mod config;
mod debug;
pub(crate) mod mem;
pub mod prelude;
