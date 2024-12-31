pub use sparreal_macros::entry;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::__export::print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        $crate::print!("{}\r\n", format_args!($($arg)*));
    };
}
