use core::arch::asm;

use aarch64_cpu::registers::*;
use aarch64_cpu_ext::cache;
use context::Context;
use log::{trace, warn};
use somehal::mem::MapRangeConfig;
use sparreal_kernel::{
    driver::IrqId,
    hal_al::{
        CacheOp, DeviceId, DriverRegisterSlice, Hal,
        mmu::{Access, MapConfig, Mmu, PageTableRef, PagingError},
    },
    impl_trait,
    irq::IrqParam,
    mem::mmu::{AccessSetting, BootRegion, CacheSetting},
    task::TaskControlBlock,
};

use crate::{
    arch::context::__tcb_switch,
    mem::{driver_registers, stack_cpu0},
};

mod boot;
mod context;
mod debug;
mod gic;
// mod paging;
mod power;
mod timer;
mod trap;

struct BAccess<'a>(&'a mut dyn Access);
impl Access for BAccess<'_> {
    unsafe fn alloc(
        &mut self,
        layout: core::alloc::Layout,
    ) -> Option<sparreal_kernel::hal_al::mmu::PhysAddr> {
        unsafe { self.0.alloc(layout) }
    }

    unsafe fn dealloc(
        &mut self,
        ptr: sparreal_kernel::hal_al::mmu::PhysAddr,
        layout: core::alloc::Layout,
    ) {
        unsafe { self.0.dealloc(ptr, layout) }
    }

    fn phys_to_mut(&self, phys: sparreal_kernel::hal_al::mmu::PhysAddr) -> *mut u8 {
        self.0.phys_to_mut(phys)
    }
}

struct HalImpl;

impl_trait! {

impl Mmu for HalImpl {
    fn setup() {
        somehal::mem::init();
    }

    fn page_size() -> usize {
        somehal::mem::page_size()
    }

    fn kimage_va_offset() -> usize {
        crate::mem::va_offset()
    }

    fn new_table(alloc: &mut dyn Access) -> Result<PageTableRef, PagingError> {
        let mut baccess = BAccess(alloc);
        let table = somehal::mem::mmu::new_table(&mut baccess) ?;
        Ok(PageTableRef { id: 0, addr: table.paddr().raw().into() })
    }

    fn release_table(_table: PageTableRef, alloc: &mut dyn Access) {
        let mut baccess = BAccess(alloc);
        warn!("release_table is not implemented"); // TODO
    }

    fn get_kernel_table() -> PageTableRef {
        let tb = somehal::mem::mmu::get_kernal_table();
        PageTableRef { id: tb.id, addr: tb.addr.into() }
    }

    fn set_kernel_table(new_table: PageTableRef) {
        somehal::mem::mmu::set_kernal_table(map_table(new_table));
    }

    fn table_map(
        table: PageTableRef,
        alloc: &mut dyn Access,
        config: &MapConfig,
    ) -> Result<(), PagingError> {
        let mut baccess = BAccess(alloc);

        let access = match config.access {
            AccessSetting::Read => somehal::mem::AccessKind::Read,
            AccessSetting::ReadWrite => somehal::mem::AccessKind::ReadWrite,
            AccessSetting::ReadExecute => somehal::mem::AccessKind::ReadExecute,
            AccessSetting::ReadWriteExecute => somehal::mem::AccessKind::ReadWriteExecute,
        };

        let mut cpu_share = true;

        let cache = match config.cache {
            CacheSetting::Normal => somehal::mem::CacheKind::Normal,
            CacheSetting::Device => somehal::mem::CacheKind::Device,
            CacheSetting::NonCacheable => somehal::mem::CacheKind::NoCache,
            CacheSetting::PerCpu => {
                cpu_share = false;
                somehal::mem::CacheKind::Normal
            },
        };

        let config = MapRangeConfig {
            vaddr: config.va_start.raw() as _,
            paddr: config.pa_start.raw(),
            size: config.size,
            name: config.name,
            access,
            cache,
            cpu_share,
        };
        somehal::mem::mmu::table_map(map_table(table), &mut baccess, config)?;
         Ok(())
    }
}
}

fn map_table(v: PageTableRef) -> somehal::mem::PageTable {
    somehal::mem::PageTable {
        id: v.id,
        addr: v.addr.into(),
    }
}

impl_trait! {
impl Hal for HalImpl {
    fn kstack_size() -> usize {
        stack_cpu0().len()
    }

    fn cpu_id() -> usize {
        MPIDR_EL1.get() as usize & 0xff00ffffff
    }

    fn cpu_context_size() -> usize {
        size_of::<Context>()
    }

    fn boot_region_by_index(index: usize) -> Option<BootRegion>{
        crate::mem::boot_regions().get(index).cloned()
    }

    unsafe fn get_current_tcb_addr() -> *mut u8 {
        SP_EL0.get() as usize as _
    }

    unsafe fn set_current_tcb_addr(addr: *mut u8) {
        SP_EL0.set(addr as usize as _);
    }

    unsafe fn cpu_context_sp(ctx_ptr: *const u8) -> usize {
        let ctx: &Context = unsafe { &*(ctx_ptr as *const Context) };
        ctx.sp as _
    }

    unsafe fn cpu_context_set_sp(ctx_ptr: *const u8, sp: usize) {
        unsafe {
            let ctx = &mut *(ctx_ptr as *mut Context);
            ctx.sp = sp as _;
        }
    }

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

    fn irq_init_current_cpu(id: DeviceId) {
        gic::init_current_cpu(id);
    }

    fn irq_ack() -> IrqId {
        gic::ack()
    }

    fn irq_eoi(irq: IrqId) {
        gic::eoi(irq);
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

    fn irq_enable(config: IrqParam) {
        gic::irq_enable(config);
    }

    fn irq_disable(id: DeviceId, irq: IrqId) {
        gic::irq_disable(id, irq);
    }

    fn shutdown() -> ! {
        somehal::power::shutdown()
    }

    fn debug_put(b: u8) {
        debug::put(b);
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
}

// struct PlatformImpl;

// #[api_impl]
// impl Platform for PlatformImpl {
//     fn kstack_size() -> usize {
//         consts::STACK_SIZE
//     }

//     fn cpu_id() -> usize {
//         MPIDR_EL1.get() as usize & 0xff00ffffff
//     }

//     fn cpu_context_size() -> usize {
//         size_of::<Context>()
//     }

//     unsafe fn cpu_context_sp(ctx_ptr: *const u8) -> usize {
//         let ctx: &Context = unsafe { &*(ctx_ptr as *const Context) };
//         ctx.sp as _
//     }

//     unsafe fn get_current_tcb_addr() -> *mut u8 {
//         SP_EL0.get() as usize as _
//     }

//     unsafe fn set_current_tcb_addr(addr: *mut u8) {
//         SP_EL0.set(addr as usize as _);
//     }

//     /// # Safety
//     ///
//     /// `ctx_ptr` 是有效的上下文指针
//     unsafe fn cpu_context_set_sp(ctx_ptr: *const u8, sp: usize) {
//         unsafe {
//             let ctx = &mut *(ctx_ptr as *mut Context);
//             ctx.sp = sp as _;
//         }
//     }

//     /// # Safety
//     ///
//     /// `ctx_ptr` 是有效的上下文指针
//     unsafe fn cpu_context_set_pc(ctx_ptr: *const u8, pc: usize) {
//         unsafe {
//             let ctx = &mut *(ctx_ptr as *mut Context);
//             ctx.pc = pc as _;
//             ctx.lr = pc as _;
//         }
//     }

//     unsafe fn cpu_context_switch(prev_ptr: *mut u8, next_ptr: *mut u8) {
//         let next = TaskControlBlock::from(next_ptr);
//         trace!("switch to: {:?}", unsafe { &*(next.sp as *const Context) });
//         unsafe { __tcb_switch(prev_ptr, next_ptr) };
//     }

//     fn wait_for_interrupt() {
//         aarch64_cpu::asm::wfi();
//     }

//     fn shutdown() -> ! {
//         // psci::system_off()
//         loop {
//             aarch64_cpu::asm::wfi();
//         }
//     }

//     fn debug_put(b: u8) {
//         debug::put(b);
//     }

//     fn irq_init_current_cpu(id: DeviceId) {
//         gic::init_current_cpu(id);
//     }

//     fn irq_ack() -> IrqId {
//         gic::ack()
//     }
//     fn irq_eoi(irq: IrqId) {
//         gic::eoi(irq);
//     }

//     fn irq_all_enable() {
//         unsafe { asm!("msr daifclr, #2") };
//     }
//     fn irq_all_disable() {
//         unsafe { asm!("msr daifset, #2") };
//     }
//     fn irq_all_is_enabled() -> bool {
//         !DAIF.is_set(DAIF::I)
//     }

//     fn irq_enable(config: IrqParam) {
//         gic::irq_enable(config);
//     }

//     fn irq_disable(id: DeviceId, irq: IrqId) {
//         gic::irq_disable(id, irq);
//     }

//     fn dcache_range(op: CacheOp, addr: usize, size: usize) {
//         cache::dcache_range(
//             match op {
//                 CacheOp::Invalidate => cache::CacheOp::Invalidate,
//                 CacheOp::Clean => cache::CacheOp::Clean,
//                 CacheOp::CleanAndInvalidate => cache::CacheOp::CleanAndInvalidate,
//             },
//             addr,
//             size,
//         );
//     }

//     fn driver_registers() -> DriverRegisterSlice {
//         DriverRegisterSlice::from_raw(driver_registers())
//     }
// }
