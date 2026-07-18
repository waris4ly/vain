use crate::console::vga_font;

use crate::boot;
use crate::sync::Spinlock;
use core::fmt;

const FONT_WIDTH: usize = 8;
const FONT_HEIGHT: usize = 16;

struct FramebufferWriter {
    x_pos: usize,
    y_pos: usize,
    color: u32,
}

impl FramebufferWriter {
    const fn new() -> Self {
        Self {
            x_pos: 0,
            y_pos: 0,
            color: 0x00FFFFFF, // White
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\r' => self.x_pos = 0,
            byte => {
                let fb_response = boot::FRAMEBUFFER.get_response();
                if let Some(response) = fb_response {
                    if let Some(fb) = response.framebuffers().next() {
                        if self.x_pos + FONT_WIDTH > fb.width() as usize {
                            self.new_line();
                        }
                        self.draw_char(&fb, byte as char);
                        self.x_pos += FONT_WIDTH;
                    }
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.x_pos = 0;
        self.y_pos += FONT_HEIGHT;
        let fb_response = boot::FRAMEBUFFER.get_response();
        if let Some(response) = fb_response {
            if let Some(fb) = response.framebuffers().next() {
                if self.y_pos + FONT_HEIGHT > fb.height() as usize {
                    // For now, just wrap around to the top
                    // A real console would scroll the screen
                    self.y_pos = 0;
                    self.clear_screen(&fb);
                }
            }
        }
    }

    fn draw_char(&self, fb: &limine::framebuffer::Framebuffer, c: char) {
        let ascii = c as usize;
        if ascii >= 256 {
            return;
        }

        let glyph = vga_font::VGA_FONT[ascii];
        let bytes_per_pixel = (fb.bpp() / 8) as usize;
        let pitch = fb.pitch() as usize;

        for (row, byte) in glyph.iter().enumerate() {
            for col in 0..8 {
                if (byte & (0x80 >> col)) != 0 {
                    let pixel_offset =
                        (self.y_pos + row) * pitch + (self.x_pos + col) * bytes_per_pixel;
                    unsafe {
                        let ptr = fb.addr().add(pixel_offset) as *mut u32;
                        ptr.write_volatile(self.color);
                    }
                }
            }
        }
    }

    fn clear_screen(&self, fb: &limine::framebuffer::Framebuffer) {
        let size = (fb.pitch() as usize) * (fb.height() as usize);
        unsafe {
            core::ptr::write_bytes(fb.addr() as *mut u8, 0, size);
        }
    }
}

impl fmt::Write for FramebufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

static WRITER: Spinlock<FramebufferWriter> = Spinlock::new(FramebufferWriter::new());

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
