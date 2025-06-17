use core::panic::PanicInfo;

use log::error;

use crate::platform::shutdown;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("kernel panic: {info:?}");
    shutdown()
}
