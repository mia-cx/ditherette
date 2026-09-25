//! Channel selection for effects that map each RGB channel independently.

use serde::{Deserialize, Serialize};

/// Which encoded sRGB channels a per-channel effect changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Channel {
    Rgb,
    Red,
    Green,
    Blue,
}

impl Channel {
    /// Applies `map` to the selected channels and leaves the others untouched.
    pub fn apply(self, rgb: &mut [f32; 3], map: impl Fn(f32) -> f32) {
        match self {
            Self::Rgb => *rgb = rgb.map(map),
            Self::Red => rgb[0] = map(rgb[0]),
            Self::Green => rgb[1] = map(rgb[1]),
            Self::Blue => rgb[2] = map(rgb[2]),
        }
    }
}

impl Channel {
    /// Whether RGB channel `index` (0 red, 1 green, 2 blue) is changed.
    pub const fn selects(self, index: usize) -> bool {
        match self {
            Self::Rgb => true,
            Self::Red => index == 0,
            Self::Green => index == 1,
            Self::Blue => index == 2,
        }
    }
}
