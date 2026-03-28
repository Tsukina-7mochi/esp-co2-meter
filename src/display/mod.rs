mod frame_buffer;

use crate::display::frame_buffer::Invertible;
use core::ops::Index;
use embedded_graphics::geometry::AnchorX;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::primitives::{Line, PrimitiveStyleBuilder, StyledDrawable};
use embedded_graphics::{
    image::Image,
    mono_font::{self, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Rectangle,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use frame_buffer::MyFrameBuffer;
use scd4x::types::SensorData;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
use tinybmp::Bmp;

type PhysicalDisplay<I> =
    Ssd1306<I2CInterface<I>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

const IMG_0: ImageRaw<BinaryColor> = ImageRaw::new(
    &[
        0b_111_00000,
        0b_101_00000,
        0b_101_00000,
        0b_101_00000,
        0b_111_00000,
    ],
    3,
);
const IMG_1K: ImageRaw<BinaryColor> = ImageRaw::new(
    &[
        0b_010_0_101_0,
        0b_010_0_110_0,
        0b_010_0_100_0,
        0b_010_0_110_0,
        0b_010_0_101_0,
    ],
    7,
);
const IMG_2K: ImageRaw<BinaryColor> = ImageRaw::new(
    &[
        0b_111_0_101_0,
        0b_001_0_110_0,
        0b_111_0_100_0,
        0b_100_0_110_0,
        0b_111_0_101_0,
    ],
    7,
);
const IMG_1H: ImageRaw<BinaryColor> = ImageRaw::new(
    &[
        0b_010_0_101_0,
        0b_010_0_101_0,
        0b_010_0_111_0,
        0b_010_0_101_0,
        0b_010_0_101_0,
    ],
    7,
);
const IMG_2H: ImageRaw<BinaryColor> = ImageRaw::new(
    &[
        0b_111_0_101_0,
        0b_001_0_101_0,
        0b_111_0_111_0,
        0b_100_0_101_0,
        0b_111_0_101_0,
    ],
    7,
);

struct BarChart<T> {
    min: T,
    max: T,
    value: T,
    rect: Rectangle,
}

impl<T> BarChart<T> {
    fn text_top_right(&self) -> Point {
        self.rect.top_left + Point::new((self.rect.size.width as i32) - 2, -1)
    }

    fn bar_rect(&self) -> Rectangle
    where
        T: Clone + Into<f32>,
    {
        let min = self.min.clone().into();
        let max = self.max.clone().into();
        let value = self.value.clone().into();
        let ratio = (value - min) / (max - min);
        let width = ((self.rect.size.width as f32) * ratio) as u32;
        self.rect.resized_width(width, AnchorX::Left)
    }
}

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

    pub fn toggle_on_with_initialization_message(&mut self) {
        self.display.clear_buffer();
        self.draw_initialization_message();
        self.display.flush().unwrap();
        self.display.set_display_on(true).unwrap();
    }

    pub fn toggle_on_with_measurement(&mut self, measurement: &SensorData) {
        self.display.clear_buffer();
        self.draw_bar_chart(measurement);
        self.display.flush().unwrap();
        self.display.set_display_on(true).unwrap();
    }

    pub fn toggle_on_with_history<T, U>(&mut self, history: &T)
    where
        T: Index<usize, Output = U>,
        U: Clone + Into<i32>,
    {
        self.display.clear_buffer();
        self.draw_line_chart(history);
        self.display.flush().unwrap();
        self.display.set_display_on(true).unwrap();
    }

    pub fn toggle_off(&mut self) {
        self.display.set_display_on(false).unwrap();
    }

    fn draw_initialization_message(&mut self) {
        let char_style = MonoTextStyle::new(&mono_font::ascii::FONT_6X10, BinaryColor::On);
        let text_style = TextStyleBuilder::new()
            .alignment(Alignment::Center)
            .baseline(Baseline::Middle)
            .build();
        let bounds = self.display.bounding_box();

        Text::with_text_style(
            "Initializing...",
            bounds.top_left + bounds.size / 2,
            char_style,
            text_style,
        )
        .draw(&mut self.display)
        .unwrap();
    }

    fn draw_bar_chart(&mut self, measurement: &SensorData) {
        let mut frame_buf = MyFrameBuffer::new();

        let screen_img = include_bytes!("../img/screen.bmp");
        let screen_img = Bmp::<BinaryColor>::from_slice(screen_img).unwrap();
        Image::new(&screen_img, Point::zero())
            .draw(&mut frame_buf)
            .unwrap();

        let tmp_chart = BarChart {
            min: 13.0,
            max: 33.0,
            value: measurement.temperature,
            rect: Rectangle::new(Point::new(2, 2), Size::new(124, 16)),
        };
        let rh_chart = BarChart {
            min: 0.0,
            max: 100.0,
            value: measurement.humidity,
            rect: Rectangle::new(Point::new(2, 24), Size::new(124, 16)),
        };
        let co2_chart = BarChart {
            min: 0,
            max: 2000,
            value: measurement.co2,
            rect: Rectangle::new(Point::new(2, 46), Size::new(124, 16)),
        };

        let char_style = MonoTextStyle::new(&mono_font::ascii::FONT_9X18_BOLD, BinaryColor::On);
        let text_style = TextStyleBuilder::new()
            .alignment(Alignment::Right)
            .baseline(Baseline::Top)
            .build();

        let mut itoa_buf = itoa::Buffer::new();
        Text::with_text_style(
            itoa_buf.format(tmp_chart.value as u32),
            tmp_chart.text_top_right(),
            char_style,
            text_style,
        )
        .draw(&mut frame_buf)
        .unwrap();
        Text::with_text_style(
            itoa_buf.format(rh_chart.value as u32),
            rh_chart.text_top_right(),
            char_style,
            text_style,
        )
        .draw(&mut frame_buf)
        .unwrap();
        Text::with_text_style(
            itoa_buf.format(co2_chart.value as u32),
            co2_chart.text_top_right(),
            char_style,
            text_style,
        )
        .draw(&mut frame_buf)
        .unwrap();

        frame_buf.invert_rect(tmp_chart.bar_rect());
        frame_buf.invert_rect(rh_chart.bar_rect());
        frame_buf.invert_rect(co2_chart.bar_rect());

        frame_buf.as_image().draw(&mut self.display).unwrap();
    }

    fn draw_line_chart<T, U>(&mut self, history: &T)
    where
        T: Index<usize, Output = U>,
        U: Clone + Into<i32>,
    {
        let rect_style = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .build();
        Rectangle::new(Point::new(8, 0), Size::new(120, 56))
            .draw_styled(&rect_style, &mut self.display)
            .unwrap();

        Image::new(&IMG_2K, Point::new(0, 0))
            .draw(&mut self.display)
            .unwrap();
        Image::new(&IMG_1K, Point::new(0, 28))
            .draw(&mut self.display)
            .unwrap();
        Image::new(&IMG_0, Point::new(4, 59))
            .draw(&mut self.display)
            .unwrap();
        Image::new(&IMG_1H, Point::new(60, 59))
            .draw(&mut self.display)
            .unwrap();
        Image::new(&IMG_2H, Point::new(121, 59))
            .draw(&mut self.display)
            .unwrap();

        let mut prev_value = history[0].clone().into();
        for i in 1..120 {
            let prev_x = 8 + (i as i32) - 1;
            let x = 8 + (i as i32);
            let value = history[i].clone().into();

            let prev_height = prev_value * 56 / 2000;
            let height = value * 56 / 2000;
            Line::new(
                Point::new(prev_x, 56 - prev_height),
                Point::new(x, 56 - height),
            )
            .draw_styled(&rect_style, &mut self.display)
            .unwrap();

            prev_value = value;
        }
    }
}
