use core::cell::UnsafeCell;

use alloc::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};
use log::{debug, warn};
pub use rdrive::Phandle;
use rdrive::{DeviceId, driver::intc::*};
use spin::Mutex;

use crate::{
    globals::{self, cpu_global},
    platform::{self, cpu_hard_id},
    platform_if::PlatformImpl,
};

#[derive(Default)]
pub struct CpuIrqChips(BTreeMap<DeviceId, Chip>);

pub struct Chip {
    mutex: Mutex<()>,
    device: Box<dyn local::Interface>,
    handlers: UnsafeCell<BTreeMap<IrqId, Box<IrqHandler>>>,
}

unsafe impl Send for Chip {}
unsafe impl Sync for Chip {}

pub type IrqHandler = dyn Fn(IrqId) -> IrqHandleResult;

pub fn enable_all() {
    PlatformImpl::irq_all_enable();
}

pub fn disable_all() {
    PlatformImpl::irq_all_disable();
}

pub(crate) fn init_main_cpu() {
    for chip in rdrive::get_list::<Intc>() {
        debug!(
            "[{}]({:?}) open",
            chip.descriptor().name,
            chip.descriptor().device_id()
        );
        chip.lock().unwrap().open().unwrap();
    }

    init_current_cpu();
}

pub(crate) fn init_current_cpu() {
    let globals = unsafe { globals::cpu_global_mut() };

    for intc in rdrive::get_list::<Intc>() {
        let id = intc.descriptor().device_id();
        let g = intc.lock().unwrap();

        let Some(mut cpu_if) = g.cpu_local() else {
            continue;
        };

        cpu_if.open().unwrap();
        cpu_if.set_eoi_mode(false);

        debug!(
            "[{}]({:?}) init cpu: {:?}",
            intc.descriptor().name,
            id,
            platform::cpu_hard_id(),
        );

        globals.irq_chips.0.insert(
            id,
            Chip {
                mutex: Mutex::new(()),
                device: cpu_if,
                handlers: UnsafeCell::new(BTreeMap::new()),
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

        if self.param.cfg.is_private
            && let local::Capability::ConfigLocalIrq(c) = chip.device.capability()
        {
            if let Some(p) = self.priority {
                c.set_priority(irq, p).unwrap();
            } else {
                c.set_priority(irq, 0).unwrap();
            }
            c.set_trigger(irq, self.param.cfg.trigger).unwrap();
            c.irq_enable(irq).unwrap();
        } else {
            let mut c = rdrive::get::<Intc>(irq_parent).unwrap().lock().unwrap();
            if let Some(p) = self.priority {
                c.set_priority(irq, p).unwrap();
            } else {
                c.set_priority(irq, 0).unwrap();
            }
            if !self.param.cfg.is_private {
                c.set_target_cpu(irq, cpu_hard_id().into()).unwrap();
            }
            c.set_trigger(irq, self.param.cfg.trigger).unwrap();
            c.irq_enable(irq).unwrap();
        }

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
        let irq = self.device.ack()?;

        if let Some(handler) = unsafe { &mut *self.handlers.get() }.get(&irq) {
            let res = (handler)(irq);
            if let IrqHandleResult::None = res {
                return Some(());
            }
        } else {
            warn!("IRQ {irq:?} no handler");
        }
        self.device.eoi(irq);
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
