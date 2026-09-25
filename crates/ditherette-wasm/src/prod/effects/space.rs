//! Continuous conversions between the encoded sRGB carrier and working spaces.
//!
//! Unlike `prod::color`, these accept and return unclipped `f32` values, so an
//! effect can leave the sRGB gamut and a later effect still sees the result.

use crate::prod::contract::request::WorkingSpace;

/// The sRGB decode curve, extended to any finite value.
pub fn srgb_unit_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// The sRGB encode curve, extended to any finite value.
pub fn linear_to_srgb_unit(channel: f32) -> f32 {
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

/// Decodes carrier RGB with the piecewise sRGB curve, extended to any finite value.
pub fn to_linear(rgb: [f32; 3]) -> [f32; 3] {
    rgb.map(srgb_unit_to_linear)
}

/// Encodes linear-light RGB with the same extended piecewise curve.
pub fn from_linear(linear: [f32; 3]) -> [f32; 3] {
    linear.map(linear_to_srgb_unit)
}

/// Linear sRGB to Oklab with Ottosson's f32 matrices and signed cube roots.
pub fn linear_to_oklab([r, g, b]: [f32; 3]) -> [f32; 3] {
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

/// Oklab to linear sRGB, the exact inverse recipe of `spec::color::oklab` without clipping.
pub fn oklab_to_linear([lightness, a, b]: [f32; 3]) -> [f32; 3] {
    let l = lightness + 0.396_337_78 * a + 0.215_803_76 * b;
    let m = lightness - 0.105_561_346 * a - 0.063_854_17 * b;
    let s = lightness - 0.089_484_18 * a - 1.291_485_5 * b;
    let [l, m, s] = [l * l * l, m * m * m, s * s * s];
    [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ]
}

/// Linear sRGB to D65 CIELAB, with the same matrix and `f` as `spec::color::cielab`.
pub fn linear_to_cielab([r, g, b]: [f32; 3]) -> [f32; 3] {
    let x = 0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b;
    let z = 0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b;
    let [fx, fy, fz] = [x / D65_XN, y / D65_YN, z / D65_ZN].map(lab_f);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// D65 CIELAB to linear sRGB, the inverse recipe of `spec::color::cielab` without clipping.
pub fn cielab_to_linear([lightness, a, b]: [f32; 3]) -> [f32; 3] {
    let fy = (lightness + 16.0) / 116.0;
    let [x, y, z] = [fy + a / 500.0, fy, fy - b / 200.0].map(inverse_lab_f);
    let [x, y, z] = [D65_XN * x, D65_YN * y, D65_ZN * z];
    [
        3.240_455 * x - 1.537_138_8 * y - 0.498_531_55 * z,
        -0.969_266_4 * x + 1.876_010_9 * y + 0.041_556_083 * z,
        0.055_643_42 * x - 0.204_025_85 * y + 1.057_225_1 * z,
    ]
}

const D65_XN: f32 = 0.95047;
const D65_YN: f32 = 1.0;
const D65_ZN: f32 = 1.08883;

fn lab_f(t: f32) -> f32 {
    const EPSILON: f32 = 216.0 / 24_389.0;
    const KAPPA: f32 = 24_389.0 / 27.0;
    if t > EPSILON {
        t.cbrt()
    } else {
        (KAPPA * t + 16.0) / 116.0
    }
}

fn inverse_lab_f(value: f32) -> f32 {
    const DELTA: f32 = 6.0 / 29.0;
    const KAPPA: f32 = 24_389.0 / 27.0;
    if value > DELTA {
        value * value * value
    } else {
        (116.0 * value - 16.0) / KAPPA
    }
}

/// Luma and centred chroma: `Y = kr R + (1 - kr - kb) G + kb B`, `u = (B - Y) / (2 - 2kb)`,
/// `v = (R - Y) / (2 - 2kr)`. BT.601 on encoded sRGB, BT.709 on linear light.
fn to_luma_chroma([r, g, b]: [f32; 3], kr: f32, kb: f32) -> [f32; 3] {
    let y = kr * r + (1.0 - kr - kb) * g + kb * b;
    [y, (b - y) / (2.0 - 2.0 * kb), (r - y) / (2.0 - 2.0 * kr)]
}

fn from_luma_chroma([y, u, v]: [f32; 3], kr: f32, kb: f32) -> [f32; 3] {
    let r = y + (2.0 - 2.0 * kr) * v;
    let b = y + (2.0 - 2.0 * kb) * u;
    let g = (y - kr * r - kb * b) / (1.0 - kr - kb);
    [r, g, b]
}

/// Lightness plus two opponent axes of carrier RGB in `space`, with lightness near `[0, 1]`
/// and neutral colours at `u = v = 0`. Cylindrical spaces use their cartesian form.
pub fn to_opponent(rgb: [f32; 3], space: WorkingSpace) -> [f32; 3] {
    match space {
        WorkingSpace::Srgb | WorkingSpace::Ycbcr => to_luma_chroma(rgb, 0.299, 0.114),
        WorkingSpace::LinearRgb => to_luma_chroma(to_linear(rgb), 0.2126, 0.0722),
        WorkingSpace::Oklab | WorkingSpace::Oklch => linear_to_oklab(to_linear(rgb)),
        WorkingSpace::Cielab | WorkingSpace::Cielch => {
            linear_to_cielab(to_linear(rgb)).map(|coordinate| coordinate / 100.0)
        }
    }
}

/// The exact inverse formulas of `to_opponent`, back to unclipped carrier RGB.
pub fn from_opponent(opponent: [f32; 3], space: WorkingSpace) -> [f32; 3] {
    match space {
        WorkingSpace::Srgb | WorkingSpace::Ycbcr => from_luma_chroma(opponent, 0.299, 0.114),
        WorkingSpace::LinearRgb => from_linear(from_luma_chroma(opponent, 0.2126, 0.0722)),
        WorkingSpace::Oklab | WorkingSpace::Oklch => from_linear(oklab_to_linear(opponent)),
        WorkingSpace::Cielab | WorkingSpace::Cielch => from_linear(cielab_to_linear(
            opponent.map(|coordinate| coordinate * 100.0),
        )),
    }
}
