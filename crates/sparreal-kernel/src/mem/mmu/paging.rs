use page_table_generic::{PTEArch, PTEGeneric};

use crate::platform::PageTableImpl;

pub type PageTableRef<'a> = page_table_generic::PageTableRef<'a, PTEImpl>;

pub fn get_kernal_table<'a>() -> PageTableRef<'a> {
    let addr = PageTableImpl::get_kernel_table();
    let level = PageTableImpl::table_level();
    PageTableRef::from_addr(addr, level)
}

#[derive(Clone, Copy)]
pub struct PTEImpl;

impl PTEArch for PTEImpl {
    fn page_size() -> usize {
        PageTableImpl::page_size()
    }

    fn level() -> usize {
        PageTableImpl::table_level()
    }

    fn new_pte(config: PTEGeneric) -> usize {
        PageTableImpl::new_pte(config)
    }

    fn read_pte(pte: usize) -> PTEGeneric {
        PageTableImpl::read_pte(pte)
    }
}
