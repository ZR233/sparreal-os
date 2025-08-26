use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use arrayvec::ArrayVec;
use core::{ops::Range, ptr::NonNull};
use fdt_parser::{Node, Pci};
use log::warn;
use rdrive::{Phandle, driver::Intc};

use super::{CPUInfo, SerialPort};
use crate::mem::PhysAddr;
use crate::{irq::IrqInfo, mem::mmu::LINER_OFFSET};

#[derive(Clone)]
pub struct Fdt(PhysAddr);

impl Fdt {
    pub fn new(addr: PhysAddr) -> Self {
        Self(addr)
    }

    pub fn model_name(&self) -> Option<String> {
        let fdt = self.get();
        let node = fdt.all_nodes().next()?;

        let model = node.find_property("model")?;

        Some(model.str().to_string())
    }

    pub fn cpus(&self) -> Vec<CPUInfo> {
        let fdt = self.get();

        fdt.find_nodes("/cpus/cpu")
            .map(|cpu| {
                let reg = cpu.reg().unwrap().next().unwrap();
                CPUInfo {
                    cpu_id: super::CPUHardId(reg.address as usize),
                }
            })
            .collect()
    }

    pub fn get(&self) -> fdt_parser::Fdt<'static> {
        fdt_parser::Fdt::from_ptr(self.get_addr()).unwrap()
    }

    pub fn get_addr(&self) -> NonNull<u8> {
        NonNull::new((self.0 + LINER_OFFSET).raw() as _).unwrap()
    }

    pub fn memorys(&self) -> ArrayVec<Range<PhysAddr>, 12> {
        let mut out = ArrayVec::new();

        let fdt = self.get();

        for node in fdt.memory() {
            for region in node.regions() {
                let addr = (region.address as usize).into();
                out.push(addr..addr + region.size);
            }
        }
        out
    }

    pub fn take_memory(&self) -> Range<PhysAddr> {
        let region = self
            .get()
            .memory()
            .next()
            .unwrap()
            .regions()
            .next()
            .unwrap();
        let addr = (region.address as usize).into();
        addr..addr + region.size
    }

    pub fn debugcon(&self) -> Option<SerialPort> {
        let fdt = self.get();
        let stdout = fdt.chosen()?.stdout()?;
        let compatible = stdout.node.compatibles();
        let reg = stdout.node.reg()?.next()?;
        Some(SerialPort::new(
            (reg.address as usize).into(),
            reg.size,
            compatible,
        ))
    }
}

pub trait GetIrqConfig {
    fn irq_info(&self) -> Option<IrqInfo>;
}

impl GetIrqConfig for Node<'_> {
    fn irq_info(&self) -> Option<IrqInfo> {
        let irq_chip_node = self.interrupt_parent()?;
        let phandle = irq_chip_node.node.phandle()?;

        let interrupts = self.interrupts()?.map(|o| o.collect()).collect::<Vec<_>>();

        parse_irq_config(phandle, &interrupts)
    }
}

fn parse_irq_config(parent: Phandle, interrupts: &[Vec<u32>]) -> Option<IrqInfo> {
    let irq_parent = rdrive::fdt_phandle_to_device_id(parent)?;
    let parent = rdrive::get::<Intc>(irq_parent).expect("Intc not found");
    let parse_fun = { parent.lock().unwrap().parse_dtb_fn()? };

    let mut cfgs = Vec::new();
    for raw in interrupts {
        if let Ok(v) = parse_fun(raw) {
            cfgs.push(v);
        } else {
            warn!("Failed to parse IRQ config: {raw:?}");
            continue;
        }
    }

    Some(IrqInfo { irq_parent, cfgs })
}

pub trait GetPciIrqConfig {
    fn child_irq_info(&self, bus: u8, device: u8, function: u8, irq_pin: u8) -> Option<IrqInfo>;
}
impl GetPciIrqConfig for Pci<'_> {
    fn child_irq_info(&self, bus: u8, device: u8, func: u8, irq_pin: u8) -> Option<IrqInfo> {
        let irq = self
            .child_interrupts(bus, device, func, irq_pin as _)
            .ok()?;

        let raw = irq.irqs.collect::<Vec<_>>();

        parse_irq_config(irq.parent, &[raw])
    }
}
