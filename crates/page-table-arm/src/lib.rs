#![no_std]

mod pte64;
pub use pte64::*;

#[derive(Debug, Clone, Copy)]
pub enum MAIRKind {
    Device,
    Normal,
    NonCache,
}

pub trait MAIRSetting {
    fn get_idx(kind: MAIRKind) -> usize;
    fn from_idx(idx: usize) -> MAIRKind;
}
