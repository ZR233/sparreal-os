use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use core::ops::{Add, Deref, Range, Sub};
use core::ptr::NonNull;

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

macro_rules! def_addr {
    ($name:ident, $t:ty) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, PartialOrd)]
        pub struct $name<T>($t, core::marker::PhantomData<T>);

        impl<T> $name<T> {
            pub const fn new(val: $t) -> Self {
                Self(val, core::marker::PhantomData)
            }

            pub fn raw(self) -> $t {
                self.0
            }

            pub fn align_down(self, align: usize) -> Self {
                (align_down(self.0 as _, align) as $t).into()
            }

            pub fn align_up(self, align: usize) -> Self {
                (align_up(self.0 as _, align) as $t).into()
            }

            pub fn align_offset(self, align: usize) -> usize {
                align_offset(self.0 as _, align)
            }

            pub fn is_aligned_4k(self) -> bool {
                self.is_aligned_to(0x1000)
            }

            pub fn is_aligned_to(self, align: usize) -> bool {
                self.align_offset(align) == 0
            }
        }
        impl<T> core::fmt::Debug for $name<T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{:#x}", self.0)
            }
        }
        impl<T> core::fmt::Display for $name<T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{:#x}", self.0)
            }
        }
        impl<T> From<$t> for $name<T> {
            fn from(value: $t) -> Self {
                Self(value, core::marker::PhantomData)
            }
        }
        impl<T> From<$name<T>> for $t {
            fn from(value: $name<T>) -> Self {
                value.0
            }
        }

        impl<T> core::ops::Add<usize> for $name<T> {
            type Output = Self;

            fn add(self, rhs: usize) -> Self::Output {
                Self(self.0 as usize + rhs, core::marker::PhantomData)
            }
        }

        impl<T> core::ops::Sub<usize> for $name<T> {
            type Output = Self;

            fn sub(self, rhs: usize) -> Self::Output {
                Self(self.0 as usize - rhs, core::marker::PhantomData)
            }
        }

        impl<T> core::ops::Sub<Self> for $name<T> {
            type Output = usize;

            fn sub(self, rhs: Self) -> Self::Output {
                self.0 as usize - rhs.0 as usize
            }
        }
    };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CRange<T: Debug + Sized + Clone + Copy> {
    pub start: T,
    pub end: T,
}

impl<T: Debug + Sized + Clone + Copy> CRange<T> {
    pub fn to_range(&self) -> Range<T> {
        self.start..self.end
    }
}

impl<T: Debug + Sized + Clone + Copy> From<Range<T>> for CRange<T> {
    fn from(value: Range<T>) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl<T: Debug + Sized + Clone + Copy> Debug for CRange<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{:?}, {:?})", self.start, self.end)
    }
}

def_addr!(Virt, usize);
def_addr!(Phys, usize);

pub type VirtAddr = Virt<u8>;
pub type PhysAddr = Phys<u8>;
pub type VirtCRange = CRange<VirtAddr>;
pub type PhysCRange = CRange<PhysAddr>;

/// 运行地址
pub struct RunAddr(usize);

pub const fn align_offset(addr: usize, align: usize) -> usize {
    addr & (align - 1)
}

pub const fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

pub const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

impl<T> From<Virt<T>> for *const T {
    fn from(value: Virt<T>) -> Self {
        value.0 as _
    }
}

impl<T> From<Virt<T>> for *mut T {
    fn from(value: Virt<T>) -> *mut T {
        value.0 as _
    }
}

impl<T> From<NonNull<T>> for Virt<T> {
    fn from(value: NonNull<T>) -> Self {
        Self(value.as_ptr() as _, core::marker::PhantomData)
    }
}

#[macro_export]
macro_rules! pa {
    (val: $val:expr) => {
        Phys::new($val as _)
    };
}

#[macro_export]
macro_rules! va {
    (val: $val:expr) => {
        Virt::new($val as _)
    };
}
