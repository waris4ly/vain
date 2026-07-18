use crate::console::serial::SerialPort;
use core::arch::asm;
use core::fmt::Write;

const PS2_DATA_PORT: u16 = 0x60;

// A very simple Scancode Set 1 map for printable characters
static SCANCODE_MAP_LOWER: &[u8] =
    b"??1234567890-=\x08\tqwertyuiop[]\n?asdfghjkl;'`?\\zxcvbnm,./?*? ?";
static SCANCODE_MAP_UPPER: &[u8] =
    b"??!@#$%^&*()_+\x08\tQWERTYUIOP{}\n?ASDFGHJKL:\"~?\\ZXCVBNM<>??*? ?";

static mut SHIFT_PRESSED: bool = false;

unsafe fn inb(port: u16) -> u8 {
    let mut val: u8;
    unsafe {
        asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags))
    };
    val
}

pub fn handle_interrupt() {
    let scancode = unsafe { inb(PS2_DATA_PORT) };

    match scancode {
        0x2A | 0x36 => unsafe { SHIFT_PRESSED = true }, // Left or Right Shift pressed
        0xAA | 0xB6 => unsafe { SHIFT_PRESSED = false }, // Left or Right Shift released
        _ => {
            if scancode < 0x80 {
                // Key pressed
                let idx = scancode as usize;
                if idx < SCANCODE_MAP_LOWER.len() {
                    let is_shift = unsafe { SHIFT_PRESSED };
                    let char_byte = if is_shift {
                        SCANCODE_MAP_UPPER[idx]
                    } else {
                        SCANCODE_MAP_LOWER[idx]
                    };

                    if char_byte != b'?' {
                        let mut serial = SerialPort::COM1;
                        // Print to serial
                        let _ = write!(serial, "{}", char_byte as char);
                        // And also to VGA, though we haven't implemented a proper TTY yet.
                        // We can just log it using crate::print! instead of direct serial.
                        crate::print!("{}", char_byte as char);
                    }
                }
            }
        }
    }
}
