//! Palette-free Bayer thresholds with compile-time tables from the frozen rank formula.

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

pub const fn bayer_value(x: usize, y: usize, width: usize) -> u16 {
    assert!(width.is_power_of_two());
    assert!(width >= 2 && width <= 16);
    let mut value = 0;
    let mut bit = 1;
    while bit < width {
        let rx = ((x & bit) != 0) as usize;
        let ry = ((y & bit) != 0) as usize;
        value = (value << 2) | ((rx ^ ry) << 1) | ry;
        bit <<= 1;
    }
    value as u16
}

const fn threshold_numerators<const N: usize>(width: usize) -> [f32; N] {
    let mut values = [0.0; N];
    let mut index = 0;
    while index < N {
        let rank = bayer_value(index % width, index / width, width);
        // Integer construction also supports the pinned threaded-build toolchain.
        values[index] = (2 * rank as i32 + 1 - N as i32) as f32;
        index += 1;
    }
    values
}

static TWO: [f32; 4] = threshold_numerators(2);
static FOUR: [f32; 16] = threshold_numerators(4);
static EIGHT: [f32; 64] = threshold_numerators(8);
static SIXTEEN: [f32; 256] = threshold_numerators(16);

/// Palette-free centered Bayer threshold at global image coordinates.
/// Matrix cell centers avoid either endpoint of [-0.5,0.5].
#[inline]
pub fn bayer_noise_at(x: u32, y: u32, size: BayerSize) -> f32 {
    let width = size.width();
    // Scale by 1 / (2 * cell count); every centered threshold is exactly representable.
    let (values, scale): (&[f32], f32) = match size {
        BayerSize::Two => (&TWO, 0.125),
        BayerSize::Four => (&FOUR, 0.03125),
        BayerSize::Eight => (&EIGHT, 0.0078125),
        BayerSize::Sixteen => (&SIXTEEN, 0.001953125),
    };
    let mask = width - 1;
    values[(y as usize & mask) * width + (x as usize & mask)] * scale
}
