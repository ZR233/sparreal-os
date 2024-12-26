use crate::{MAIRKind, MAIRSetting};
use aarch64_cpu::registers::*;

pub struct MAIRDefault;

impl MAIRDefault {
    #[cfg(target_arch = "aarch64")]
    pub const fn mair_value() -> u64 {
        // Device-nGnRE memory
        use aarch64_cpu::registers::*;
        let attr0 = MAIR_EL1::Attr0_Device::nonGathering_nonReordering_EarlyWriteAck.value;
        // Normal memory
        let attr1 = MAIR_EL1::Attr1_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc.value
            | MAIR_EL1::Attr1_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc.value;
        let attr2 = MAIR_EL1::Attr2_Normal_Inner::NonCacheable.value
            | MAIR_EL1::Attr2_Normal_Outer::NonCacheable.value;
        attr0 | attr1 | attr2 // 0x44_ff_04
    }

    #[cfg(target_arch = "aarch64")]
    pub fn mair_el1_apply() {
        MAIR_EL1.set(Self::mair_value());
    }

    #[cfg(target_arch = "aarch64")]
    pub fn mair_el2_apply() {
        MAIR_EL2.set(Self::mair_value());
    }
}

impl MAIRSetting for MAIRDefault {
    fn get_idx(kind: MAIRKind) -> usize {
        match kind {
            MAIRKind::Device => 0,
            MAIRKind::Normal => 1,
            MAIRKind::NonCache => 2,
        }
    }

    fn from_idx(idx: usize) -> MAIRKind {
        match idx {
            0 => MAIRKind::Device,
            1 => MAIRKind::Normal,
            2 => MAIRKind::NonCache,
            _ => panic!("invalid mair index"),
        }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PTE(u64);

impl PTE {
    const PHYS_ADDR_MASK: u64 = 0x0000_ffff_ffff_f000; // bits 12..48

    pub const fn empty() -> Self {
        PTE(0)
    }

    pub const fn from_paddr(paddr: usize) -> Self {
        PTE(paddr as u64 & Self::PHYS_ADDR_MASK)
    }

    pub fn paddr(&self) -> usize {
        (self.0 & Self::PHYS_ADDR_MASK) as _
    }

    pub fn set_flags(&mut self, flags: PTEFlags) {
        self.0 |= flags.bits();
    }

    pub fn get_flags(&self) -> PTEFlags {
        PTEFlags::from_bits_truncate(self.0)
    }

    pub fn set_mair_idx(&mut self, idx: usize) {
        self.0 |= ((idx as u64) & 0b111) << 2;
    }

    pub fn get_mair_idx(&self) -> usize {
        ((self.0 >> 2) & 0b111) as usize
    }
}

impl From<u64> for PTE {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<PTE> for u64 {
    fn from(value: PTE) -> Self {
        value.0
    }
}

bitflags::bitflags! {
    /// Memory attribute fields in the VMSAv8-64 translation table format descriptors.
    #[derive(Debug, Clone, Copy)]
    pub struct PTEFlags: u64 {
        // Attribute fields in stage 1 VMSAv8-64 Block and Page descriptors:

        /// Whether the descriptor is valid.
        const VALID =       1 << 0;
        /// The descriptor gives the address of the next level of translation table or 4KB page.
        /// (not a 2M, 1G block)
        const NON_BLOCK =   1 << 1;

        /// Non-secure bit. For memory accesses from Secure state, specifies whether the output
        /// address is in Secure or Non-secure memory.
        const NS =          1 << 5;
        /// Access permission: accessable at EL0.
        const AP_EL0 =      1 << 6;
        /// Access permission: read-only.
        const AP_RO =       1 << 7;
        /// Shareability: Inner Shareable (otherwise Outer Shareable).
        const INNER =       1 << 8;
        /// Shareability: Inner or Outer Shareable (otherwise Non-shareable).
        const SHAREABLE =   1 << 9;
        /// The Access flag.
        const AF =          1 << 10;
        /// The not global bit.
        const NG =          1 << 11;
        /// Indicates that 16 adjacent translation table entries point to contiguous memory regions.
        const CONTIGUOUS =  1 <<  52;
        /// The Privileged execute-never field.
        const PXN =         1 <<  53;
        /// The Execute-never or Unprivileged execute-never field.
        const UXN =         1 <<  54;

        // Next-level attributes in stage 1 VMSAv8-64 Table descriptors:

        /// PXN limit for subsequent levels of lookup.
        const PXN_TABLE =           1 << 59;
        /// XN limit for subsequent levels of lookup.
        const XN_TABLE =            1 << 60;
        /// Access permissions limit for subsequent levels of lookup: access at EL0 not permitted.
        const AP_NO_EL0_TABLE =     1 << 61;
        /// Access permissions limit for subsequent levels of lookup: write access not permitted.
        const AP_NO_WRITE_TABLE =   1 << 62;
        /// For memory accesses from Secure state, specifies the Security state for subsequent
        /// levels of lookup.
        const NS_TABLE =            1 << 63;
    }
}
