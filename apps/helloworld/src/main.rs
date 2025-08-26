#![no_std]
#![no_main]
extern crate alloc;
extern crate sparreal_rt;

use core::{sync::atomic::AtomicBool, time::Duration};

use alloc::{string::ToString, sync::Arc};
use log::info;
use sparreal_kernel::{
    prelude::*,
    task::{self, TaskConfig},
    time::{self, spin_delay},
};

#[entry]
fn main() {
    info!("Hello, world!");
    let irq_done = Arc::new(AtomicBool::new(false));
    time::after(Duration::from_secs(2), {
        let irq_done = irq_done.clone();
        move || {
            // info!("Timer callback");
            // shutdown();
            irq_done.store(true, core::sync::atomic::Ordering::SeqCst);
        }
    });

    task::spawn_with_config(
        || {
            info!("task2");

            // loop {
            //     spin_loop();
            // }
        },
        TaskConfig {
            name: "task2".to_string(),
            priority: 0,
            stack_size: 0x1000 * 4,
        },
    )
    .unwrap();

    loop {
        spin_delay(Duration::from_secs(1));
        info!("123");
        if irq_done.load(core::sync::atomic::Ordering::SeqCst) {
            info!("irq done");
        }
    }
}
