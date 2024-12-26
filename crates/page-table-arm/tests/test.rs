#[cfg(test)]
mod test {
    extern crate std;
    use page_table_arm::*;

    #[test]
    fn test_l1() {
        let mut pte = PTE::from_paddr(0x1234_5678_9000);
        pte.set_flags(PTEFlags::VALID | PTEFlags::AP_EL0 | PTEFlags::AP_RO);
        pte.set_mair_idx(5);

        assert_eq!(
            pte.get_flags().bits(),
            (PTEFlags::VALID | PTEFlags::AP_EL0 | PTEFlags::AP_RO).bits()
        );
        assert_eq!(pte.paddr(), 0x1234_5678_9000);
        assert_eq!(pte.get_mair_idx(), 5);
    }
}
