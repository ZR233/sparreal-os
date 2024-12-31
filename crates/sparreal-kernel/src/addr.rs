#![cfg_attr(not(test), no_std)]

use core::ops::{Add, Sub};

#[derive(Debug, Clone, Copy)]
pub struct Address {
    pub cpu: usize,
    pub virt: Option<usize>,
    pub bus: Option<u64>,
}

impl Address {
    pub fn new(cpu: usize, virt: Option<*mut u8>, bus: Option<u64>) -> Self {
        Self {
            cpu,
            virt: virt.map(|s| s as usize),
            bus,
        }
    }

    pub fn as_ptr(&self) -> *const u8 {
        match self.virt {
            Some(virt) => virt as *const u8,
            None => self.cpu as *const u8,
        }
    }

    pub fn bus(&self) -> u64 {
        match self.bus {
            Some(bus) => bus,
            None => self.cpu as _,
        }
    }

    pub fn physical(&self) -> usize {
        self.cpu
    }
}

impl Add<usize> for Address {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            cpu: self.cpu + rhs,
            virt: self.virt.map(|s| s + rhs),
            bus: self.bus.map(|s| s + rhs as u64),
        }
    }
}

impl Sub<usize> for Address {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            cpu: self.cpu - rhs,
            virt: self.virt.map(|s| s - rhs),
            bus: self.bus.map(|s| s - rhs as u64),
        }
    }
}
