use core::arch::asm;

use aarch64_cpu::registers::*;
use context::{__tcb_switch, Context};
use log::trace;
use somehal::{mem::MapRangeConfig, println};
use sparreal_kernel::{
    driver::IrqId,
    hal_al::{
        CacheOp, DeviceId, DriverRegisterSlice, Hal,
        mmu::{Access, Mmu, PageTable, PagingError},
    },
    impl_trait,
    irq::IrqParam,
    mem::{
        Phys, PhysAddr, Virt,
        mmu::{AccessSetting, BootMemoryKind, BootRegion, CacheSetting, RegionKind},
        region,
    },
    task::TaskControlBlock,
};

use crate::{consts, mem::driver_registers};
use aarch64_cpu_ext::cache;

mod boot;
mod context;
mod debug;
mod gic;
// mod paging;
mod power;
mod timer;
// mod trap;

// #[cfg(not(feature = "vm"))]
// pub fn is_mmu_enabled() -> bool {
//     SCTLR_EL1.matches_any(&[SCTLR_EL1::M::Enable])
// }
// #[cfg(feature = "vm")]
// pub fn is_mmu_enabled() -> bool {
//     SCTLR_EL2.matches_any(&[SCTLR_EL2::M::Enable])
// }

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

    fn new_table(alloc: &mut dyn Access) -> Result<PageTable, PagingError> {
        let mut baccess = BAccess(alloc);
        let table = somehal::mem::mmu::new_table(&mut baccess) ?;
        Ok(PageTable { id: 0, addr: table.paddr().raw().into() })
    }

    fn release_table(_table: PageTable) {
        todo!()
    }

    fn current_table() -> PageTable {
        let tb = somehal::mem::mmu::get_kernal_table();
        PageTable { id: tb.id, addr: tb.addr.into() }
    }

    fn switch_table(new_table: PageTable) {
        somehal::mem::mmu::set_kernal_table(map_table(new_table));
    }

    fn map_range(
        table: PageTable,
        alloc: &mut dyn Access,
        name: &'static str,
        va_start: Virt<u8>,
        pa_start: Phys<u8>,
        size: usize,
        access: AccessSetting,
        cache: CacheSetting,
    ) -> Result<(), PagingError> {
        let mut baccess = BAccess(alloc);


        let access = match access {
            AccessSetting::Read => somehal::mem::AccessKind::Read,
            AccessSetting::ReadWrite => somehal::mem::AccessKind::ReadWrite,
            AccessSetting::ReadExecute => somehal::mem::AccessKind::ReadExecute,
            AccessSetting::ReadWriteExecute => somehal::mem::AccessKind::ReadWriteExecute,
        };

        let mut cpu_share = true;

        let cache = match cache {
            CacheSetting::Normal => somehal::mem::CacheKind::Normal,
            CacheSetting::Device => somehal::mem::CacheKind::Device,
            CacheSetting::NonCacheable => somehal::mem::CacheKind::NoCache,
            CacheSetting::PerCpu => {
                cpu_share = false;
                somehal::mem::CacheKind::Normal
            },
        };

        let config = MapRangeConfig {
            vaddr: va_start.raw() as _,
            paddr: pa_start.raw(),
            size,
            name,
            access,
            cache,
            cpu_share,
        };
        somehal::mem::mmu::table_map(map_table(table), &mut baccess, config)?;
         Ok(())
    }
}
}

fn map_table(v: PageTable) -> somehal::mem::PageTable {
    somehal::mem::PageTable {
        id: v.id,
        addr: v.addr.into(),
    }
}

impl_trait! {
impl Hal for HalImpl {
    fn kstack_size() -> usize {
        todo!()
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
        todo!()
    }

    unsafe fn set_current_tcb_addr(addr: *mut u8) {
        todo!()
    }

    unsafe fn cpu_context_sp(ctx_ptr: *const u8) -> usize {
        todo!()
    }

    unsafe fn cpu_context_set_sp(ctx_ptr: *const u8, sp: usize) {
        todo!()
    }

    unsafe fn cpu_context_set_pc(ctx_ptr: *const u8, pc: usize) {
        todo!()
    }

    #[doc = " # Safety"]
    #[doc = ""]
    #[doc = ""]
    unsafe fn cpu_context_switch(prev_tcb: *mut u8, next_tcb: *mut u8) {
        todo!()
    }

    fn wait_for_interrupt() {

    }

    fn irq_init_current_cpu(id: DeviceId) {
        todo!()
    }

    fn irq_ack() -> IrqId {
        todo!()
    }

    fn irq_eoi(irq: IrqId) {
        todo!()
    }

    fn irq_all_enable() {
        todo!()
    }

    fn irq_all_disable() {

    }

    fn irq_all_is_enabled() -> bool {
        todo!()
    }

    fn irq_enable(config: IrqParam) {
        todo!()
    }

    fn irq_disable(id: DeviceId, irq: IrqId) {
        todo!()
    }

    fn shutdown() -> ! {
        somehal::power::shutdown()
    }

    fn debug_put(b: u8) {
        debug::put(b);
    }

    fn dcache_range(op: CacheOp, addr: usize, size: usize) {
        todo!()
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
