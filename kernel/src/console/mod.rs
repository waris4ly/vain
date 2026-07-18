pub mod framebuffer;
pub mod keyboard;
pub mod serial;
pub mod vga_font;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::serial::_print(core::format_args!($($arg)*));
        $crate::console::framebuffer::_print(core::format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", core::format_args!($($arg)*)));
}
