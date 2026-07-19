#![no_std]
#![no_main]

use driver_framework::{Driver, run_driver};
use libos::print;
use libos::syscall;

const PS2_DATA_PORT: u16 = 0x60;

static SCANCODE_MAP_LOWER: &[u8] =
    b"??1234567890-=\x08\tqwertyuiop[]\n?asdfghjkl;'`?\\zxcvbnm,./?*? ?";
static SCANCODE_MAP_UPPER: &[u8] =
    b"??!@#$%^&*()_+\x08\tQWERTYUIOP{}\n?ASDFGHJKL:\"~?\\ZXCVBNM<>??*? ?";

struct KeyboardDriver {
    shift_pressed: bool,
}

impl Driver for KeyboardDriver {
    fn init(&mut self) -> Result<(), &'static str> {
        // Clear out any pending bytes
        while (syscall::sys_port_in(0x64) & 1) != 0 {
            syscall::sys_port_in(PS2_DATA_PORT);
        }

        // Enable first PS/2 port
        syscall::sys_port_out(0x64, 0xAE);

        // Read configuration byte
        syscall::sys_port_out(0x64, 0x20);
        let mut config = syscall::sys_port_in(0x60);

        // Enable first PS/2 port interrupt (bit 0)
        config |= 1;

        // Write configuration byte
        syscall::sys_port_out(0x64, 0x60);
        syscall::sys_port_out(0x60, config);

        Ok(())
    }

    fn handle_interrupt(&mut self) {
        libos::print!("*");
        let scancode = syscall::sys_port_in(PS2_DATA_PORT);

        match scancode {
            0x2A | 0x36 => self.shift_pressed = true,
            0xAA | 0xB6 => self.shift_pressed = false,
            _ => {
                if scancode < 0x80 {
                    let idx = scancode as usize;
                    if idx < SCANCODE_MAP_LOWER.len() {
                        let char_byte = if self.shift_pressed {
                            SCANCODE_MAP_UPPER[idx]
                        } else {
                            SCANCODE_MAP_LOWER[idx]
                        };

                        if char_byte != b'?' {
                            print!("{}", char_byte as char);
                        }
                    }
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let driver = KeyboardDriver {
        shift_pressed: false,
    };

    let irq_handle = 0;

    run_driver(driver, irq_handle)
}
