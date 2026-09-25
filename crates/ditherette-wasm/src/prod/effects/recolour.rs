//! Palette-aware recolouring: fit continuous colour to what a palette can represent.
//!
//! A recipe is plain data: a lightness curve, an opponent shift and scale, and
//! hue-group adjustments, all in the quantization working space. A step without
//! a recipe analyses the image reaching it (`recolour_analysis`).

use std::f32::consts::PI;

use serde::{Deserialize, Serialize};

use crate::prod::contract::{
    error::{DitheretteError, ErrorCode},
    request::WorkingSpace,
};

use super::{
    chain::{check_bounded, Effect, EffectContext, Needs},
    curves::{Curves, Spline},
    image::EffectImage,
    recolour_analysis::analyze,
    space::{from_opponent, to_opponent},
};

/// Most hue groups one recipe may hold.
pub const MAX_GROUPS: usize = 12;
/// Opponent chroma below which hue is too unstable to target. Group weights fade in up to it.
pub const NEUTRAL_CHROMA: f32 = 0.02;
/// Largest accepted opponent shift on either axis.
pub const MAX_SHIFT: f32 = 0.5;

/// Adjusts colours near one hue. Weight falls from 1 at `hue` to 0 at `width` degrees away.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// Centre, degrees in `[0, 360]` in the working space's opponent plane.
    pub hue: f32,
    /// Half-width in degrees, `[1, 180]`.
    pub width: f32,
    /// Hue turn in degrees at full weight, `[-180, 180]`.
    pub turn: f32,
    /// Chroma scale at full weight, `[0, 2]`.
    pub chroma: f32,
}

/// An inspectable, editable recolouring treatment in one working space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecolourRecipe {
    /// The space it was analysed in. Applying it needs the same context space.
    pub space: WorkingSpace,
    /// Lightness curve points `[in, out]`, as for `curves`.
    pub tone: Vec<[f32; 2]>,
    /// Scale for both opponent axes after `shift`, `[0, 2]`.
    pub chroma: f32,
    /// Offset added to the opponent axes first, each in `[-0.5, 0.5]`.
    pub shift: [f32; 2],
    /// Hue-targeted adjustments, applied after `chroma`.
    pub groups: Vec<Group>,
}

impl RecolourRecipe {
    /// Leaves every colour where it is.
    pub fn identity(space: WorkingSpace) -> Self {
        Self {
            space,
            tone: vec![[0.0, 0.0], [1.0, 1.0]],
            chroma: 1.0,
            shift: [0.0, 0.0],
            groups: Vec::new(),
        }
    }

    /// True for the identity recipe, which application skips rather than round-trip.
    pub fn is_identity(&self) -> bool {
        *self == Self::identity(self.space)
    }

    /// Checks every field against its documented domain, naming the failing field.
    pub fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        Curves::validate_points(&self.tone, &format!("{path}.tone"))?;
        check_bounded(self.chroma, 0.0, 2.0, format!("{path}.chroma"))?;
        for (axis, value) in self.shift.iter().enumerate() {
            check_bounded(
                *value,
                -MAX_SHIFT,
                MAX_SHIFT,
                format!("{path}.shift.{axis}"),
            )?;
        }
        if self.groups.len() > MAX_GROUPS {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                format!("{path}.groups"),
                format!("A recipe holds at most {MAX_GROUPS} groups."),
            ));
        }
        for (index, group) in self.groups.iter().enumerate() {
            let at = format!("{path}.groups.{index}");
            check_bounded(group.hue, 0.0, 360.0, format!("{at}.hue"))?;
            check_bounded(group.width, 1.0, 180.0, format!("{at}.width"))?;
            check_bounded(group.turn, -180.0, 180.0, format!("{at}.turn"))?;
            check_bounded(group.chroma, 0.0, 2.0, format!("{at}.chroma"))?;
        }
        Ok(())
    }

    /// Applies the full recipe, then blends with the input by `strength`.
    pub fn apply(&self, image: &mut EffectImage, strength: f32) {
        if strength == 0.0 || self.is_identity() {
            return;
        }
        let tone = Spline::new(&self.tone);
        for rgb in &mut image.rgb {
            let adjusted = self.map(&tone, *rgb);
            *rgb = if strength == 1.0 {
                adjusted
            } else {
                std::array::from_fn(|channel| {
                    rgb[channel] + strength * (adjusted[channel] - rgb[channel])
                })
            };
        }
    }

    /// One carrier pixel through tone, shift, chroma, and groups, in the recipe's space.
    pub fn map(&self, tone: &Spline, rgb: [f32; 3]) -> [f32; 3] {
        let [lightness, u, v] = to_opponent(rgb, self.space);
        let lightness = tone.eval(lightness);
        let u = (u + self.shift[0]) * self.chroma;
        let v = (v + self.shift[1]) * self.chroma;
        let [u, v] = self.adjust_groups(u, v);
        from_opponent([lightness, u, v], self.space)
    }

    /// Sums every group's weighted turn and chroma change, then rotates and scales once.
    /// Near-neutral colours fade out of every group; the scale never goes below 0.
    fn adjust_groups(&self, u: f32, v: f32) -> [f32; 2] {
        let ramp = (u.hypot(v) / NEUTRAL_CHROMA).min(1.0);
        if self.groups.is_empty() || ramp == 0.0 {
            return [u, v];
        }
        let hue = v.atan2(u).to_degrees().rem_euclid(360.0);
        let mut turn = 0.0;
        let mut scale = 1.0;
        for group in &self.groups {
            let weight = window(hue, group) * ramp;
            turn += weight * group.turn;
            scale += weight * (group.chroma - 1.0);
        }
        let scale = scale.max(0.0);
        let (sin, cos) = turn.to_radians().sin_cos();
        [(u * cos - v * sin) * scale, (u * sin + v * cos) * scale]
    }
}

/// Raised cosine of the circular distance from the group centre: 1 there, 0 at `width` and beyond.
/// Groups spaced `width` apart sum to exactly 1 between their centres.
pub fn window(hue: f32, group: &Group) -> f32 {
    let distance = ((hue - group.hue + 180.0).rem_euclid(360.0) - 180.0).abs();
    if distance >= group.width {
        return 0.0;
    }
    0.5 * (1.0 + (PI * distance / group.width).cos())
}

/// The `recolour` effect. With no recipe it analyses the image it receives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recolour {
    /// Blend from the input (0) to the complete recipe (1).
    pub strength: f32,
    pub recipe: Option<RecolourRecipe>,
}

impl Effect for Recolour {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        check_bounded(self.strength, 0.0, 1.0, format!("{path}.strength"))?;
        match &self.recipe {
            Some(recipe) => recipe.validate(&format!("{path}.recipe")),
            None => Ok(()),
        }
    }

    fn needs(&self) -> Needs {
        Needs {
            palette: self.recipe.is_none(),
            space: true,
        }
    }

    fn check_context(
        &self,
        context: &EffectContext<'_>,
        path: &str,
    ) -> Result<(), DitheretteError> {
        match &self.recipe {
            Some(recipe) if Some(recipe.space) != context.space => Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                format!("{path}.recipe.space"),
                "The recipe was analysed in a different working space than this context.",
            )),
            _ => Ok(()),
        }
    }

    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>) {
        if self.strength == 0.0 {
            return;
        }
        match &self.recipe {
            Some(recipe) => recipe.apply(image, self.strength),
            None => analyze(image, context).apply(image, self.strength),
        }
    }
}
