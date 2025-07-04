use aarch64_cpu::registers::*;
use alloc::boxed::Box;
use log::debug;
use sparreal_kernel::driver::{
    DriverGeneric, PlatformDevice,
    driver::{intc::IrqConfig, systick::*},
    module_driver,
    probe::OnProbeError,
    register::*,
};

module_driver!(
    name: "ARMv8 Timer",
    level: ProbeLevel::PreKernel,
    priority: ProbePriority::DEFAULT,
    probe_kinds: &[
        ProbeKind::Fdt {
            compatibles: &["arm,armv8-timer"],
            on_probe: probe_timer
        }
    ],
);

#[derive(Clone)]
struct ArmV8Timer {
    irq: IrqConfig,
}

impl Interface for ArmV8Timer {
    fn cpu_local(&mut self) -> local::Boxed {
        CNTP_CTL_EL0.modify(CNTP_CTL_EL0::ENABLE::SET + CNTP_CTL_EL0::IMASK::SET);
        debug!("ARMv8 Timer: Enabled");
        Box::new(self.clone())
    }
}

impl local::Interface for ArmV8Timer {
    fn set_timeval(&self, ticks: usize) {
        CNTP_TVAL_EL0.set(ticks as _);
    }

    fn current_ticks(&self) -> usize {
        CNTPCT_EL0.get() as _
    }

    fn tick_hz(&self) -> usize {
        CNTFRQ_EL0.get() as _
    }

    fn set_irq_enable(&self, enable: bool) {
        CNTP_CTL_EL0.modify(if enable {
            CNTP_CTL_EL0::IMASK::CLEAR
        } else {
            CNTP_CTL_EL0::IMASK::SET
        });
    }

    fn get_irq_status(&self) -> bool {
        CNTP_CTL_EL0.is_set(CNTP_CTL_EL0::ISTATUS)
    }

    fn irq(&self) -> IrqConfig {
        self.irq.clone()
    }
}

impl DriverGeneric for ArmV8Timer {
    fn open(&mut self) -> Result<(), KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        Ok(())
    }
}

fn probe_timer(_info: FdtInfo<'_>, plat_dev: PlatformDevice) -> Result<(), OnProbeError> {
    let timer = ArmV8Timer {
        irq: plat_dev.descriptor.irqs[1].clone(),
    };

    plat_dev.register_systick(timer);

    Ok(())
}
