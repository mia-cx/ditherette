//! Curves: a smooth, overshoot-free tone curve through caller control points.

use serde::{Deserialize, Serialize};

use crate::spec::contract::error::{DitheretteError, ErrorCode};

use super::{
    chain::{check_bounded, Effect, EffectContext},
    channel::Channel,
    image::EffectImage,
};

/// Fewest and most control points one curve accepts.
pub const MIN_POINTS: usize = 2;
pub const MAX_POINTS: usize = 16;

/// Per-channel curve. `points` are `[x, y]` pairs in encoded sRGB units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Curves {
    pub channel: Channel,
    pub points: Vec<[f32; 2]>,
}

/// Monotone cubic Hermite spline (Fritsch–Butland tangents) through validated points.
/// Monotone data stays monotone, and no segment overshoots its endpoints.
#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    points: Vec<[f32; 2]>,
    tangents: Vec<f32>,
}

impl Spline {
    /// `points` must have strictly increasing x. Two points give a straight line.
    pub fn new(points: &[[f32; 2]]) -> Self {
        let last = points.len() - 1;
        let secant =
            |k: usize| (points[k + 1][1] - points[k][1]) / (points[k + 1][0] - points[k][0]);
        let tangents = (0..=last)
            .map(|k| {
                if k == 0 {
                    return secant(0);
                }
                if k == last {
                    return secant(last - 1);
                }
                let (before, after) = (secant(k - 1), secant(k));
                if before * after <= 0.0 {
                    return 0.0;
                }
                let h_before = points[k][0] - points[k - 1][0];
                let h_after = points[k + 1][0] - points[k][0];
                let w_before = 2.0 * h_after + h_before;
                let w_after = h_after + 2.0 * h_before;
                (w_before + w_after) / (w_before / before + w_after / after)
            })
            .collect();
        Self {
            points: points.to_vec(),
            tangents,
        }
    }

    /// Clamps `x` to the first and last point, then evaluates the segment containing it.
    pub fn eval(&self, x: f32) -> f32 {
        let last = self.points.len() - 1;
        let x = x.clamp(self.points[0][0], self.points[last][0]);
        let k = (0..last)
            .find(|&k| x <= self.points[k + 1][0])
            .expect("x is clamped to the last point");
        let [x0, y0] = self.points[k];
        let [x1, y1] = self.points[k + 1];
        let (h, secant) = (x1 - x0, (y1 - y0) / (x1 - x0));
        let (m0, m1) = (self.tangents[k], self.tangents[k + 1]);
        let c2 = (3.0 * secant - 2.0 * m0 - m1) / h;
        let c3 = (m0 + m1 - 2.0 * secant) / (h * h);
        let s = x - x0;
        y0 + s * (m0 + s * (c2 + s * c3))
    }
}

impl Curves {
    /// Checks point count, bounds, and strictly increasing x, naming the failing point.
    pub fn validate_points(points: &[[f32; 2]], path: &str) -> Result<(), DitheretteError> {
        if !(MIN_POINTS..=MAX_POINTS).contains(&points.len()) {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                path,
                format!("Expected {MIN_POINTS} to {MAX_POINTS} points."),
            ));
        }
        for (index, [x, y]) in points.iter().enumerate() {
            check_bounded(*x, 0.0, 1.0, format!("{path}.{index}.0"))?;
            check_bounded(*y, 0.0, 1.0, format!("{path}.{index}.1"))?;
            if index > 0 && *x <= points[index - 1][0] {
                return Err(DitheretteError::new(
                    ErrorCode::InvalidSettings,
                    format!("{path}.{index}.0"),
                    "Point x values must strictly increase.",
                ));
            }
        }
        Ok(())
    }
}

impl Effect for Curves {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        Self::validate_points(&self.points, &format!("{path}.points"))
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        let spline = Spline::new(&self.points);
        for rgb in &mut image.rgb {
            self.channel.apply(rgb, |value| spline.eval(value));
        }
    }
}
