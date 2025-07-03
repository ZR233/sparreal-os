use core::arch::asm;

use aarch64_cpu::registers::*;
use context::{__tcb_switch, Context};
use log::trace;
use sparreal_kernel::{platform_if::*, task::TaskControlBlock};

use crate::{consts, mem::driver_registers};
use aarch64_cpu_ext::cache;

mod boot;
// mod cache;
mod context;
mod debug;
mod gic;
mod paging;
mod power;
mod timer;
mod trap;

#[cfg(not(feature = "vm"))]
pub fn is_mmu_enabled() -> bool {
    SCTLR_EL1.matches_any(&[SCTLR_EL1::M::Enable])
}
#[cfg(feature = "vm")]
pub fn is_mmu_enabled() -> bool {
    SCTLR_EL2.matches_any(&[SCTLR_EL2::M::Enable])
}

struct PlatformImpl;

#[api_impl]
impl Platform for PlatformImpl {
    fn kstack_size() -> usize {
        consts::STACK_SIZE
    }

    fn cpu_id() -> usize {
        MPIDR_EL1.get() as usize & 0xff00ffffff
    }

    fn cpu_context_size() -> usize {
        size_of::<Context>()
    }

    unsafe fn cpu_context_sp(ctx_ptr: *const u8) -> usize {
        let ctx: &Context = unsafe { &*(ctx_ptr as *const Context) };
        ctx.sp as _
    }

    unsafe fn get_current_tcb_addr() -> *mut u8 {
        SP_EL0.get() as usize as _
    }

    unsafe fn set_current_tcb_addr(addr: *mut u8) {
        SP_EL0.set(addr as usize as _);
    }

    /// # Safety
    ///
    /// `ctx_ptr` 是有效的上下文指针
    unsafe fn cpu_context_set_sp(ctx_ptr: *const u8, sp: usize) {
        unsafe {
            let ctx = &mut *(ctx_ptr as *mut Context);
            ctx.sp = sp as _;
        }
    }

    /// # Safety
    ///
    /// `ctx_ptr` 是有效的上下文指针
    unsafe fn cpu_context_set_pc(ctx_ptr: *const u8, pc: usize) {
        unsafe {
            let ctx = &mut *(ctx_ptr as *mut Context);
            ctx.pc = pc as _;
            ctx.lr = pc as _;
        }
    }

    unsafe fn cpu_context_switch(prev_ptr: *mut u8, next_ptr: *mut u8) {
        let next = TaskControlBlock::from(next_ptr);
        trace!("switch to: {:?}", unsafe { &*(next.sp as *const Context) });
        unsafe { __tcb_switch(prev_ptr, next_ptr) };
    }

    fn wait_for_interrupt() {
        aarch64_cpu::asm::wfi();
    }

    fn shutdown() -> ! {
        // psci::system_off()
        loop {
            aarch64_cpu::asm::wfi();
        }
    }

    fn debug_put(b: u8) {
        debug::put(b);
    }

    fn irq_all_enable() {
        unsafe { asm!("msr daifclr, #2") };
    }
    fn irq_all_disable() {
        unsafe { asm!("msr daifset, #2") };
    }
    fn irq_all_is_enabled() -> bool {
        !DAIF.is_set(DAIF::I)
    }

    fn dcache_range(op: CacheOp, addr: usize, size: usize) {
        cache::dcache_range(
            match op {
                CacheOp::Invalidate => cache::CacheOp::Invalidate,
                CacheOp::Clean => cache::CacheOp::Clean,
                CacheOp::CleanAndInvalidate => cache::CacheOp::CleanAndInvalidate,
            },
            addr,
            size,
        );
    }

    fn driver_registers() -> DriverRegisterSlice {
        DriverRegisterSlice::from_raw(driver_registers())
    }
}
