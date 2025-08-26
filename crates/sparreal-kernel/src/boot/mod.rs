#![allow(unused)]

use ansi_rgb::{Foreground, orange};
use log::{LevelFilter, debug};

use crate::{
    driver,
    globals::{self, global_val},
    io::{self, print::*},
    irq,
    logger::KLogger,
    mem::{self, VirtAddr, region, stack_top},
    platform::{self, app_main, module_registers, platform_name, shutdown},
    println, task, time,
};

pub mod debug;

// #[cfg(feature = "mmu")]
// mod mmu;

// #[cfg(feature = "mmu")]
// pub use mmu::start;

// pub extern "C" fn __start() -> ! {
//     println!("Kernel starting...");
//     set_mmu_enabled();
//     irq::disable_all();

//     io::print::stdout_use_debug();

//     let _ = log::set_logger(&KLogger);
//     log::set_max_level(LevelFilter::Trace);

//     mem::init_heap();

//     unsafe { globals::setup_percpu() };

//     print_start_msg();

//     mem::init_page_and_memory();

//     driver::init();
//     debug!("Driver initialized");
//     task::init();

//     irq::enable_all();

//     driver::probe();

//     app_main();

//     shutdown()
// }

