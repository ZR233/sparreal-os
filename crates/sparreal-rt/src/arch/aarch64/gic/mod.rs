use arm_gic_driver::IntId;
use sparreal_kernel::{
    driver::{DeviceId, IrqId, driver::intc::Trigger},
    irq::IrqParam,
};

mod gic_v2;
mod gic_v3;

static mut VERSION: usize = 0;

fn version() -> usize {
    unsafe { VERSION }
}

pub fn irq_enable(config: IrqParam) {
    match version() {
        2 => gic_v2::irq_enable(config),
        3 => gic_v3::irq_enable(config),
        _ => panic!("Unsupported GIC version"),
    }
}

pub fn irq_disable(id: DeviceId, irq: IrqId) {
    match version() {
        2 => gic_v2::irq_disable(id, irq),
        3 => gic_v3::irq_disable(id, irq),
        _ => panic!("Unsupported GIC version"),
    }
}

pub fn init_current_cpu(id: DeviceId) {
    match version() {
        2 => gic_v2::init_current_cpu(id),
        3 => gic_v3::init_current_cpu(id),
        _ => panic!("Unsupported GIC version"),
    }
}

pub fn ack() -> IrqId {
    match version() {
        2 => gic_v2::ack(),
        3 => gic_v3::ack(),
        _ => panic!("Unsupported GIC version"),
    }
}

pub fn eoi(irq: IrqId) {
    match version() {
        2 => gic_v2::eoi(irq),
        3 => gic_v3::eoi(irq),
        _ => panic!("Unsupported GIC version"),
    }
}

fn id_convert(irq: IrqId) -> IntId {
    let intid: usize = irq.into();
    unsafe { IntId::raw(intid as _) }
}

fn trigger_convert(trigger: Trigger) -> arm_gic_driver::v2::Trigger {
    match trigger {
        Trigger::EdgeBoth => arm_gic_driver::v2::Trigger::Edge,
        Trigger::EdgeRising => arm_gic_driver::v2::Trigger::Edge,
        Trigger::EdgeFailling => arm_gic_driver::v2::Trigger::Edge,
        Trigger::LevelHigh => arm_gic_driver::v2::Trigger::Level,
        Trigger::LevelLow => arm_gic_driver::v2::Trigger::Level,
    }
}
