use core::cell::LazyCell;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::pixelcolor::BinaryColor;
use tinybmp::Bmp;

const BACKGROUND_RAW: &[u8] = include_bytes!("./screen.bmp");
pub const BACKGROUND: LazyCell<Bmp<BinaryColor>> = LazyCell::new(|| {
    Bmp::<BinaryColor>::from_slice(BACKGROUND_RAW).unwrap()
});

pub const TEXT_0: ImageRaw<BinaryColor> = ImageRaw::new(
    &[
        0b_111_00000,
        0b_101_00000,
        0b_101_00000,
        0b_101_00000,
        0b_111_00000,
    ],
    3,
);
pub const TEXT_1K: ImageRaw<BinaryColor> = ImageRaw::new(
    &[
        0b_010_0_101_0,
        0b_010_0_110_0,
        0b_010_0_100_0,
        0b_010_0_110_0,
        0b_010_0_101_0,
    ],
    7,
);
pub const TEXT_2K: ImageRaw<BinaryColor> = ImageRaw::new(
    &[
        0b_111_0_101_0,
        0b_001_0_110_0,
        0b_111_0_100_0,
        0b_100_0_110_0,
        0b_111_0_101_0,
    ],
    7,
);
pub const TEXT_1H: ImageRaw<BinaryColor> = ImageRaw::new(
    &[
        0b_010_0_101_0,
        0b_010_0_101_0,
        0b_010_0_111_0,
        0b_010_0_101_0,
        0b_010_0_101_0,
    ],
    7,
);
pub const TEXT_2H: ImageRaw<BinaryColor> = ImageRaw::new(
    &[
        0b_111_0_101_0,
        0b_001_0_101_0,
        0b_111_0_111_0,
        0b_100_0_101_0,
        0b_111_0_101_0,
    ],
    7,
);
