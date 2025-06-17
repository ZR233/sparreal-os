use core::arch::{asm, naked_asm};

use aarch64_cpu::registers::*;
use pie_boot::BootArgs;
use sparreal_kernel::{globals::PlatformInfoKind, io::print::*, platform::shutdown, println};

use super::debug;
use crate::mem::{self, clean_bss};

#[pie_boot::entry]
fn rust_entry(args: &BootArgs) -> ! {
    clean_bss();
    set_trap();
    unsafe {
        asm!(
            "mov x0, {args}",
            "bl {switch_sp}",
            args = in(reg) args,
            switch_sp = sym switch_sp,
        )
    }
}

fn sp_fixed_entry(args: &BootArgs) -> ! {
    let text_va = args.kimage_start_vma - args.kimage_start_lma;
    let fdt = args.fdt_addr();

    unsafe {
        mem::mmu::set_text_va_offset(text_va);
        debug::setup_by_fdt(fdt, |r| r as _);
        stdout_use_debug();
        let sp: usize;
        asm!(
            "mov {}, sp",
            out(reg) sp,
        );
        println!("SP: {sp:#x}");

        match CurrentEL.read(CurrentEL::EL) {
            1 => println!("EL1"),
            2 => println!("EL2"),
            3 => println!("EL3"),
            _ => unreachable!(),
        }
        mem::setup_boot_args(args);
        println!("FDT: {fdt:?}",);

        let platform_info: PlatformInfoKind = if !fdt.is_null() {
            PlatformInfoKind::new_fdt((fdt as usize - text_va).into())
        } else {
            todo!()
        };

        if let Err(s) = sparreal_kernel::boot::start(text_va, platform_info) {
            println!("Boot start error: {s}");
        }
    }
    shutdown()
}

#[unsafe(naked)]
unsafe extern "C" fn switch_sp(_args: usize) -> ! {
    naked_asm!(
        "
        ldr     x8, =_stack_top
        mov     sp, x8
        bl      {}
        ",
        sym sp_fixed_entry,
    )
}

fn set_trap() {
    unsafe {
        asm!(
            "
        LDR      {0}, =vector_table_el1
        MSR      VBAR_EL1, {0}
        ",
            out(reg) _,
        );
    }
}
