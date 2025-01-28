mod constants;
use core::{
    fmt::{self, Write},
    ptr,
};
use bootloader_api::info::FrameBufferInfo;
use constants::font_constants;
use constants::font_constants::{BACKUP_CHAR, CHAR_RASTER_HEIGHT, FONT_WEIGHT};
use noto_sans_mono_bitmap::{get_raster, RasterizedChar};

const LINE_SPACING: usize = 2;
const LETTER_SPACING: usize = 0;
const BORDER_PADDING: usize = 1;

// ANSI-like color codes
const COLOR_BLUE: [u8; 3] = [255, 0, 0]; // RGB for blue
const COLOR_WHITE: [u8; 3] = [255, 255, 255]; // RGB for white (default color)

fn get_char_raster(c: char) -> RasterizedChar {
    fn get(c: char) -> Option<RasterizedChar> {
        get_raster(c, FONT_WEIGHT, CHAR_RASTER_HEIGHT)
    }
    get(c).unwrap_or_else(|| get(BACKUP_CHAR).expect("Should get raster of backup char."))
}

pub struct FrameBufferWriter {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    x_pos: usize,
    y_pos: usize,
    current_color: [u8; 3],
}

impl FrameBufferWriter {
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut writer = Self {
            framebuffer,
            info,
            x_pos: BORDER_PADDING,
            y_pos: BORDER_PADDING,
            current_color: COLOR_WHITE,
        };
        writer.clear();
        writer
    }

    /// Prints text with automatic wrapping, scrolling, and ANSI-like escape sequences.
    pub fn print(&mut self, text: &str) {
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    if let Some(next) = chars.next() {
                        match next {
                            'c' => self.current_color = COLOR_BLUE,  // Change to blue
                            'r' => self.current_color = COLOR_WHITE, // Reset to white
                            _ => self.write_char(c),                // Unknown sequence
                        }
                    }
                }
                '\n' => self.newline(),
                '\t' => self.indent_tab(),
                _ => self.write_char(c),
            }
        }
    }

    fn newline(&mut self) {
        self.x_pos = BORDER_PADDING;
        self.y_pos += CHAR_RASTER_HEIGHT.val() + LINE_SPACING;
        if self.y_pos + CHAR_RASTER_HEIGHT.val() >= self.height() {
            self.scroll_up();
        }
    }

    fn scroll_up(&mut self) {
        let row_height = CHAR_RASTER_HEIGHT.val() + LINE_SPACING;
        let offset = self.info.stride * row_height;

        // Shift framebuffer content up
        self.framebuffer.copy_within(offset.., 0);

        // Clear the last row
        let start_of_last_row = self.framebuffer.len() - self.width() * row_height;
        for pixel in &mut self.framebuffer[start_of_last_row..] {
            *pixel = 0;
        }

        self.y_pos = self.height() - row_height;
    }

    fn indent_tab(&mut self) {
        for _ in 0..4 {
            self.write_char(' ');
        }
    }

    fn write_char(&mut self, c: char) {
        if c == '\n' {
            self.newline();
            return;
        }

        let char_width = font_constants::CHAR_RASTER_WIDTH;
        if self.x_pos + char_width >= self.width() {
            self.newline();
        }

        let char_height = CHAR_RASTER_HEIGHT.val();
        if self.y_pos + char_height >= self.height() {
            self.scroll_up();
        }

        self.write_rendered_char(get_char_raster(c));
    }

    fn write_rendered_char(&mut self, rendered_char: RasterizedChar) {
        let x_start = self.x_pos;
        let y_start = self.y_pos;

        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                if *byte > 0 {
                    self.write_pixel(x_start + x, y_start + y, *byte);
                }
            }
        }

        self.x_pos += rendered_char.width() + LETTER_SPACING;
    }

    fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        if x >= self.width() || y >= self.height() {
            return;
        }

        let pixel_offset = y * self.info.stride + x;
        let color = [
            (self.current_color[0] as u16 * intensity as u16 / 255) as u8,
            (self.current_color[1] as u16 * intensity as u16 / 255) as u8,
            (self.current_color[2] as u16 * intensity as u16 / 255) as u8,
        ];
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }

    pub fn clear(&mut self) {
        self.x_pos = BORDER_PADDING;
        self.y_pos = BORDER_PADDING;
        self.framebuffer.fill(0);
    }

    fn width(&self) -> usize {
        self.info.width
    }

    fn height(&self) -> usize {
        self.info.height
    }
}

unsafe impl Send for FrameBufferWriter {}
unsafe impl Sync for FrameBufferWriter {}

impl Write for FrameBufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print(s);
        Ok(())
    }
}
