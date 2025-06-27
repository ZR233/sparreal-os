use crate::{globals::global_val, irq, platform, time};
use log::debug;
pub use rdrive::*;

pub fn init() {
    let info = match &global_val().platform_info {
        crate::globals::PlatformInfoKind::DeviceTree(fdt) => Platform::Fdt {
            addr: fdt.get_addr(),
        },
    };

    rdrive::init(info).unwrap();

    rdrive::register_append(&platform::module_registers());

    debug!("add registers");

    rdrive::probe_pre_kernel().unwrap();

    irq::init_main_cpu();
    time::init_current_cpu();
}

pub fn probe() {
    rdrive::probe_all(true).unwrap();
}
