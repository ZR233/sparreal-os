use core::{future::Future, time::Duration};

use crate::platform::PlatformImpl;

pub fn since_boot() -> Duration {
    let current_tick = unsafe { PlatformImpl::current_ticks() };
    let freq = unsafe { PlatformImpl::tick_hz() };
    Duration::from_nanos(current_tick * 1_000_000_000 / freq)
}
