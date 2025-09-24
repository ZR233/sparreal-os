use core::error::Error;

use alloc::{boxed::Box, format};
use log::{debug, error};
use rdif_power::*;
use smccc::{Hvc, Smc, psci};
use sparreal_kernel::driver::{
    DriverGeneric, PlatformDevice, module_driver, probe::OnProbeError, register::*,
};

module_driver!(
    name: "ARM PSCI",
    level: ProbeLevel::PreKernel,
    priority: ProbePriority::DEFAULT,
    probe_kinds: &[
        ProbeKind::Fdt {
            compatibles: &["arm,psci-1.0","arm,psci-0.2","arm,psci"],
            on_probe: probe
        }
    ],
);

#[derive(Debug, Clone, Copy)]
enum Method {
    Smc,
    Hvc,
}

impl TryFrom<&str> for Method {
    type Error = Box<dyn Error>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "smc" => Ok(Method::Smc),
            "hvc" => Ok(Method::Hvc),
            _ => Err(format!("method [{value}] not support").into()),
        }
    }
}

struct Psci {
    method: Method,
}

impl DriverGeneric for Psci {
    fn open(&mut self) -> Result<(), KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        Ok(())
    }
}

impl Interface for Psci {
    fn shutdown(&mut self) {
        if let Err(e) = match self.method {
            Method::Smc => psci::system_off::<Smc>(),
            Method::Hvc => psci::system_off::<Hvc>(),
        } {
            error!("shutdown failed: {e}");
        }
    }
}

fn probe(info: FdtInfo<'_>, plat_dev: PlatformDevice) -> Result<(), OnProbeError> {
    let method = info
        .node
        .find_property("method")
        .ok_or(OnProbeError::other("fdt no method property"))?
        .str();
    let method = Method::try_from(method)?;

    plat_dev.register(Power::new(Psci { method }));

    debug!("PCSI [{method:?}]");
    Ok(())
}
