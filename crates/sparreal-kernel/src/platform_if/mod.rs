pub use page_table_generic::Access;
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
#[api_trait]
pub trait MMU {
    /// 启动所需的内存范围
    ///
    /// # Safety
    ///
    /// `MMU` 开启以前，链接地址是物理地址，开启后，为虚拟地址，应在`MMU`开启前调用并保存，开启`MMU`后若调用会错误的使用虚拟地址作为返回值
    unsafe fn boot_regions() -> BootRsvRegionVec;
    fn set_kernel_table(addr: usize);
    fn get_kernel_table() -> usize;
    fn set_user_table(addr: usize);
    fn get_user_table() -> usize;

    /// flush tlb
    /// # Safety
    /// addr must be page aligned
    unsafe fn flush_tlb(addr: *const u8);
    fn flush_tlb_all();
    fn page_size() -> usize;
    fn table_level() -> usize;
    fn new_pte(config: PTEGeneric) -> usize;
    fn read_pte(pte: usize) -> PTEGeneric;
    fn enable_mmu(stack_top: usize, jump_to: usize) -> !;
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
