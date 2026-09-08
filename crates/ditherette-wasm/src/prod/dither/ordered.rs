//! Literal palette-free Bayer field fragments from the frozen ordered reference.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BayerSize {
    Two,
    Four,
    Eight,
    Sixteen,
}

impl BayerSize {
    pub const fn width(self) -> usize {
        match self {
            Self::Two => 2,
            Self::Four => 4,
            Self::Eight => 8,
            Self::Sixteen => 16,
        }
    }
}

pub fn bayer_value(x: usize, y: usize, width: usize) -> u16 {
    assert!(width.is_power_of_two());
    assert!((2..=16).contains(&width));
    let mut value = 0;
    let mut bit = 1;
    while bit < width {
        let rx = usize::from((x & bit) != 0);
        let ry = usize::from((y & bit) != 0);
        value = (value << 2) | ((rx ^ ry) << 1) | ry;
        bit <<= 1;
    }
    value as u16
}

/// Palette-free centered Bayer threshold at global image coordinates.
/// Matrix cell centers avoid either endpoint of [-0.5,0.5].
pub fn bayer_noise_at(x: u32, y: u32, size: BayerSize) -> f32 {
    let width = size.width();
    let rank = bayer_value(x as usize % width, y as usize % width, width);
    (f32::from(rank) + 0.5) / (width * width) as f32 - 0.5
}
