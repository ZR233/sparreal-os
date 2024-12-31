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
}
