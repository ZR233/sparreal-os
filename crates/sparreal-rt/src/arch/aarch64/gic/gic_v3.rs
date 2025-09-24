use alloc::format;
use arm_gic_driver::v3::*;
use rdif_intc::*;
use sparreal_kernel::{
    driver::{
        self, DeviceId, IrqId, PlatformDevice, module_driver, probe::OnProbeError,
        register::FdtInfo,
    },
    irq::IrqParam,
    mem::iomap,
};

use crate::arch::gic::{VERSION, id_convert, trigger_convert};

module_driver!(
    name: "GICv3",
    level: ProbeLevel::PreKernel,
    priority: ProbePriority::INTC,
    probe_kinds: &[
        ProbeKind::Fdt {
            compatibles: &["arm,gic-v3"],
            on_probe: probe_gic
        }
    ],
);

fn probe_gic(info: FdtInfo<'_>, dev: PlatformDevice) -> Result<(), OnProbeError> {
    let mut reg = info.node.reg().ok_or(OnProbeError::other(format!(
        "[{}] has no reg",
        info.node.name()
    )))?;

    let gicd_reg = reg.next().unwrap();
    let gicr_reg = reg.next().unwrap();

    let gicd = iomap(
        (gicd_reg.address as usize).into(),
        gicd_reg.size.unwrap_or(0x1000),
    );
    let gicr = iomap(
        (gicr_reg.address as usize).into(),
        gicr_reg.size.unwrap_or(0x1000),
    );

    let gic = unsafe { Gic::new(gicd.into(), gicr.into()) };

    dev.register(Intc::new(gic));

    unsafe { VERSION = 3 };

    Ok(())
}

fn with_gic<R>(id: DeviceId, f: impl FnOnce(&mut Gic) -> R) -> R {
    let mut gic = driver::get::<Intc>(id).unwrap().lock().unwrap();
    f(gic.typed_mut().unwrap())
}

pub fn irq_enable(config: IrqParam) {
    with_gic(config.intc, |gic| {
        let intid = id_convert(config.cfg.irq);
        gic.set_irq_enable(intid, true);
        gic.set_priority(intid, 0);
        if !intid.is_private() {
            gic.set_cfg(intid, trigger_convert(config.cfg.trigger));
            gic.set_target_cpu(intid, Some(Affinity::current()));
        }
    });
}

pub fn irq_disable(id: DeviceId, irq: IrqId) {
    with_gic(id, |gic| {
        let intid = id_convert(irq);
        gic.set_irq_enable(intid, false);
    });
}

pub fn init_current_cpu(id: DeviceId) {
    let mut cpu = with_gic(id, |gic| gic.cpu_interface());
    cpu.init_current_cpu().unwrap();
}

pub fn ack() -> IrqId {
    (ack1().to_u32() as usize).into()
}

pub fn eoi(irq: IrqId) {
    let intid = id_convert(irq);

    eoi1(intid);

    if eoi_mode() {
        dir(intid);
    }
}
