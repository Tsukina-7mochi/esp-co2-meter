mod frame_buffer;

use embedded_graphics::{
    image::Image,
    mono_font::{self, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Rectangle,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use embedded_graphics::geometry::AnchorX;
use scd4x::types::SensorData;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
use tinybmp::Bmp;
use frame_buffer::MyFrameBuffer;
use crate::display::frame_buffer::Invertible;

type PhysicalDisplay<I> =
    Ssd1306<I2CInterface<I>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

pub struct Display<I> {
    display: PhysicalDisplay<I>,
}

impl<I> Display<I>
where
    I: embedded_hal::i2c::I2c,
{
    pub fn new(i2c: I) -> Self {
        let interface = I2CDisplayInterface::new(i2c);
        let display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        Self { display }
    }

    pub fn init(&mut self) {
        self.display.init().unwrap();
        self.display.set_display_on(false).unwrap();
    }

    pub fn toggle_on_with(&mut self, measurement: &SensorData) {
        self.draw(measurement);
        self.display.flush().unwrap();
        self.display.set_display_on(true).unwrap();
    }

    pub fn toggle_off(&mut self) {
        self.display.set_display_on(false).unwrap();
    }

    fn draw(&mut self, measurement: &SensorData) {
        let mut frame_buf = MyFrameBuffer::new();

        let screen_img = include_bytes!("../img/screen.bmp");
        let screen_img = Bmp::<BinaryColor>::from_slice(screen_img).unwrap();
        Image::new(&screen_img, Point::zero())
            .draw(&mut frame_buf)
            .unwrap();

        let char_style = MonoTextStyle::new(&mono_font::ascii::FONT_9X18_BOLD, BinaryColor::On);
        let text_style = TextStyleBuilder::new()
            .alignment(Alignment::Right)
            .baseline(Baseline::Top)
            .build();

        let mut itoa_buf = itoa::Buffer::new();
        Text::with_text_style(
            itoa_buf.format(measurement.temperature as u32),
            Point::new(124, 1),
            char_style,
            text_style,
        )
        .draw(&mut frame_buf)
        .unwrap();
        Text::with_text_style(
            itoa_buf.format((measurement.humidity) as u32),
            Point::new(124, 23),
            char_style,
            text_style,
        )
        .draw(&mut frame_buf)
        .unwrap();
        Text::with_text_style(
            itoa_buf.format(measurement.co2 as u32),
            Point::new(124, 45),
            char_style,
            text_style,
        )
        .draw(&mut frame_buf)
        .unwrap();

        let tmp_area = Rectangle::new(Point::new(2, 2), Size::new(124, 16));
        let rh_area = Rectangle::new(Point::new(2, 24), Size::new(124, 16));
        let co2_area = Rectangle::new(Point::new(2, 46), Size::new(124, 16));

        let tmp_width = tmp_area.size.width * ((measurement.temperature as u32) - 13) / 20;
        let rh_width = ((rh_area.size.width as f32) * measurement.humidity / 100.0) as u32;
        let co2_width = co2_area.size.width * (measurement.co2 as u32) / 2000;

        let tmp_width = tmp_width.clamp(0, tmp_area.size.width);
        let rh_width = rh_width.clamp(0, rh_area.size.width);
        let co2_width = co2_width.clamp(0, co2_area.size.width);

        frame_buf.invert_rect(tmp_area.resized_width(tmp_width, AnchorX::Left));
        frame_buf.invert_rect(rh_area.resized_width(rh_width, AnchorX::Left));
        frame_buf.invert_rect(co2_area.resized_width(co2_width, AnchorX::Left));

        frame_buf.as_image().draw(&mut self.display).unwrap();
    }
}
