use page_table_generic::PTEGeneric;
use sparreal_macros::api_trait;

#[api_trait]
pub trait Platform {
    fn wait_for_interrupt();
    fn debug_put(b: u8);
}

#[cfg(feature = "mmu")]
#[api_trait]
pub trait PageTable {
    fn set_kernel_table(addr: usize);
    fn get_kernel_table() -> usize;
    fn set_user_table(addr: usize);
    fn get_user_table() -> usize;
    fn flush_tlb(addr: *const u8);
    fn flush_tlb_all();
    fn page_size() -> usize;
    fn table_level() -> usize;
    fn new_pte(config: PTEGeneric) -> usize;
    fn read_pte(pte: usize) -> PTEGeneric;
}
