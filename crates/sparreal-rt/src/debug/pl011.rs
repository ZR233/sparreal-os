use core::sync::atomic::{Ordering, fence};

pub struct Pl011 {}

impl Pl011 {
    pub fn write(&self, base: usize, byte: u8) {
        const TXFF: u32 = 1 << 5;

        unsafe {
            let state = (base + 0x18) as *const u32;
            loop {
                let lsr = state.read_volatile();

                fence(Ordering::Release);
                if lsr & 1 == 0 {
                    break;
                }
            }
            let data = base as *mut u32;
            data.write_volatile(byte as _);
        }
    }
}
