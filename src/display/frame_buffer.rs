use embedded_graphics::framebuffer::{buffer_size, Framebuffer};
use embedded_graphics::pixelcolor::raw::{BigEndian, RawU1};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::primitives::Rectangle;

pub type MyFrameBuffer =
Framebuffer<BinaryColor, RawU1, BigEndian, 128, 64, { buffer_size::<BinaryColor>(128, 64) }>;

pub trait Invertible {
    fn invert_rect(&mut self, rect: Rectangle);
}

fn create_mask_128(x: u32, width: u32) -> u128 {
    let right = 128 - (x + width);
    !(!0u128 << width) << right
}

impl Invertible for MyFrameBuffer {
    fn invert_rect(&mut self, rect: Rectangle) {
        let buf = self.data_mut();
        let mask = create_mask_128(rect.top_left.x as u32, rect.size.width);
        for y in rect.rows() {
            for i in 0..16 {
                buf[(y * 16 + i) as usize] ^= (mask >> (15 - i) * 8) as u8;
            }
        }
    }
}
