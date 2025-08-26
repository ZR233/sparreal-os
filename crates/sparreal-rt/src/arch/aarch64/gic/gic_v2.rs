use core::{cell::UnsafeCell, ops::Deref};

use alloc::format;
use arm_gic_driver::v2::*;
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
    name: "GICv2",
    level: ProbeLevel::PreKernel,
    priority: ProbePriority::INTC,
    probe_kinds: &[
        ProbeKind::Fdt {
            compatibles: &["arm,cortex-a15-gic", "arm,gic-400"],
            on_probe: probe_gic
        },
    ] ,
);

fn probe_gic(info: FdtInfo<'_>, dev: PlatformDevice) -> Result<(), OnProbeError> {
    let mut reg = info.node.reg().ok_or(OnProbeError::other(format!(
        "[{}] has no reg",
        info.node.name()
    )))?;

    let gicd_reg = reg.next().unwrap();
    let gicc_reg = reg.next().unwrap();

    let gicd = iomap(
        (gicd_reg.address as usize).into(),
        gicd_reg.size.unwrap_or(0x1000),
    );
    let gicc = iomap(
        (gicc_reg.address as usize).into(),
        gicc_reg.size.unwrap_or(0x1000),
    );

    let mut hyper = None;

    if let Some(gich) = reg.next()
        && let Some(gicv) = reg.next()
    {
        let gich = iomap((gich.address as usize).into(), gich.size.unwrap_or(0x1000));
        let gicv = iomap((gicv.address as usize).into(), gicv.size.unwrap_or(0x1000));
        hyper = Some(HyperAddress::new(gich.into(), gicv.into()));
    }

    let gic = unsafe { Gic::new(gicd.into(), gicc.into(), hyper) };
    let cpu = gic.cpu_interface();
    unsafe {
        (&mut *TRAP.0.get()).replace(cpu.trap_operations());
    };

    dev.register_intc(gic);
    unsafe { VERSION = 2 };
    Ok(())
}

fn with_gic<R>(id: DeviceId, f: impl FnOnce(&mut Gic) -> R) -> R {
    let mut gic = driver::get::<driver::driver::Intc>(id)
        .unwrap()
        .lock()
        .unwrap();
    f(gic.typed_mut().unwrap())
}

pub fn irq_enable(config: IrqParam) {
    with_gic(config.intc, |gic| {
        let intid = id_convert(config.cfg.irq);
        gic.set_irq_enable(intid, true);
        gic.set_priority(intid, 0);
        if !intid.is_private() {
            gic.set_cfg(intid, trigger_convert(config.cfg.trigger));
            gic.set_target_cpu(intid, TargetList::new([0].into_iter()));
        }
    });
}

pub fn irq_disable(id: DeviceId, irq: IrqId) {
    with_gic(id, |gic| {
        let intid = id_convert(irq);
        gic.set_irq_enable(intid, false);
    });
}

static TRAP: TrapOpWarp = TrapOpWarp(UnsafeCell::new(None));

struct TrapOpWarp(UnsafeCell<Option<TrapOp>>);

unsafe impl Send for TrapOpWarp {}
unsafe impl Sync for TrapOpWarp {}

impl Deref for TrapOpWarp {
    type Target = TrapOp;

    fn deref(&self) -> &Self::Target {
        let r = unsafe { &*self.0.get() };
        r.as_ref().unwrap()
    }
}

pub fn init_current_cpu(id: DeviceId) {
    let mut cpu = with_gic(id, |gic| gic.cpu_interface());
    cpu.init_current_cpu();
}

pub fn ack() -> IrqId {
    (match TRAP.ack() {
        Ack::SGI { intid, cpu_id: _ } => intid,
        Ack::Other(intid) => intid,
    }
    .to_u32() as usize)
        .into()
}

pub fn eoi(irq: IrqId) {
    let intid = id_convert(irq);
    TRAP.eoi(Ack::Other(intid));
}
