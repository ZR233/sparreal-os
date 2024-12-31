use core::{
    arch::{asm, global_asm},
    hint::spin_loop,
    ptr::{NonNull, slice_from_raw_parts, slice_from_raw_parts_mut},
};

use crate::mem::{self, PageAllocator, clear_bss, get_fdt, get_fdt_data, move_dtb};
use aarch64_cpu::{
    asm::{
        self,
        barrier::{NSH, SY, dsb, isb},
        ret,
    },
    registers::*,
};
use alloc::{format, string::ToString};
use fdt_parser::Fdt;
use page_table_arm::MAIRDefault;
use page_table_generic::{AccessSetting, CacheSetting, MapConfig, PageTableRef};

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("vectors.s"));

pub fn write_bytes(s: &[u8]) {
    for ch in s {
        crate::debug::put(*ch);
    }
}

#[unsafe(no_mangle)]
extern "C" fn __rust_boot(va_offset: usize, fdt_addr: usize) {
    unsafe {
        clear_bss();
        asm!("tlbi vmalle1");
        dsb(NSH);

        move_dtb(fdt_addr as _);

        let fdt = if let Some(fdt) = get_fdt() {
            fdt
        } else {
            panic!("fdt is not found");
        };

        crate::debug::init_by_fdt(&fdt);

        let table = if let Some(table) = new_boot_table(va_offset, &fdt) {
            table
        } else {
            panic!("boot table is not found");
        };

        MAIRDefault::mair_el1_apply();

        // Enable TTBR0 and TTBR1 walks, page size = 4K, vaddr size = 48 bits, paddr size = 40 bits.
        let tcr_flags0 = TCR_EL1::EPD0::EnableTTBR0Walks
            + TCR_EL1::TG0::KiB_4
            + TCR_EL1::SH0::Inner
            + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::T0SZ.val(16);
        let tcr_flags1 = TCR_EL1::EPD1::EnableTTBR1Walks
            + TCR_EL1::TG1::KiB_4
            + TCR_EL1::SH1::Inner
            + TCR_EL1::ORGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::T1SZ.val(16);
        TCR_EL1.write(TCR_EL1::IPS::Bits_48 + tcr_flags0 + tcr_flags1);

        isb(SY);
        // Set both TTBR0 and TTBR1
        TTBR1_EL1.set_baddr(table);
        TTBR0_EL1.set_baddr(table);

        isb(SY);

        crate::debug::put(b'B');

        crate::debug::mmu_add_offset(va_offset);
        crate::mem::set_va_offset(va_offset);
        // Enable the MMU and turn on I-cache and D-cache
        SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);
        isb(SY);
        asm!("ic iallu");
        dsb(NSH);
        isb(SY);
        crate::debug::put(b'C');
        asm!("
    LDR      x9, =_stack_top
    MOV      sp,  x9
    LDR      x8, ={entry}
    BLR      x8
    B       .
    ", 
        entry = sym crate::__main::__rust_main,
        options(noreturn)
        )
    }
}

fn new_boot_table(va_offset: usize, fdt: &Fdt<'_>) -> Option<u64> {
    let node_memory = fdt.memory().next().unwrap();
    let main_memory = fdt.memory().next().unwrap().regions().next().unwrap();
    let start = main_memory.address as usize;
    let end = start + main_memory.size as usize;

    let table =
        sparreal_kernel::mem::mmu::new_boot_table(va_offset, start..end, crate::debug::reg_base())
            .unwrap();

    Some(table as _)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __switch_to_el1() {
    SPSel.write(SPSel::SP::ELx);
    SP_EL0.set(0);
    let current_el = CurrentEL.read(CurrentEL::EL);
    if current_el >= 2 {
        if current_el == 3 {
            // Set EL2 to 64bit and enable the HVC instruction.
            SCR_EL3.write(
                SCR_EL3::NS::NonSecure + SCR_EL3::HCE::HvcEnabled + SCR_EL3::RW::NextELIsAarch64,
            );
            // Set the return address and exception level.
            SPSR_EL3.write(
                SPSR_EL3::M::EL1h
                    + SPSR_EL3::D::Masked
                    + SPSR_EL3::A::Masked
                    + SPSR_EL3::I::Masked
                    + SPSR_EL3::F::Masked,
            );
            unsafe {
                asm!(
                    "
            adr      x2, _start_boot
            msr elr_el3, x2
            "
                );
            }
        }
        // Disable EL1 timer traps and the timer offset.
        CNTHCTL_EL2.modify(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);
        CNTVOFF_EL2.set(0);
        // Set EL1 to 64bit.
        HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);
        // Set the return address and exception level.
        SPSR_EL2.write(
            SPSR_EL2::M::EL1h
                + SPSR_EL2::D::Masked
                + SPSR_EL2::A::Masked
                + SPSR_EL2::I::Masked
                + SPSR_EL2::F::Masked,
        );

        asm!(
            "
            mov     x8, sp
            msr     sp_el1, x8
            MOV      x0, x19
            adr      x2, _el1_entry
            msr      elr_el2, x2
            eret
            "
        );
    } else {
        asm!("bl _el1_entry")
    }
}
