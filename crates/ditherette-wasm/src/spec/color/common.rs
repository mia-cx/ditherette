//! Shared scalar color-space formulas for spec color conversions.

pub const D65_XN: f32 = 0.95047;
pub const D65_YN: f32 = 1.0;
pub const D65_ZN: f32 = 1.08883;

pub fn srgb8_to_unit(channel: u8) -> f32 {
    channel as f32 / 255.0
}

pub fn srgb_unit_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb_unit(channel: f32) -> f32 {
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

pub fn srgb8_to_linear(channel: u8) -> f32 {
    srgb_unit_to_linear(srgb8_to_unit(channel))
}

pub fn linear_srgb_to_xyz(r: f32, g: f32, b: f32) -> [f32; 3] {
    [
        0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b,
        0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b,
        0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b,
    ]
}

pub fn xyz_to_cielab(x: f32, y: f32, z: f32) -> [f32; 3] {
    let fx = lab_f(x / D65_XN);
    let fy = lab_f(y / D65_YN);
    let fz = lab_f(z / D65_ZN);

    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn lab_f(t: f32) -> f32 {
    const EPSILON: f32 = 216.0 / 24_389.0;
    const KAPPA: f32 = 24_389.0 / 27.0;

    if t > EPSILON {
        t.cbrt()
    } else {
        (KAPPA * t + 16.0) / 116.0
    }
}

pub fn cartesian_to_cylindrical(lightness: f32, a: f32, b: f32) -> [f32; 3] {
    let chroma = (a * a + b * b).sqrt();
    let hue = b.atan2(a).rem_euclid(std::f32::consts::TAU);
    [lightness, chroma, hue]
}

pub fn srgb8_to_oklab(r: u8, g: u8, b: u8) -> [f32; 3] {
    linear_srgb_to_oklab(srgb8_to_linear(r), srgb8_to_linear(g), srgb8_to_linear(b))
}

pub fn linear_srgb_to_oklab(r: f32, g: f32, b: f32) -> [f32; 3] {
    let l = 0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    [
        0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_,
        1.977_998_5 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_,
        0.025_904_037 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_,
    ]
}

pub fn srgb8_to_cielab(r: u8, g: u8, b: u8) -> [f32; 3] {
    let [x, y, z] = linear_srgb_to_xyz(srgb8_to_linear(r), srgb8_to_linear(g), srgb8_to_linear(b));
    xyz_to_cielab(x, y, z)
}
