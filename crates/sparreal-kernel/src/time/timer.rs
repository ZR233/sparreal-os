use core::{
    sync::atomic::{Ordering, fence},
    time::Duration,
};

use super::queue;
use alloc::boxed::Box;
use rdrive::IrqConfig;

const NANO_PER_SEC: u128 = 1_000_000_000;

pub struct Timer {
    timer: Box<dyn rdif_systick::local::Interface>,
    q: queue::Queue,
}

unsafe impl Sync for Timer {}
unsafe impl Send for Timer {}

impl Timer {
    pub fn new(timer: Box<dyn rdif_systick::local::Interface>) -> Self {
        Self {
            timer,
            q: queue::Queue::new(),
        }
    }

    pub fn since_boot(&self) -> Duration {
        self.tick_to_duration(self.timer.current_ticks() as _)
    }

    pub fn after(&mut self, duration: Duration, callback: impl Fn() + 'static) {
        let ticks = self.duration_to_tick(duration);

        let event = queue::Event {
            interval: None,
            at_tick: self.timer.current_ticks() as u64 + ticks,
            callback: Box::new(callback),
            called: false,
        };

        self.add_event(event);
    }

    pub fn every(&mut self, duration: Duration, callback: impl Fn() + 'static) {
        let ticks = self.duration_to_tick(duration);

        let event = queue::Event {
            interval: Some(ticks),
            at_tick: self.timer.current_ticks() as u64 + ticks,
            callback: Box::new(callback),
            called: false,
        };

        self.add_event(event);
    }

    fn add_event(&mut self, event: queue::Event) {
        fence(Ordering::SeqCst);

        let next_tick = self.q.add_and_next_tick(event);
        let v = next_tick as usize - self.timer.current_ticks();
        self.timer.set_timeval(v);

        fence(Ordering::SeqCst);
    }

    pub fn handle_irq(&mut self) {
        while let Some(event) = self.q.pop(self.timer.current_ticks() as u64) {
            (event.callback)();
        }

        match self.q.next_tick() {
            Some(next_tick) => {
                self.timer.set_timeval(next_tick as _);
                self.set_irq_enable(true);
            }
            None => {
                self.set_irq_enable(false);
            }
        }
    }

    pub fn set_irq_enable(&mut self, enable: bool) {
        self.timer.set_irq_enable(enable);
    }
    pub fn get_irq_status(&self) -> bool {
        self.timer.get_irq_status()
    }

    fn tick_to_duration(&self, tick: u64) -> Duration {
        Duration::from_nanos((tick as u128 * NANO_PER_SEC / self.timer.tick_hz() as u128) as _)
    }

    fn duration_to_tick(&self, duration: Duration) -> u64 {
        (duration.as_nanos() * self.timer.tick_hz() as u128 / NANO_PER_SEC) as _
    }

    pub fn irq(&self) -> IrqConfig {
        self.timer.irq()
    }
}
