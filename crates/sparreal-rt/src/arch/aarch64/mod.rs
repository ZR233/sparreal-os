use aarch64_cpu::registers::*;
use sparreal_kernel::platform::Platform;
use sparreal_macros::api_impl;

mod boot;
pub(crate) mod mmu;
mod trap;

struct PlatformImpl;

#[api_impl]
impl Platform for PlatformImpl {
    fn wait_for_interrupt() {
        aarch64_cpu::asm::wfi();
    }

    fn debug_put(b: u8) {
        crate::debug::put(b);
    }

    fn current_ticks() -> u64 {
        CNTPCT_EL0.get()
    }

    fn tick_hz() -> u64 {
        CNTFRQ_EL0.get()
    }
}
