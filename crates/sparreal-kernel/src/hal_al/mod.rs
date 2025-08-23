pub use rdrive::{DeviceId, IrqId, register::DriverRegisterSlice};

pub use crate::irq::IrqParam;
use crate::mem::mmu::BootRegion;

pub mod mmu;
pub mod run;

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

#[trait_ffi::def_extern_trait(mod_path = "hal_al")]
pub trait Hal: mmu::Mmu {
    fn kstack_size() -> usize;
    fn cpu_id() -> usize;
    fn cpu_context_size() -> usize;

    fn boot_region_by_index(index: usize) -> Option<BootRegion>;

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
