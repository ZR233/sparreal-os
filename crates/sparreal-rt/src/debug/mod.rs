use core::cell::UnsafeCell;

use aux_mini::AuxMini;
use fdt_parser::Fdt;
use pl011::Pl011;

mod aux_mini;
mod pl011;

static mut REG_BASE: usize = 0;
static UART: UartWapper = UartWapper(UnsafeCell::new(Uart::None));

struct UartWapper(UnsafeCell<Uart>);

unsafe impl Send for UartWapper {}
unsafe impl Sync for UartWapper {}

impl UartWapper {
    fn set(&self, uart: Uart) {
        unsafe {
            *self.0.get() = uart;
        }
    }
}

fn uart() -> &'static Uart {
    unsafe { &*UART.0.get() }
}

pub fn reg_base() -> usize {
    unsafe { REG_BASE }
}

pub unsafe fn mmu_add_offset(va_offset: usize) {
    unsafe { REG_BASE += va_offset };
}

pub fn put(byte: u8) {
    unsafe {
        match uart() {
            Uart::Pl011(uart) => uart.write(REG_BASE, byte),
            Uart::AuxMini(uart) => uart.write(REG_BASE, byte),
            Uart::None => {}
        }
    }
}

pub fn init_by_fdt(fdt: &Fdt<'_>) -> Option<()> {
    let chosen = fdt.chosen()?;
    let stdout = chosen.stdout()?;

    let reg = stdout.node.reg()?.next()?;

    unsafe { REG_BASE = reg.address as usize };

    unsafe {
        for c in stdout.node.compatibles() {
            if c.contains("brcm,bcm2835-aux-uart") {
                UART.set(Uart::AuxMini(aux_mini::AuxMini {}));
                break;
            }

            if c.contains("arm,pl011") {
                UART.set(Uart::Pl011(Pl011 {}));
                break;
            }

            if c.contains("arm,primecell") {
                UART.set(Uart::Pl011(Pl011 {}));
                break;
            }
        }
    }

    Some(())
}

enum Uart {
    None,
    Pl011(Pl011),
    AuxMini(AuxMini),
}
