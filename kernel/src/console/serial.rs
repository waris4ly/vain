use core::fmt;

const COM1_PORT: u16 = 0x3F8;
const BAUD_DIVISOR_115200: u16 = 1;

const UART_DATA: u16 = 0;
const UART_INTERRUPT_ENABLE: u16 = 1;
const UART_FIFO_CONTROL: u16 = 2;
const UART_LINE_CONTROL: u16 = 3;
const UART_MODEM_CONTROL: u16 = 4;
const UART_LINE_STATUS: u16 = 5;

const LINE_STATUS_TRANSMIT_EMPTY: u8 = 0x20;
const DLAB_ENABLE: u8 = 0x80;
const WORD_LENGTH_8BIT: u8 = 0x03;
const FIFO_ENABLE_AND_CLEAR: u8 = 0xC7;
const DTR_RTS_OUT2: u8 = 0x0B;

#[derive(Clone, Copy)]
pub enum SerialPort {
    COM1,
}

impl SerialPort {
    fn base_port(self) -> u16 {
        match self {
            SerialPort::COM1 => COM1_PORT,
        }
    }

    pub fn initialize(self) {
        let port = self.base_port();

        unsafe {
            write_port(port + UART_INTERRUPT_ENABLE, 0x00);
            write_port(port + UART_LINE_CONTROL, DLAB_ENABLE);
            write_port(port + UART_DATA, BAUD_DIVISOR_115200 as u8);
            write_port(
                port + UART_INTERRUPT_ENABLE,
                (BAUD_DIVISOR_115200 >> 8) as u8,
            );
            write_port(port + UART_LINE_CONTROL, WORD_LENGTH_8BIT);
            write_port(port + UART_FIFO_CONTROL, FIFO_ENABLE_AND_CLEAR);
            write_port(port + UART_MODEM_CONTROL, DTR_RTS_OUT2);
        }
    }

    fn wait_transmit_ready(self) {
        let port = self.base_port();
        unsafe {
            while read_port(port + UART_LINE_STATUS) & LINE_STATUS_TRANSMIT_EMPTY == 0 {
                core::hint::spin_loop();
            }
        }
    }

    pub fn write_byte(self, byte: u8) {
        self.wait_transmit_ready();
        unsafe {
            write_port(self.base_port(), byte);
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    let mut serial = SerialPort::COM1;
    serial.write_fmt(args).unwrap();
}

unsafe fn write_port(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

unsafe fn read_port(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}
