use crate::io;

pub fn run() {
    io::print::stdout_use_debug();
    println!("Kernel starting...");
    crate::mem::init();
}
