#![no_std]
#![no_main]

use dma_api::DVec;

pub fn new_dma() -> DVec<u8> {
    DVec::zeros(u64::MAX, 10, 0x1000, dma_api::Direction::Bidirectional).unwrap()
}
