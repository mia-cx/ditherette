//! Step masks: curves whose output is a step's strength per pixel.
//!
//! Prepared masks are fixed-size and borrow grid values by curve index, so evaluating one
//! inside the colour memo never allocates.

use serde::{Deserialize, Serialize};

use crate::prod::contract::error::{DitheretteError, ErrorCode};

use super::{
    chain::StackPath,
    curves::{
        validate_channel, validate_periodic_points, ChannelKind, ColourChannel, CurveGrid,
        CurveSpline, Curves, ResolvedChannel,
    },
};

/// Most curves one step's mask holds.
pub const MAX_MASK_CURVES: usize = 4;

/// Strength by one input channel. A hue input uses the cyclic spline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OneInputMask {
    pub x: ColourChannel,
    pub points: Vec<[f32; 2]>,
}

/// Strength by two input channels through a control grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TwoInputMask {
    pub x: ColourChannel,
    pub x2: ColourChannel,
    pub grid: CurveGrid,
}

/// A strict one-input or two-input mask curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MaskCurve {
    OneInput(OneInputMask),
    TwoInput(TwoInputMask),
}

/// Checks the count, then each curve, under `{step}.mask`. Never allocates on success.
pub fn validate(mask: &[MaskCurve], step: &str) -> Result<(), DitheretteError> {
    if mask.len() > MAX_MASK_CURVES {
        return Err(DitheretteError::new(
            ErrorCode::InvalidSettings,
            StackPath::new(format_args!("{step}.mask")).as_str(),
            format!("Expected 0 to {MAX_MASK_CURVES} mask curves."),
        ));
    }
    for (index, curve) in mask.iter().enumerate() {
        curve.validate(StackPath::new(format_args!("{step}.mask.{index}")).as_str())?;
    }
    Ok(())
}

impl MaskCurve {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        match self {
            Self::OneInput(curve) => {
                let x =
                    validate_channel(curve.x, StackPath::new(format_args!("{path}.x")).as_str())?;
                let points = StackPath::new(format_args!("{path}.points"));
                Curves::validate_points(&curve.points, points.as_str())?;
                if x.kind == ChannelKind::Hue {
                    validate_periodic_points(&curve.points, points.as_str())?;
                }
                Ok(())
            }
            Self::TwoInput(curve) => {
                let x =
                    validate_channel(curve.x, StackPath::new(format_args!("{path}.x")).as_str())?;
                let x2 =
                    validate_channel(curve.x2, StackPath::new(format_args!("{path}.x2")).as_str())?;
                if curve.x2 == curve.x {
                    return Err(DitheretteError::new(
                        ErrorCode::InvalidSettings,
                        StackPath::new(format_args!("{path}.x2")).as_str(),
                        "A two-input curve must use two different input channels.",
                    ));
                }
                curve.grid.validate(
                    StackPath::new(format_args!("{path}.grid")).as_str(),
                    x.kind == ChannelKind::Hue,
                    x2.kind == ChannelKind::Hue,
                )
            }
        }
    }

    /// Every value is exactly 1, so the curve multiplies strength by exactly 1 everywhere.
    fn is_full(&self) -> bool {
        match self {
            Self::OneInput(curve) => curve.points.iter().all(|point| point[1] == 1.0),
            Self::TwoInput(curve) => curve
                .grid
                .values
                .iter()
                .flatten()
                .all(|&value| value == 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PreparedCurve {
    OneInput {
        x: ResolvedChannel,
        spline: CurveSpline,
    },
    TwoInput {
        curve_index: usize,
        x: ResolvedChannel,
        x2: ResolvedChannel,
    },
}

/// A validated mask's resolved curves and inline splines. Full-strength curves are dropped:
/// multiplying by exactly 1 changes nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedMask {
    curves: [Option<PreparedCurve>; MAX_MASK_CURVES],
    len: usize,
}

impl PreparedMask {
    pub fn new(mask: &[MaskCurve]) -> Self {
        let mut prepared = Self {
            curves: [None; MAX_MASK_CURVES],
            len: 0,
        };
        for (curve_index, curve) in mask.iter().enumerate() {
            if curve.is_full() {
                continue;
            }
            let entry = match curve {
                MaskCurve::OneInput(curve) => {
                    let x = curve.x.resolve().expect("validated mask x channel");
                    PreparedCurve::OneInput {
                        x,
                        spline: CurveSpline::new(&curve.points, x.kind == ChannelKind::Hue),
                    }
                }
                MaskCurve::TwoInput(curve) => PreparedCurve::TwoInput {
                    curve_index,
                    x: curve.x.resolve().expect("validated mask x channel"),
                    x2: curve.x2.resolve().expect("validated mask x2 channel"),
                },
            };
            prepared.curves[prepared.len] = Some(entry);
            prepared.len += 1;
        }
        prepared
    }

    /// True when the mask is 1 everywhere, so the step runs unmasked.
    pub fn is_full(&self) -> bool {
        self.len == 0
    }

    /// The pixel's strength: 1, times each remaining curve's value in list order.
    /// `mask` is the list this state was prepared from.
    pub fn strength(&self, mask: &[MaskCurve], rgb: [f32; 3]) -> f32 {
        let mut strength = 1.0;
        for curve in self.curves[..self.len].iter().flatten() {
            strength *= match *curve {
                PreparedCurve::OneInput { x, spline } => {
                    let coordinates = x.model.to_normalized(rgb);
                    let value = spline.eval(coordinates[x.index]).clamp(0.0, 1.0);
                    if x.kind == ChannelKind::Hue {
                        fade(value, x.model.hue_weight(rgb, coordinates))
                    } else {
                        value
                    }
                }
                PreparedCurve::TwoInput { curve_index, x, x2 } => {
                    let MaskCurve::TwoInput(curve) = &mask[curve_index] else {
                        unreachable!("prepared grid index resolved to a one-input mask curve")
                    };
                    let x_coordinates = x.model.to_normalized(rgb);
                    let x2_coordinates = if x2.model == x.model {
                        x_coordinates
                    } else {
                        x2.model.to_normalized(rgb)
                    };
                    let value = curve.grid.eval(
                        x_coordinates[x.index],
                        x2_coordinates[x2.index],
                        x.kind == ChannelKind::Hue,
                        x2.kind == ChannelKind::Hue,
                    );
                    let mut weight: f32 = 1.0;
                    if x.kind == ChannelKind::Hue {
                        weight = x.model.hue_weight(rgb, x_coordinates);
                    }
                    if x2.kind == ChannelKind::Hue {
                        weight = weight.min(x2.model.hue_weight(rgb, x2_coordinates));
                    }
                    fade(value, weight)
                }
            };
        }
        strength
    }
}

/// Near-grey hue is unreliable, so a hue-keyed value moves toward 1 as confidence falls.
fn fade(value: f32, weight: f32) -> f32 {
    if weight == 1.0 {
        value
    } else {
        1.0 - weight * (1.0 - value)
    }
}

/// Each pixel's strength for a validated mask, as the reference computes it.
pub fn strengths(mask: &[MaskCurve], rgb: &[[f32; 3]]) -> Vec<f32> {
    let prepared = PreparedMask::new(mask);
    rgb.iter()
        .map(|&rgb| prepared.strength(mask, rgb))
        .collect()
}

/// Moves `output` back toward `input` by `1 - strength`, per channel in carrier units.
/// Strength 1 returns `output` and strength 0 returns `input`, both exactly.
pub fn blend(input: [f32; 3], output: [f32; 3], strength: f32) -> [f32; 3] {
    if strength == 1.0 {
        return output;
    }
    if strength == 0.0 {
        return input;
    }
    std::array::from_fn(|channel| input[channel] + strength * (output[channel] - input[channel]))
}
