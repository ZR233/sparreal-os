#[cfg(target_os = "none")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("kernel panic: {info:?}");
    crate::platform::shutdown()
}
