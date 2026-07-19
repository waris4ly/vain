use crate::boot;
use crate::sync::Spinlock;
use core::fmt;

const FONT_WIDTH: usize = 8;
const FONT_HEIGHT: usize = 16;
const CHAR_SPACING: usize = 1;
const LINE_SPACING: usize = 4;

struct FramebufferWriter {
    x_pos: usize,
    y_pos: usize,
    color: u32,
    bg_color: u32,
}

impl FramebufferWriter {
    const fn new() -> Self {
        Self {
            x_pos: 0,
            y_pos: 0,
            color: 0x00FFFFFF,
            bg_color: 0x00000000,
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
                        if self.x_pos + FONT_WIDTH + CHAR_SPACING > fb.width() as usize {
                            self.new_line();
                        }
                        self.draw_char(&fb, byte as char);
                        self.x_pos += FONT_WIDTH + CHAR_SPACING;
                    }
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.x_pos = 0;
        self.y_pos += FONT_HEIGHT + LINE_SPACING;
        let fb_response = boot::FRAMEBUFFER.get_response();
        if let Some(response) = fb_response {
            if let Some(fb) = response.framebuffers().next() {
                if self.y_pos + FONT_HEIGHT + LINE_SPACING > fb.height() as usize {
                    self.y_pos = 0;
                    self.clear_screen(&fb);
                }
            }
        }
    }

    fn draw_char(&self, fb: &limine::framebuffer::Framebuffer, c: char) {
        let glyph = super::vga_font::get_glyph(c);
        let bytes_per_pixel = (fb.bpp() / 8) as usize;
        let pitch = fb.pitch() as usize;

        for row in 0..16 {
            let byte = glyph[row];
            for col in 0..8 {
                let y = self.y_pos + row;
                let x = self.x_pos + col;

                if y < fb.height() as usize && x < fb.width() as usize {
                    let pixel_offset = y * pitch + x * bytes_per_pixel;
                    let color = if (byte & (1 << col)) != 0 {
                        self.color
                    } else {
                        self.bg_color
                    };
                    
                    unsafe {
                        let ptr = fb.addr().add(pixel_offset) as *mut u32;
                        ptr.write_volatile(color);
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

pub fn clear_screen() {
    let fb_response = boot::FRAMEBUFFER.get_response();
    if let Some(response) = fb_response {
        if let Some(fb) = response.framebuffers().next() {
            WRITER.lock().clear_screen(&fb);
            WRITER.lock().x_pos = 0;
            WRITER.lock().y_pos = 0;
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
