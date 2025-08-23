use core::fmt::Write;

use crate::platform;

pub struct DebugWriter;

impl Write for DebugWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_str(s);
        Ok(())
    }
}
pub fn write_str(s: &str) {
    s.bytes().for_each(|ch| {
        platform::debug_put(ch);
    });
}

pub fn print(args: core::fmt::Arguments) {
    let mut writer = DebugWriter;
    let _ = writer.write_fmt(args);
}
