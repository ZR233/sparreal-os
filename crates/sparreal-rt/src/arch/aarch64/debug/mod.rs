use core::{
    cell::UnsafeCell,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

use any_uart::block;
pub use any_uart::{FnPhysToVirt, Sender};

static UART: UartWapper = UartWapper(UnsafeCell::new(None));
static REGBASE: AtomicUsize = AtomicUsize::new(0);

struct UartWapper(UnsafeCell<Option<Sender>>);

unsafe impl Send for UartWapper {}
unsafe impl Sync for UartWapper {}

impl UartWapper {
    fn set(&self, uart: Sender) {
        unsafe {
            *self.0.get() = Some(uart);
        }
    }

    #[allow(clippy::mut_from_ref)]
    fn get(&self) -> &mut Sender {
        unsafe { &mut *self.0.get().as_mut().unwrap().as_mut().unwrap() }
    }
}

pub fn reg() -> usize {
    REGBASE.load(Ordering::SeqCst)
}

pub fn put(byte: u8) {
    let _ = block!(UART.get().write(byte));
}
pub fn setup_by_fdt(fdt: Option<NonNull<u8>>, f: FnPhysToVirt) -> Option<()> {
    let mut uart = any_uart::init(fdt?, f)?;
    uart.set_irq_enable(false);
    let tx = uart.tx.take().unwrap();
    let reg = REGBASE.load(Ordering::SeqCst);
    if reg == 0 {
        REGBASE.store(tx.mmio(), Ordering::SeqCst);
    }

    UART.set(tx);

    Some(())
}
