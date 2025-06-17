use core::fmt::{self, Write};

use alloc::boxed::Box;
use spin::Mutex;

use crate::{boot::debug, platform_if::PlatformImpl};

static STDOUT: Mutex<Option<Box<dyn fmt::Write + Send>>> = Mutex::new(None);

pub fn stdout_use_debug() {
    *STDOUT.lock() = Some(Box::new(debug::DebugWriter {}));
}

pub fn print(args: fmt::Arguments<'_>) {
    let mut g = STDOUT.lock();

    if let Some(ref mut writer) = *g {
        let _ = writer.write_fmt(args);
    }
}