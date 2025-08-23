pub use crate::mem::{Phys, Virt};

bitflags::bitflags! {
    /// Generic page table entry flags that indicate the corresponding mapped
    /// memory region permissions and attributes.
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct AccessSetting: u8 {
        const Read = 1 << 0;
        const Write = 1 << 1;
        const Execute = 1 << 2;
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
    PerCpuCacheable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingError {
    InvalidAddress,
    NoMemory,
    AlreadyMapped,
}

#[trait_ffi::def_extern_trait(not_def_impl)]
pub trait Mmu {
    /// Called once after memory management is ready.
    fn setup();
    fn page_size() -> usize;

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
