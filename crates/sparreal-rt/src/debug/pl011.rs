use core::sync::atomic::{Ordering, fence};

pub struct Pl011 {}

impl Pl011 {
    pub fn write(&self, base: usize, byte: u8) {
        const TXFF: u8 = 1 << 5;

        unsafe {
            let state = (base + 0x18) as *mut u8;
            let put = (base) as *mut u8;
            while (state.read_volatile() & TXFF) != 0 {}
            put.write_volatile(byte);
        }
    }
}
