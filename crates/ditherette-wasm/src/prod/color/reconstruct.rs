//! Scalar reconstruction for field offsets, retaining f32 source-color storage.
//! The existing inverse formulas use f64 arithmetic here to cover every valid f32 strength.

use crate::prod::contract::request::WorkingSpace;

/// Reconstructs field-generated coordinates, clipping encoded RGB and rounding byte ties upward.
/// Coordinates may exceed source gamut boxes; valid field offsets remain finite through these f64 formulas.
pub fn coordinates_to_rgb8(coordinates: [f64; 3], space: WorkingSpace) -> [u8; 3] {
    coordinates_to_srgb(coordinates, space)
        .map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Unclipped encoded RGB for field-generated coordinates, before the explicit byte boundary.
pub fn coordinates_to_srgb(coordinates: [f64; 3], space: WorkingSpace) -> [f64; 3] {
    match space {
        WorkingSpace::Srgb => coordinates,
        WorkingSpace::LinearRgb => coordinates.map(encode_linear),
        WorkingSpace::Oklab => oklab_to_linear(coordinates).map(encode_linear),
        WorkingSpace::Oklch => {
            oklab_to_linear(cylindrical_to_cartesian(coordinates)).map(encode_linear)
        }
        WorkingSpace::Cielab => cielab_to_linear(coordinates).map(encode_linear),
        WorkingSpace::Cielch => {
            cielab_to_linear(cylindrical_to_cartesian(coordinates)).map(encode_linear)
        }
        WorkingSpace::Ycbcr => {
            let [y, cb, cr] = coordinates;
            [
                y + 1.402 * (cr - 0.5),
                y - (0.114 * 1.772 * (cb - 0.5) + 0.299 * 1.402 * (cr - 0.5)) / 0.587,
                y + 1.772 * (cb - 0.5),
            ]
        }
    }
}

fn encode_linear(channel: f64) -> f64 {
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

fn cylindrical_to_cartesian([lightness, chroma, hue]: [f64; 3]) -> [f64; 3] {
    if chroma <= 0.0 {
        return [lightness, 0.0, 0.0];
    }
    let hue = hue.rem_euclid(std::f64::consts::TAU);
    let hue = if hue == std::f64::consts::TAU {
        0.0
    } else {
        hue
    };
    [lightness, chroma * hue.cos(), chroma * hue.sin()]
}

fn oklab_to_linear([lightness, a, b]: [f64; 3]) -> [f64; 3] {
    let l_root = lightness + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_root = lightness - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_root = lightness - 0.089_484_18 * a - 1.291_485_5 * b;
    let l = l_root * l_root * l_root;
    let m = m_root * m_root * m_root;
    let s = s_root * s_root * s_root;
    [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ]
}

fn cielab_to_linear([lightness, a, b]: [f64; 3]) -> [f64; 3] {
    let fy = (lightness + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let x = 0.95047 * inverse_lab_f(fx);
    let y = inverse_lab_f(fy);
    let z = 1.08883 * inverse_lab_f(fz);
    [
        3.240_455 * x - 1.537_138_8 * y - 0.498_531_55 * z,
        -0.969_266_4 * x + 1.876_010_9 * y + 0.041_556_083 * z,
        0.055_643_42 * x - 0.204_025_85 * y + 1.057_225_1 * z,
    ]
}

fn inverse_lab_f(value: f64) -> f64 {
    const DELTA: f64 = 6.0 / 29.0;
    const KAPPA: f64 = 24_389.0 / 27.0;
    if value > DELTA {
        value * value * value
    } else {
        (116.0 * value - 16.0) / KAPPA
    }
}
