use core::sync::atomic::{Ordering, fence};

use super::__start;
use crate::{
    globals::{self, global_val},
    io::print::*,
    mem::{mmu::*, region::init_boot_rsv_region, stack_top},
    platform::{PlatformInfoKind, regsions},
    platform_if::MMUImpl,
    println,
};

pub fn start(text_va_offset: usize, platform_info: PlatformInfoKind) -> Result<(), &'static str> {
    println!("Booting up");
    unsafe {
        init_boot_rsv_region();
    }

    if let Err(e) = unsafe { globals::setup(platform_info) } {
        println!("setup globle error: {e}");
    }
    let table = new_boot_table()?;

    fence(Ordering::SeqCst);

    set_user_table(table);
    set_kernel_table(table);

    let stack_top = stack_top();

    let jump_to = __start as usize;

    println!("begin enable mmu");

    println!("Jump to __start: {jump_to:#x}, stack top: {stack_top:#x}");

    fence(Ordering::SeqCst);
    MMUImpl::enable_mmu(stack_top, jump_to)
}
