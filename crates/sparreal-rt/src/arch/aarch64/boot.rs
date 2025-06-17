use core::arch::asm;

use aarch64_cpu::registers::*;
use pie_boot::BootArgs;
use sparreal_kernel::{globals::PlatformInfoKind, io::print::*, platform::shutdown};

use super::debug;
use crate::mem::{self, clean_bss};

#[pie_boot::entry]
fn rust_entry(args: &BootArgs) -> ! {
    let args = args.clone();
    clean_bss();
    let text_va = args.kimage_start_vma - args.kimage_start_lma;
    let fdt = args.fdt_addr();

    unsafe {
        mem::mmu::set_text_va_offset(text_va);
        debug::setup_by_fdt(fdt, |r| r as _);
        asm!(
            "
        LDR      x0, =vector_table_el1
        MSR      VBAR_EL1, x0
        ",
        );
    }
    match CurrentEL.read(CurrentEL::EL) {
        1 => early_dbgln("EL1"),
        2 => early_dbgln("EL2"),
        3 => early_dbgln("EL3"),
        _ => unreachable!(),
    }

    unsafe {
        let fdt = mem::save_fdt(fdt);

        let platform_info: PlatformInfoKind = if let Some(addr) = fdt {
            PlatformInfoKind::new_fdt((addr.as_ptr() as usize).into())
        } else {
            todo!()
        };

        if let Err(s) = sparreal_kernel::boot::start(text_va, platform_info) {
            early_dbgln(s);
        }
    }
    shutdown()
}
