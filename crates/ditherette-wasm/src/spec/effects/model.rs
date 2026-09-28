//! Shared colour-model conversion and normalized channel coordinates.

use serde::{Deserialize, Serialize};

use super::space::{
    cielab_to_linear, from_linear, linear_to_cielab, linear_to_oklab, oklab_to_linear, to_linear,
};

/// Every colour model understood by colour-channel effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColourModel {
    Srgb,
    LinearRgb,
    Hsl,
    Hsv,
    Oklab,
    Oklch,
    Cielab,
    Cielch,
    Ycbcr,
}

impl ColourModel {
    /// Converts carrier RGB to the model's three normalized editing coordinates.
    pub fn to_normalized(self, rgb: [f32; 3]) -> [f32; 3] {
        match self {
            Self::Srgb => rgb,
            Self::LinearRgb => to_linear(rgb),
            Self::Hsl => rgb_to_hsl(rgb.map(|value| value.clamp(0.0, 1.0))),
            Self::Hsv => rgb_to_hsv(rgb.map(|value| value.clamp(0.0, 1.0))),
            Self::Oklab => {
                let [l, a, b] = linear_to_oklab(to_linear(rgb));
                [l, a / 0.8 + 0.5, b / 0.8 + 0.5]
            }
            Self::Oklch => {
                let [l, a, b] = linear_to_oklab(to_linear(rgb));
                [l, a.hypot(b) / 0.4, hue(a, b)]
            }
            Self::Cielab => {
                let [l, a, b] = linear_to_cielab(to_linear(rgb));
                [l / 100.0, a / 250.0 + 0.5, b / 250.0 + 0.5]
            }
            Self::Cielch => {
                let [l, a, b] = linear_to_cielab(to_linear(rgb));
                [l / 100.0, a.hypot(b) / 150.0, hue(a, b)]
            }
            Self::Ycbcr => {
                let [y, cb, cr] = rgb_to_ycbcr(rgb);
                [y, cb + 0.5, cr + 0.5]
            }
        }
    }

    /// Converts normalized editing coordinates back to unclipped carrier RGB.
    pub fn from_normalized(self, coordinates: [f32; 3]) -> [f32; 3] {
        match self {
            Self::Srgb => coordinates,
            Self::LinearRgb => from_linear(coordinates),
            Self::Hsl => hsl_to_rgb(coordinates),
            Self::Hsv => hsv_to_rgb(coordinates),
            Self::Oklab => {
                let [l, a, b] = coordinates;
                from_linear(oklab_to_linear([l, (a - 0.5) * 0.8, (b - 0.5) * 0.8]))
            }
            Self::Oklch => {
                let [l, c, h] = coordinates;
                let angle = h.rem_euclid(1.0) * std::f32::consts::TAU;
                from_linear(oklab_to_linear([
                    l,
                    c * 0.4 * angle.cos(),
                    c * 0.4 * angle.sin(),
                ]))
            }
            Self::Cielab => {
                let [l, a, b] = coordinates;
                from_linear(cielab_to_linear([
                    l * 100.0,
                    (a - 0.5) * 250.0,
                    (b - 0.5) * 250.0,
                ]))
            }
            Self::Cielch => {
                let [l, c, h] = coordinates;
                let angle = h.rem_euclid(1.0) * std::f32::consts::TAU;
                from_linear(cielab_to_linear([
                    l * 100.0,
                    c * 150.0 * angle.cos(),
                    c * 150.0 * angle.sin(),
                ]))
            }
            Self::Ycbcr => {
                let [y, cb, cr] = coordinates;
                ycbcr_to_rgb([y, cb - 0.5, cr - 0.5])
            }
        }
    }

    /// Index of the circular hue channel, when this model has one.
    pub const fn hue_channel(self) -> Option<usize> {
        match self {
            Self::Hsl | Self::Hsv => Some(0),
            Self::Oklch | Self::Cielch => Some(2),
            _ => None,
        }
    }

    /// Confidence in the model's hue coordinate. Exact greys have zero confidence.
    pub fn hue_weight(self, rgb: [f32; 3], normalized: [f32; 3]) -> f32 {
        // Float conversions leave exact greys a residual chroma, but they have no hue.
        if rgb[0] == rgb[1] && rgb[1] == rgb[2] {
            return 0.0;
        }
        match self {
            Self::Hsl | Self::Hsv => {
                let rgb = rgb.map(|value| value.clamp(0.0, 1.0));
                let maximum = rgb[0].max(rgb[1]).max(rgb[2]);
                let minimum = rgb[0].min(rgb[1]).min(rgb[2]);
                ((maximum - minimum) / 0.02).clamp(0.0, 1.0)
            }
            Self::Oklch => (normalized[1] * 0.4 / 0.02).clamp(0.0, 1.0),
            Self::Cielch => (normalized[1] * 150.0 / 2.0).clamp(0.0, 1.0),
            _ => 1.0,
        }
    }
}

fn hue(a: f32, b: f32) -> f32 {
    if a == 0.0 && b == 0.0 {
        0.0
    } else {
        b.atan2(a).to_degrees().rem_euclid(360.0) / 360.0
    }
}

fn rgb_hue([r, g, b]: [f32; 3], maximum: f32, chroma: f32) -> f32 {
    if chroma == 0.0 {
        return 0.0;
    }
    let sector = if maximum == r {
        ((g - b) / chroma).rem_euclid(6.0)
    } else if maximum == g {
        (b - r) / chroma + 2.0
    } else {
        (r - g) / chroma + 4.0
    };
    sector / 6.0
}

fn rgb_to_hsl(rgb: [f32; 3]) -> [f32; 3] {
    let maximum = rgb[0].max(rgb[1]).max(rgb[2]);
    let minimum = rgb[0].min(rgb[1]).min(rgb[2]);
    let chroma = maximum - minimum;
    let lightness = (maximum + minimum) / 2.0;
    let saturation = if chroma == 0.0 {
        0.0
    } else {
        chroma / (1.0 - (2.0 * lightness - 1.0).abs())
    };
    [rgb_hue(rgb, maximum, chroma), saturation, lightness]
}

fn rgb_to_hsv(rgb: [f32; 3]) -> [f32; 3] {
    let maximum = rgb[0].max(rgb[1]).max(rgb[2]);
    let minimum = rgb[0].min(rgb[1]).min(rgb[2]);
    let chroma = maximum - minimum;
    let saturation = if chroma == 0.0 { 0.0 } else { chroma / maximum };
    [rgb_hue(rgb, maximum, chroma), saturation, maximum]
}

fn hue_rgb(hue: f32, chroma: f32, minimum: f32) -> [f32; 3] {
    let sector = hue.rem_euclid(1.0) * 6.0;
    let x = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let rgb = match sector as u32 {
        0 => [chroma, x, 0.0],
        1 => [x, chroma, 0.0],
        2 => [0.0, chroma, x],
        3 => [0.0, x, chroma],
        4 => [x, 0.0, chroma],
        _ => [chroma, 0.0, x],
    };
    rgb.map(|value| value + minimum)
}

fn hsl_to_rgb([hue, saturation, lightness]: [f32; 3]) -> [f32; 3] {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    hue_rgb(hue, chroma, lightness - chroma / 2.0)
}

fn hsv_to_rgb([hue, saturation, value]: [f32; 3]) -> [f32; 3] {
    let chroma = value * saturation;
    hue_rgb(hue, chroma, value - chroma)
}

fn rgb_to_ycbcr([r, g, b]: [f32; 3]) -> [f32; 3] {
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    [y, (b - y) / 1.772, (r - y) / 1.402]
}

fn ycbcr_to_rgb([y, cb, cr]: [f32; 3]) -> [f32; 3] {
    let r = y + 1.402 * cr;
    let b = y + 1.772 * cb;
    let g = (y - 0.299 * r - 0.114 * b) / 0.587;
    [r, g, b]
}
