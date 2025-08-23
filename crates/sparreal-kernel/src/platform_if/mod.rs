pub use page_table_generic::Access;
#[cfg(feature = "mmu")]
use page_table_generic::{AccessSetting, CacheSetting, err::PagingError};
pub use rdrive::register::DriverRegisterSlice;
pub use rdrive::{DeviceId, IrqId};
pub use sparreal_macros::api_impl;
use sparreal_macros::api_trait;

pub use crate::irq::IrqParam;
pub use crate::mem::region::BootRsvRegionVec;

#[api_trait]
pub trait Platform {
    fn kstack_size() -> usize;
    fn cpu_id() -> usize;
    fn cpu_context_size() -> usize;

    /// # Safety
    ///
    ///
    unsafe fn get_current_tcb_addr() -> *mut u8;

    /// # Safety
    ///
    ///
    unsafe fn set_current_tcb_addr(addr: *mut u8);

    /// # Safety
    ///
    /// `ctx_ptr` 是有效的上下文指针
    unsafe fn cpu_context_sp(ctx_ptr: *const u8) -> usize;

    /// # Safety
    ///
    /// `ctx_ptr` 是有效的上下文指针
    unsafe fn cpu_context_set_sp(ctx_ptr: *const u8, sp: usize);

    /// # Safety
    ///
    /// `ctx_ptr` 是有效的上下文指针
    unsafe fn cpu_context_set_pc(ctx_ptr: *const u8, pc: usize);

    /// # Safety
    ///
    ///
    unsafe fn cpu_context_switch(prev_tcb: *mut u8, next_tcb: *mut u8);

    fn wait_for_interrupt();

    fn irq_init_current_cpu(id: DeviceId);

    fn irq_ack() -> IrqId;
    fn irq_eoi(irq: IrqId);

    fn irq_all_enable();
    fn irq_all_disable();
    fn irq_all_is_enabled() -> bool;

    fn irq_enable(config: IrqParam);
    fn irq_disable(id: DeviceId, irq: IrqId);

    fn shutdown() -> !;
    fn debug_put(b: u8);

    fn dcache_range(op: CacheOp, addr: usize, size: usize);

    fn driver_registers() -> DriverRegisterSlice;
}

#[cfg(feature = "mmu")]
pub use crate::mem::mmu::*;
#[cfg(feature = "mmu")]
use crate::mem::{Phys, Virt};

#[cfg(feature = "mmu")]
#[api_trait]
pub trait MMU {
    /// Called once after memory management is ready.
    fn setup();

    fn new_table() -> Phys<u8>;
    fn release_table(table_addr: Phys<u8>);
    fn current_table_addr() -> Phys<u8>;
    fn switch_table(new_table_addr: Phys<u8>);
    fn map_range(
        table_addr: Phys<u8>,
        va_start: Virt<u8>,
        pa_start: Virt<u8>,
        size: usize,
        access: AccessSetting,
        cache: CacheSetting,
    ) -> Result<(), PagingError>;
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum CacheOp {
    /// Write back to memory
    Clean,
    /// Invalidate cache
    Invalidate,
    /// Clean and invalidate
    CleanAndInvalidate,
}
