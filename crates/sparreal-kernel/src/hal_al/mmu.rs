use core::fmt::Debug;

pub use page_table_generic::{Access, PagingError, PhysAddr};

pub use crate::mem::{Phys, Virt};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccessSetting {
    Read,
    ReadWrite,
    ReadExecute,
    ReadWriteExecute,
}
impl core::fmt::Debug for AccessSetting {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AccessSetting::Read => write!(f, "R--"),
            AccessSetting::ReadWrite => write!(f, "RW-"),
            AccessSetting::ReadExecute => write!(f, "R-X"),
            AccessSetting::ReadWriteExecute => write!(f, "RWX"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheSetting {
    /// Normal memory, cacheable, write-back
    Normal,
    /// Device memory, non-cacheable
    Device,
    /// Non-cacheable memory, strongly ordered
    NonCacheable,
    /// Per-CPU cacheable
    PerCpu,
}

#[derive(Debug, Clone, Copy)]
pub struct PageTableRef {
    pub id: usize,
    pub addr: Phys<u8>,
}

#[trait_ffi::def_extern_trait(not_def_impl)]
pub trait Mmu {
    /// Called once after memory management is ready.
    fn setup();
    fn page_size() -> usize;
    fn kimage_va_offset() -> usize;

    fn new_table(alloc: &mut dyn Access) -> Result<PageTableRef, PagingError>;
    fn release_table(table: PageTableRef, alloc: &mut dyn Access);
    fn get_kernel_table() -> PageTableRef;
    fn set_kernel_table(new_table: PageTableRef);
    fn table_map(
        table: PageTableRef,
        alloc: &mut dyn Access,
        config: &MapConfig,
    ) -> Result<(), PagingError>;
}

#[derive(Debug, Clone)]
pub struct MapConfig {
    pub name: &'static str,
    pub va_start: Virt<u8>,
    pub pa_start: Phys<u8>,
    pub size: usize,
    pub access: AccessSetting,
    pub cache: CacheSetting,
}
