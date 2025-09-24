use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::{
    globals::{cpu_global, cpu_global_meybeuninit, cpu_global_mut},
    irq::{IrqHandleResult, IrqParam},
};

use rdrive::IrqId;
use spin::{Mutex, MutexGuard};
pub use timer::Timer;

mod queue;
mod timer;

#[derive(Default)]
pub(crate) struct TimerData {
    mutex: Mutex<()>,
    timer: UnsafeCell<Option<Timer>>,
}

unsafe impl Sync for TimerData {}
unsafe impl Send for TimerData {}

pub(crate) struct Guard<'a> {
    _guard: MutexGuard<'a, ()>,
    timer: *mut Option<Timer>,
    irq_state: bool,
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        if let Some(t) = unsafe { &mut *self.timer } {
            t.set_irq_enable(self.irq_state);
        }
    }
}

impl Deref for Guard<'_> {
    type Target = Option<Timer>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.timer }
    }
}

impl DerefMut for Guard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.timer }
    }
}

impl TimerData {
    pub fn lock<'a>(&'a self) -> Guard<'a> {
        let timer = unsafe { &mut *self.timer.get() };
        let mut irq_state = false;
        if let Some(t) = timer {
            irq_state = t.get_irq_status();
            t.set_irq_enable(false);
        }
        let g = self.mutex.lock();
        Guard {
            _guard: g,
            timer: timer as _,
            irq_state,
        }
    }

    fn force_use(&self) -> *mut Option<Timer> {
        self.timer.get()
    }
}

pub fn since_boot() -> Duration {
    _since_boot().unwrap_or_default()
}

fn _since_boot() -> Option<Duration> {
    let timer = unsafe { &*cpu_global_meybeuninit()?.timer.force_use() }.as_ref()?;
    Some(timer.since_boot())
}

pub(crate) fn init_current_cpu() -> Option<()> {
    let intc;
    let cfg;
    {
        let systick = rdrive::get_one::<rdif_systick::Systick>().unwrap();
        intc = systick.descriptor().irq_parent?;
        let cpu_if = { systick.lock().unwrap().cpu_local() };
        cfg = cpu_if.irq();
        cpu_if.set_irq_enable(false);
        let t = Timer::new(cpu_if);

        unsafe { *cpu_global_mut().timer.lock() = Some(t) };
    };
    IrqParam { intc, cfg }
        .register_builder(irq_handle)
        .register();

    Some(())
}

fn irq_handle(_irq: IrqId) -> IrqHandleResult {
    let timer = unsafe { &mut *timer_data().force_use() };
    if let Some(t) = timer.as_mut() {
        t.handle_irq();
    } else {
        // Timer not initialized, do nothing
        return IrqHandleResult::None;
    }
    IrqHandleResult::Handled
}

fn timer_data() -> &'static TimerData {
    &cpu_global().timer
}

pub fn after(duration: Duration, call: impl Fn() + 'static) {
    let mut g = timer_data().lock();
    if let Some(t) = g.as_mut() {
        t.after(duration, call);
    }
}

pub fn spin_delay(duration: Duration) {
    let now = since_boot();
    let at = now + duration;

    loop {
        if since_boot() >= at {
            break;
        }
    }
}

pub fn sleep(duration: Duration) {
    let pid = crate::task::current().pid;
    after(duration, move || {
        crate::task::wake_up_in_irq(pid);
    });
    crate::task::suspend();
}
