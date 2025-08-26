use arrayvec::ArrayVec;
// use page_table_generic::{AccessSetting, CacheSetting};

use crate::platform;

use super::{mmu::BootRegion, once::OnceStatic};

const MAX_BOOT_RSV_SIZE: usize = 12;
pub type BootRsvRegionVec = ArrayVec<BootRegion, MAX_BOOT_RSV_SIZE>;

static BOOT_RSV_REGION: OnceStatic<BootRsvRegionVec> = OnceStatic::new(ArrayVec::new_const());

pub(crate) unsafe fn init_boot_rsv_region() {
    unsafe {
        let mut rsv_regions = BootRsvRegionVec::new_const();

        let mut index = 0;
        while let Some(region) = platform::boot_region_by_index(index) {
            rsv_regions.push(region);
            index += 1;
        }

        BOOT_RSV_REGION.set(rsv_regions);
    }
}

pub fn boot_regions() -> &'static BootRsvRegionVec {
    &BOOT_RSV_REGION
}
