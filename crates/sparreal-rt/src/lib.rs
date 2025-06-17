#![no_std]
#![no_main]
#![feature(concat_idents)]
#![feature(used_with_arg)]

extern crate alloc;

#[cfg_attr(target_arch = "aarch64", path = "arch/aarch64/mod.rs")]
pub mod arch;
pub(crate) mod consts;
pub(crate) mod mem;
pub mod prelude;
