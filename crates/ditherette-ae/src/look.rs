//! Effect controls and their translation into a package recipe.
//!
//! Labels, defaults and the recipe mapping follow the website editor
//! (`src/routes/components/*-options.ts`, `src/lib/processing/package-adapter.ts`),
//! so a look made in the browser can be rebuilt in After Effects.

use ditherette_wasm::prod::contract::request::{
    Anchor, BayerSize, Diffusion, DiffusionFeedback, DitherPolicy, Field, MatchPolicy, OrderedTile,
    PerturbPolicy, Placement, ResizePolicy, Support, WorkingSpace,
};

/// One popup row. After Effects counts separators as rows.
pub enum Row<T> {
    Choice(&'static str, T),
    Separator,
}

/// The popup labels, with After Effects' separator marker.
pub fn labels<T>(rows: &[Row<T>]) -> Vec<&'static str> {
    rows.iter()
        .map(|row| match row {
            Row::Choice(label, _) => *label,
            Row::Separator => "(-",
        })
        .collect()
}

/// The choice at a 1-based popup value. Separators and stale values fall back to the first row.
pub fn choice<T: Copy>(rows: &[Row<T>], value: i32) -> T {
    let first = match rows[0] {
        Row::Choice(_, choice) => choice,
        Row::Separator => unreachable!("popups start with a choice"),
    };
    match usize::try_from(value - 1)
        .ok()
        .and_then(|index| rows.get(index))
    {
        Some(Row::Choice(_, choice)) => *choice,
        _ => first,
    }
}

/// 1-based popup value of `wanted`.
pub fn position<T: PartialEq>(rows: &[Row<T>], wanted: T) -> i32 {
    let index = rows
        .iter()
        .position(|row| matches!(row, Row::Choice(_, choice) if *choice == wanted))
        .expect("default is listed");
    index as i32 + 1
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dither {
    None,
    Field(Field),
    Diffusion(Diffusion),
    Yliluoma(BayerSize),
}

use Row::{Choice, Separator};

pub const DITHERS: &[Row<Dither>] = &[
    Choice("None", Dither::None),
    Separator,
    Choice(
        "Bayer 2×2",
        Dither::Field(Field::Bayer {
            size: BayerSize::Two,
        }),
    ),
    Choice(
        "Bayer 4×4",
        Dither::Field(Field::Bayer {
            size: BayerSize::Four,
        }),
    ),
    Choice(
        "Bayer 8×8",
        Dither::Field(Field::Bayer {
            size: BayerSize::Eight,
        }),
    ),
    Choice(
        "Bayer 16×16",
        Dither::Field(Field::Bayer {
            size: BayerSize::Sixteen,
        }),
    ),
    Choice(
        "Ordered 3×1",
        Dither::Field(Field::Ordered {
            tile: OrderedTile::ThreeByOne,
        }),
    ),
    Choice(
        "Ordered 4×1",
        Dither::Field(Field::Ordered {
            tile: OrderedTile::FourByOne,
        }),
    ),
    Choice(
        "Ordered 4×2",
        Dither::Field(Field::Ordered {
            tile: OrderedTile::FourByTwo,
        }),
    ),
    Choice(
        "Ordered 5×3",
        Dither::Field(Field::Ordered {
            tile: OrderedTile::FiveByThree,
        }),
    ),
    Separator,
    Choice(
        "Floyd–Steinberg",
        Dither::Diffusion(Diffusion::FloydSteinberg),
    ),
    Choice("Sierra", Dither::Diffusion(Diffusion::Sierra)),
    Choice("Sierra Lite", Dither::Diffusion(Diffusion::SierraLite)),
    Choice("Atkinson", Dither::Diffusion(Diffusion::Atkinson)),
    Choice(
        "Jarvis–Judice–Ninke",
        Dither::Diffusion(Diffusion::JarvisJudiceNinke),
    ),
    Choice("Stucki", Dither::Diffusion(Diffusion::Stucki)),
    Choice("Burkes", Dither::Diffusion(Diffusion::Burkes)),
    Choice("Two-row Sierra", Dither::Diffusion(Diffusion::TwoRowSierra)),
    Choice("Fan", Dither::Diffusion(Diffusion::Fan)),
    Choice("Shiau–Fan", Dither::Diffusion(Diffusion::ShiauFan)),
    Choice("Shiau–Fan 2", Dither::Diffusion(Diffusion::ShiauFan2)),
    Choice("Simple 2D", Dither::Diffusion(Diffusion::Simple2d)),
    Separator,
    Choice("Random", Dither::Field(Field::Random { seed: RANDOM_SEED })),
    Choice("Blue noise", Dither::Field(Field::BlueNoise {})),
    Separator,
    Choice("Yliluoma 2×2", Dither::Yliluoma(BayerSize::Two)),
    Choice("Yliluoma 4×4", Dither::Yliluoma(BayerSize::Four)),
    Choice("Yliluoma 8×8", Dither::Yliluoma(BayerSize::Eight)),
    Choice("Yliluoma 16×16", Dither::Yliluoma(BayerSize::Sixteen)),
];

/// The editor's default seed. A fixed seed keeps the noise still from frame to frame.
const RANDOM_SEED: u32 = 0xc0ff_ee42;

/// A colour space row: the matching metric and the space dithering may work in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Space {
    pub matching: MatchPolicy,
    pub working: WorkingSpace,
    /// Weighted RGB is a metric, not coordinates, so it cannot dither in its own space.
    pub dithers: bool,
}

const fn space(matching: MatchPolicy, working: WorkingSpace) -> Space {
    Space {
        matching,
        working,
        dithers: true,
    }
}

pub const SPACES: &[Row<Space>] = &[
    Choice(
        "OKLab",
        space(MatchPolicy::OklabEuclidean, WorkingSpace::Oklab),
    ),
    Choice(
        "sRGB",
        space(MatchPolicy::SrgbEuclidean, WorkingSpace::Srgb),
    ),
    Choice(
        "Linear RGB",
        space(MatchPolicy::LinearRgbEuclidean, WorkingSpace::LinearRgb),
    ),
    Choice(
        "Weighted RGB",
        Space {
            matching: MatchPolicy::SrgbCompuphase,
            working: WorkingSpace::Srgb,
            dithers: false,
        },
    ),
    Choice(
        "Weighted RGB · Rec.601",
        space(MatchPolicy::SrgbRec601, WorkingSpace::Srgb),
    ),
    Choice(
        "Weighted RGB · Rec.709",
        space(MatchPolicy::SrgbRec709, WorkingSpace::Srgb),
    ),
    Choice(
        "OKLCH",
        space(MatchPolicy::OklchHueArc, WorkingSpace::Oklch),
    ),
    Choice(
        "OKLCH · circular hue",
        space(MatchPolicy::OklchCircularHue, WorkingSpace::Oklch),
    ),
    Choice(
        "OKLCH · Euclidean",
        space(MatchPolicy::OklchEuclidean, WorkingSpace::Oklch),
    ),
    Choice(
        "CIELAB ΔE76",
        space(MatchPolicy::CielabEuclidean, WorkingSpace::Cielab),
    ),
    Choice(
        "CIELAB ΔE2000",
        space(MatchPolicy::CielabCiede2000, WorkingSpace::Cielab),
    ),
    Choice(
        "CIELCh",
        space(MatchPolicy::CielchHueArc, WorkingSpace::Cielch),
    ),
    Choice(
        "CIELCh · circular hue",
        space(MatchPolicy::CielchCircularHue, WorkingSpace::Cielch),
    ),
    Choice(
        "CIELCh · Euclidean",
        space(MatchPolicy::CielchEuclidean, WorkingSpace::Cielch),
    ),
    Choice(
        "YCbCr",
        space(MatchPolicy::YcbcrEuclidean, WorkingSpace::Ycbcr),
    ),
];

/// Every resize is centred, as in the editor's default anchor.
const CENTER: Anchor = Anchor::Center;

pub const RESIZES: &[Row<ResizePolicy>] = &[
    Choice("Nearest", ResizePolicy::Nearest { anchor: CENTER }),
    Choice("Bilinear", ResizePolicy::Bilinear { anchor: CENTER }),
    Choice("Trilinear", ResizePolicy::Trilinear { anchor: CENTER }),
    Choice(
        "Bicubic",
        ResizePolicy::Bicubic {
            anchor: CENTER,
            support: Support::Fixed,
        },
    ),
    Choice(
        "Bicubic AA (slow)",
        ResizePolicy::Bicubic {
            anchor: CENTER,
            support: Support::ScaleAware,
        },
    ),
    Choice(
        "Lanczos2",
        ResizePolicy::Lanczos2 {
            anchor: CENTER,
            support: Support::Fixed,
        },
    ),
    Choice(
        "Lanczos2 AA (slow)",
        ResizePolicy::Lanczos2 {
            anchor: CENTER,
            support: Support::ScaleAware,
        },
    ),
    Choice(
        "Lanczos3",
        ResizePolicy::Lanczos3 {
            anchor: CENTER,
            support: Support::Fixed,
        },
    ),
    Choice(
        "Lanczos3 AA (slow)",
        ResizePolicy::Lanczos3 {
            anchor: CENTER,
            support: Support::ScaleAware,
        },
    ),
    Choice("Area / Box", ResizePolicy::Area {}),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementMode {
    Everywhere,
    Adaptive,
}

pub const PLACEMENTS: &[Row<PlacementMode>] = &[
    Choice("Everywhere", PlacementMode::Everywhere),
    Choice("Adaptive", PlacementMode::Adaptive),
];

/// Editor control ranges and defaults.
pub mod defaults {
    use super::*;

    pub const PIXEL_SIZE: i32 = 1;
    pub const MAX_PIXEL_SIZE: i32 = 64;
    pub const RESIZE: ResizePolicy = ResizePolicy::Bilinear { anchor: CENTER };
    pub const DITHER: Dither = Dither::None;
    pub const STRENGTH: f64 = 100.0;
    pub const SERPENTINE: bool = true;
    pub const DITHER_IN_SPACE: bool = false;
    pub const PLACEMENT: PlacementMode = PlacementMode::Everywhere;
    pub const SAMPLE_SIZE: i32 = 64;
    pub const MIN_SAMPLE_SIZE: i32 = 4;
    /// The editor caps Adaptive samples; the package accepts more.
    pub const MAX_SAMPLE_SIZE: i32 = 100;
    pub const CONTRAST_TOLERANCE: f64 = 12.0;
}

/// Every control value, already read from the effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Output pixel edge in layer pixels.
    pub pixel_size: u32,
    pub resize: ResizePolicy,
    pub dither: Dither,
    /// Percent.
    pub strength: f32,
    pub serpentine: bool,
    pub space: Space,
    pub dither_in_space: bool,
    pub placement: PlacementMode,
    pub sample_size: u32,
    /// Percent.
    pub contrast_tolerance: f32,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            pixel_size: defaults::PIXEL_SIZE as u32,
            resize: defaults::RESIZE,
            dither: defaults::DITHER,
            strength: defaults::STRENGTH as f32,
            serpentine: defaults::SERPENTINE,
            space: choice(SPACES, 1),
            dither_in_space: defaults::DITHER_IN_SPACE,
            placement: defaults::PLACEMENT,
            sample_size: defaults::SAMPLE_SIZE as u32,
            contrast_tolerance: defaults::CONTRAST_TOLERANCE as f32,
        }
    }
}

/// Matches the editor's byte noise scale to the package's normalized field (255 / 4).
const BYTE_FIELD_STRENGTH_RATIO: f32 = 96.0 / 63.75;

impl Look {
    /// The package dither policy, exactly as the editor builds it.
    pub fn dither_policy(&self) -> DitherPolicy {
        let placement = match self.placement {
            PlacementMode::Everywhere => Placement::Everywhere {},
            PlacementMode::Adaptive => Placement::SmoothAreaV1 {
                sample_size: self.sample_size.clamp(
                    defaults::MIN_SAMPLE_SIZE as u32,
                    defaults::MAX_SAMPLE_SIZE as u32,
                ),
                contrast_threshold: (self.contrast_tolerance / 100.0).clamp(0.0, 1.0),
            },
        };
        // Yliluoma places whole palette colours, so it has no strength to turn down.
        if let Dither::Yliluoma(size) = self.dither {
            return DitherPolicy::Yliluoma { size, placement };
        }
        let strength = self.strength / 100.0;
        if strength == 0.0 {
            return DitherPolicy::None {};
        }
        let vector = self.dither_in_space && self.space.dithers;
        match self.dither {
            Dither::None | Dither::Yliluoma(_) => DitherPolicy::None {},
            Dither::Diffusion(kernel) => DitherPolicy::Diffusion {
                kernel,
                strength,
                placement,
                serpentine: self.serpentine,
                feedback: if vector {
                    DiffusionFeedback::Matching
                } else {
                    DiffusionFeedback::SrgbBytes
                },
            },
            Dither::Field(field) => DitherPolicy::Separable {
                perturb: PerturbPolicy {
                    field,
                    space: if vector {
                        self.space.working
                    } else {
                        WorkingSpace::Srgb
                    },
                    strength: if vector {
                        strength
                    } else {
                        strength * BYTE_FIELD_STRENGTH_RATIO
                    },
                    placement,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_values_skip_separators() {
        assert_eq!(choice(DITHERS, 1), Dither::None);
        assert_eq!(
            choice(DITHERS, 3),
            Dither::Field(Field::Bayer {
                size: BayerSize::Two
            })
        );
        assert_eq!(
            choice(DITHERS, 2),
            Dither::None,
            "a separator is never a choice"
        );
        assert_eq!(
            choice(DITHERS, 99),
            Dither::None,
            "a stale value falls back"
        );
        assert_eq!(
            choice(
                DITHERS,
                position(DITHERS, Dither::Diffusion(Diffusion::Atkinson))
            ),
            Dither::Diffusion(Diffusion::Atkinson)
        );
        assert_eq!(labels(DITHERS)[1], "(-");
    }
}
