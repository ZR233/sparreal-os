use core::error::Error;

use alloc::{boxed::Box, format, vec::Vec};
use log::{debug, error};
use smccc::{Hvc, Smc, psci};
use sparreal_kernel::driver::{
    DriverGeneric, DriverResult, module_driver, power::*, probe::HardwareKind, register::*,
};

module_driver!(
    name: "ARM PSCI",
    kind: DriverKind::Power,
    probe_kinds: &[
        ProbeKind::Fdt {
            compatibles: &["arm,psci-1.0","arm,psci-0.2","arm,psci"],
            on_probe: probe
        }
    ]
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
    fn open(&mut self) -> DriverResult {
        Ok(())
    }

    fn close(&mut self) -> DriverResult {
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

fn probe(info: FdtInfo<'_>) -> Result<Vec<HardwareKind>, Box<dyn Error>> {
    let method = info
        .node
        .find_property("method")
        .ok_or("fdt no method property")?
        .str();
    let method = Method::try_from(method)?;

    let dev = HardwareKind::Power(Box::new(Psci { method }));
    debug!("PCSI [{method:?}]");
    Ok(alloc::vec![dev])
}
