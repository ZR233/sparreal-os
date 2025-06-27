#![no_std]
#![no_main]
extern crate alloc;
extern crate sparreal_rt;

use core::time::Duration;

use alloc::string::ToString;
use log::info;
use sparreal_kernel::{
    prelude::*,
    task::{self, TaskConfig},
    time::{self, spin_delay},
};

#[entry]
fn main() {
    info!("Hello, world!");

    time::after(Duration::from_secs(2), || {
        info!("Timer callback");
        // shutdown();
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
    }
}
