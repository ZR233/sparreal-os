use core::{arch::asm, ptr::NonNull};

use aarch64_cpu::{asm::barrier::*, registers::*};
use page_table_arm::*;
use sparreal_kernel::{mem::PhysAddr, platform_if::*, println};

use crate::mem::fdt_addr;

use super::cache;

pub struct PageTableImpl;

#[api_impl]
impl MMU for PageTableImpl {
    unsafe fn boot_regions() -> BootRsvRegionVec {
        let mut ret = crate::mem::rsv_regions();
        let debug_reg = PhysAddr::new(super::debug::reg()).align_down(0x1000);

        ret.push(BootRegion::new(
            debug_reg..debug_reg + 0x1000,
            c"debug_uart",
            AccessSetting::Read | AccessSetting::Write,
            CacheSetting::Device,
            RegionKind::Other,
        ));

        ret
    }

    unsafe fn flush_tlb(addr: *const u8) {
        unsafe { asm!("tlbi vaae1is, {}; dsb nsh; isb", in(reg) addr as usize) };
    }

    fn flush_tlb_all() {
        unsafe { asm!("tlbi vmalle1; dsb nsh; isb") };
    }

    fn page_size() -> usize {
        0x1000
    }

    fn table_level() -> usize {
        4
    }

    fn new_pte(config: PTEGeneric) -> usize {
        let mut pte = PTE::from_paddr(config.paddr);
        let mut flags = PTEFlags::empty();

        if config.is_valid {
            flags |= PTEFlags::VALID;
        }

        if !config.is_block {
            flags |= PTEFlags::NON_BLOCK;
        }

        pte.set_mair_idx(MAIRDefault::get_idx(match config.setting.cache_setting {
            CacheSetting::Normal => MAIRKind::Normal,
            CacheSetting::Device => MAIRKind::Device,
            CacheSetting::NonCache => MAIRKind::NonCache,
        }));

        let privilege = &config.setting.privilege_access;

        if !config.setting.is_global {
            flags |= PTEFlags::NG;
        }

        if privilege.readable() {
            flags |= PTEFlags::AF;
        }

        if !privilege.writable() {
            flags |= PTEFlags::AP_RO;
        }

        if !privilege.executable() {
            flags |= PTEFlags::PXN;
        }

        let user = &config.setting.user_access;

        if user.readable() {
            flags |= PTEFlags::AP_EL0;
        }

        if user.writable() {
            flags |= PTEFlags::AP_EL0;
            flags.remove(PTEFlags::AP_RO);
        }

        if !user.executable() {
            flags |= PTEFlags::UXN;
        }

        pte.set_flags(flags);

        let out: u64 = pte.into();

        out as _
    }

    fn read_pte(pte: usize) -> PTEGeneric {
        let pte = PTE::from(pte as u64);
        let paddr = pte.paddr();
        let flags = pte.get_flags();
        let is_valid = flags.contains(PTEFlags::VALID);
        let is_block = !flags.contains(PTEFlags::NON_BLOCK);
        let mut privilege_access = AccessSetting::empty();
        let mut user_access = AccessSetting::empty();
        let mut cache_setting = CacheSetting::Normal;
        let is_global = !flags.contains(PTEFlags::NG);

        if is_valid {
            let mair_idx = pte.get_mair_idx();

            cache_setting = match MAIRDefault::from_idx(mair_idx) {
                MAIRKind::Device => CacheSetting::Device,
                MAIRKind::Normal => CacheSetting::Normal,
                MAIRKind::NonCache => CacheSetting::NonCache,
            };

            if flags.contains(PTEFlags::AF) {
                privilege_access |= AccessSetting::Read;
            }

            if !flags.contains(PTEFlags::AP_RO) {
                privilege_access |= AccessSetting::Write;
            }

            if !flags.contains(PTEFlags::PXN) {
                privilege_access |= AccessSetting::Execute;
            }

            if flags.contains(PTEFlags::AP_EL0) {
                user_access |= AccessSetting::Read;

                if !flags.contains(PTEFlags::AP_RO) {
                    user_access |= AccessSetting::Write;
                }
            }

            if !flags.contains(PTEFlags::UXN) {
                user_access |= AccessSetting::Execute;
            }
        }

        PTEGeneric {
            paddr,
            is_block,
            is_valid,
            setting: PTESetting {
                is_global,
                privilege_access,
                user_access,
                cache_setting,
            },
        }
    }

    fn set_kernel_table(addr: usize) {
        TTBR1_EL1.set_baddr(addr as _);
        // Self::flush_tlb_all();
    }

    fn get_kernel_table() -> usize {
        TTBR1_EL1.get_baddr() as _
    }

    fn set_user_table(addr: usize) {
        TTBR0_EL1.set_baddr(addr as _);
        // Self::flush_tlb_all();
    }

    fn get_user_table() -> usize {
        TTBR0_EL1.get_baddr() as _
    }

    fn enable_mmu(stack_top: usize, jump_to: usize) -> ! {
        MAIRDefault::mair_el1_apply();
        cache::dcache_all(CacheOp::CleanAndInvalidate);

        println!("TCR_EL1: {}", TCR_EL1.get());
        unsafe {
            super::debug::setup_by_fdt(
                Some(NonNull::new(fdt_addr().unwrap().raw() as _).unwrap()),
                |r| (r + RegionKind::Other.va_offset()) as _,
            );

            MMUImpl::flush_tlb_all();

            isb(SY);
            let tmp = 0usize;
            asm!(
                "MOV      sp,  {stack}",
                "MOV      {tmp},  {entry}",
                "BLR      {tmp}",
                "B       .",
                stack = in(reg) stack_top,
                entry = in(reg) jump_to,
                tmp = in(reg) tmp,
                options(nomem, nostack,noreturn)
            )
        }
    }
}
