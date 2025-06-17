use core::arch::asm;

use aarch64_cpu::registers::*;
use pie_boot::BootArgs;
use sparreal_kernel::{globals::PlatformInfoKind, io::print::*, platform::shutdown, println};

use super::debug;
use crate::mem::{self, clean_bss};

#[pie_boot::entry]
fn rust_entry(args: &BootArgs) -> ! {
    clean_bss();
    set_trap();
    let text_va = args.kimage_start_vma - args.kimage_start_lma;
    let fdt = args.fdt_addr();

    unsafe {
        mem::mmu::set_text_va_offset(text_va);
        debug::setup_by_fdt(fdt, |r| r as _);
        stdout_use_debug();

        match CurrentEL.read(CurrentEL::EL) {
            1 => println!("EL1"),
            2 => println!("EL2"),
            3 => println!("EL3"),
            _ => unreachable!(),
        }

        let fdt = mem::save_fdt(fdt, args.pg_end);

        println!("FDT saved at: {fdt:?}",);

        let platform_info: PlatformInfoKind = if let Some(fdt) = fdt {
            PlatformInfoKind::new_fdt((fdt.as_ptr() as usize).into())
        } else {
            todo!()
        };

        if let Err(s) = sparreal_kernel::boot::start(text_va, platform_info) {
            println!("Boot start error: {s}");
        }
    }
    shutdown()
}

fn set_trap() {
    unsafe {
        asm!(
            "
        LDR      x0, =vector_table_el1
        MSR      VBAR_EL1, x0
        ",
        );
    }
}
