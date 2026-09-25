//! Continuous conversions between the encoded sRGB carrier and working spaces.
//!
//! Unlike `spec::color`, these accept and return unclipped `f32` values, so an
//! effect can leave the sRGB gamut and a later effect still sees the result.

use crate::spec::color::{linear_to_srgb_unit, srgb_unit_to_linear};

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
