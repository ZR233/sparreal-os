use core::cell::UnsafeCell;

use alloc::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};
use log::{debug, warn};
pub use rdrive::Phandle;
use rdrive::{Device, DeviceId, IrqId, intc::*};
use spin::Mutex;

use crate::{
    globals::{self, cpu_global},
    platform::{self, cpu_hard_id},
    platform_if::PlatformImpl,
};

#[derive(Default)]
pub struct CpuIrqChips(BTreeMap<DeviceId, Chip>);

pub type IrqHandler = dyn Fn(IrqId) -> IrqHandleResult;

pub struct Chip {
    device: Device<HardwareCPU>,
    mutex: Mutex<()>,
    handlers: UnsafeCell<BTreeMap<IrqId, Box<IrqHandler>>>,
}

unsafe impl Send for Chip {}
unsafe impl Sync for Chip {}

pub fn enable_all() {
    PlatformImpl::irq_all_enable();
}

pub(crate) fn init_main_cpu() {
    for (phandle, intc) in rdrive::intc_all() {
        debug!("[{}]({:?}) open", intc.descriptor.name, phandle,);
        let chip = intc.upgrade().unwrap();
        let mut g = chip.spin_try_borrow_by(0.into());

        g.open().unwrap();
    }

    init_current_cpu();
}

pub(crate) fn init_current_cpu() {
    let globals = unsafe { globals::cpu_global_mut() };

    for (phandle, intc) in rdrive::intc_all() {
        let intc = intc.upgrade().unwrap();
        let id = intc.descriptor.device_id;
        let g = intc.spin_try_borrow_by(0.into());

        let device = g.current_cpu_setup();
        debug!(
            "[{}]({:?}) init cpu: {:?}",
            intc.descriptor.name,
            phandle,
            platform::cpu_hard_id(),
        );

        let device = Device::new(intc.descriptor.clone(), device);

        globals.irq_chips.0.insert(
            id,
            Chip {
                device,
                mutex: Mutex::new(()),
                handlers: UnsafeCell::new(Default::default()),
            },
        );
    }
}

pub enum IrqHandleResult {
    Handled,
    None,
}

fn chip_cpu(id: DeviceId) -> &'static Chip {
    globals::cpu_global()
        .irq_chips
        .0
        .get(&id)
        .unwrap_or_else(|| panic!("irq chip {:?} not found", id))
}

pub struct IrqRegister {
    pub param: IrqParam,
    pub handler: Box<IrqHandler>,
    pub priority: Option<usize>,
}

impl IrqRegister {
    pub fn register(self) {
        let irq = self.param.cfg.irq;
        let irq_parent = self.param.intc;

        let chip = chip_cpu(irq_parent);
        chip.register_handle(irq, self.handler);

        let mut c = rdrive::edit(|m| m.intc.get(irq_parent))
            .unwrap()
            .upgrade()
            .unwrap()
            .spin_try_borrow_by(0.into());

        if let Some(p) = self.priority {
            c.set_priority(irq, p);
        } else {
            c.set_priority(irq, 0);
        }

        c.set_target_cpu(irq, cpu_hard_id().into());
        c.set_trigger(irq, self.param.cfg.trigger);
        c.irq_enable(irq);
        debug!("Enable irq {irq:?} on chip {irq_parent:?}");
    }

    pub fn priority(mut self, priority: usize) -> Self {
        self.priority = Some(priority);
        self
    }
}

impl Chip {
    fn register_handle(&self, irq: IrqId, handle: Box<IrqHandler>) {
        let g = NoIrqGuard::new();
        let gm = self.mutex.lock();
        unsafe { &mut *self.handlers.get() }.insert(irq, handle);
        drop(gm);
        drop(g);
    }

    fn unregister_handle(&self, irq: IrqId) {
        let g = NoIrqGuard::new();
        let gm = self.mutex.lock();
        unsafe { &mut *self.handlers.get() }.remove(&irq);
        drop(gm);
        drop(g);
    }

    fn handle_irq(&self) -> Option<()> {
        let irq = self.device.get_and_acknowledge_interrupt()?;

        if let Some(handler) = unsafe { &mut *self.handlers.get() }.get(&irq) {
            let res = (handler)(irq);
            if let IrqHandleResult::None = res {
                return Some(());
            }
        } else {
            warn!("IRQ {irq:?} no handler");
        }
        self.device.end_interrupt(irq);
        Some(())
    }
}

pub struct NoIrqGuard {
    is_enabled: bool,
}

impl NoIrqGuard {
    pub fn new() -> Self {
        let is_enabled = PlatformImpl::irq_all_is_enabled();
        PlatformImpl::irq_all_disable();
        Self { is_enabled }
    }
}

impl Default for NoIrqGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for NoIrqGuard {
    fn drop(&mut self) {
        if self.is_enabled {
            enable_all();
        }
    }
}

pub fn handle_irq() -> usize {
    for chip in cpu_global().irq_chips.0.values() {
        chip.handle_irq();
    }

    let cu = crate::task::current();

    cu.sp
}

#[derive(Debug, Clone)]
pub struct IrqInfo {
    pub irq_parent: DeviceId,
    pub cfgs: Vec<IrqConfig>,
}

#[derive(Debug, Clone)]
pub struct IrqParam {
    pub intc: DeviceId,
    pub cfg: IrqConfig,
}

impl IrqParam {
    pub fn register_builder(
        &self,
        handler: impl Fn(IrqId) -> IrqHandleResult + 'static,
    ) -> IrqRegister {
        IrqRegister {
            param: self.clone(),
            handler: Box::new(handler),
            priority: None,
        }
    }
}

pub fn unregister_irq(irq: IrqId) {
    for chip in cpu_global().irq_chips.0.values() {
        chip.unregister_handle(irq);
    }
}
