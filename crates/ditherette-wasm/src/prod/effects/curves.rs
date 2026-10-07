//! Ordered one-input and two-input colour-model channel adjustments.

use std::collections::TryReserveError;

use serde::{Deserialize, Serialize};

use crate::prod::contract::error::{DitheretteError, ErrorCode};

use super::{
    chain::{check_bounded, Effect, EffectContext, PreparedPointwise, StackPath},
    image::EffectImage,
    model::ColourModel,
};

/// Fewest and most control points one curve accepts.
pub const MIN_POINTS: usize = 2;
pub const MAX_POINTS: usize = 16;
/// Fewest and most columns a two-input grid accepts.
pub const MIN_COLUMNS: usize = 2;
pub const MAX_COLUMNS: usize = 48;
/// Fewest and most rows a two-input grid accepts.
pub const MIN_ROWS: usize = 2;
pub const MAX_ROWS: usize = 16;
/// Most curves one effect accepts.
pub const MAX_CURVES: usize = 16;
/// Smallest gap between neighbouring x values. Closer knots make the cubic's `1 / h²` overflow.
pub const MIN_GAP: f32 = 0.001;

/// Every channel name used by the supported colour models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelChannel {
    Red,
    Green,
    Blue,
    Hue,
    Saturation,
    Lightness,
    Value,
    A,
    B,
    Chroma,
    Luma,
    Cb,
    Cr,
}

/// A named channel in one colour model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColourChannel {
    pub model: ColourModel,
    pub channel: ModelChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    Hue,
    Chroma,
    Other,
}

/// Model metadata resolved after validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedChannel {
    pub model: ColourModel,
    pub index: usize,
    pub kind: ChannelKind,
}

impl ColourChannel {
    pub fn resolve(self) -> Option<ResolvedChannel> {
        use ColourModel as M;
        use ModelChannel as C;
        let (index, kind) = match (self.model, self.channel) {
            (M::Srgb | M::LinearRgb, C::Red) => (0, ChannelKind::Other),
            (M::Srgb | M::LinearRgb, C::Green) => (1, ChannelKind::Other),
            (M::Srgb | M::LinearRgb, C::Blue) => (2, ChannelKind::Other),
            (M::Hsl | M::Hsv, C::Hue) => (0, ChannelKind::Hue),
            (M::Hsl | M::Hsv, C::Saturation) => (1, ChannelKind::Chroma),
            (M::Hsl, C::Lightness) => (2, ChannelKind::Other),
            (M::Hsv, C::Value) => (2, ChannelKind::Other),
            (M::Oklab | M::Cielab, C::Lightness) => (0, ChannelKind::Other),
            (M::Oklab | M::Cielab, C::A) => (1, ChannelKind::Other),
            (M::Oklab | M::Cielab, C::B) => (2, ChannelKind::Other),
            (M::Oklch | M::Cielch, C::Lightness) => (0, ChannelKind::Other),
            (M::Oklch | M::Cielch, C::Chroma) => (1, ChannelKind::Chroma),
            (M::Oklch | M::Cielch, C::Hue) => (2, ChannelKind::Hue),
            (M::Ycbcr, C::Luma) => (0, ChannelKind::Other),
            (M::Ycbcr, C::Cb) => (1, ChannelKind::Other),
            (M::Ycbcr, C::Cr) => (2, ChannelKind::Other),
            _ => return None,
        };
        Some(ResolvedChannel {
            model: self.model,
            index,
            kind,
        })
    }
}

/// Whether a curve replaces one channel or adjusts it around a neutral midpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CurveKind {
    Remap,
    Adjust,
}

/// A one-input curve. Its input always comes from the effect's original pixel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OneInputCurve {
    pub kind: CurveKind,
    pub x: ColourChannel,
    pub y: ColourChannel,
    pub points: Vec<[f32; 2]>,
}

/// A rectangular two-input control grid in row-major order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurveGrid {
    pub columns: Vec<f32>,
    pub rows: Vec<f32>,
    pub values: Vec<Vec<f32>>,
}

/// A two-input adjustment. Both inputs come from the effect's original pixel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TwoInputCurve {
    pub kind: CurveKind,
    pub x: ColourChannel,
    pub x2: ColourChannel,
    pub y: ColourChannel,
    pub grid: CurveGrid,
}

/// A strict one-input or two-input curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Curve {
    OneInput(OneInputCurve),
    TwoInput(TwoInputCurve),
}

fn try_clone_vec<T: Copy>(items: &[T]) -> Result<Vec<T>, TryReserveError> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(items.len())?;
    copy.extend_from_slice(items);
    Ok(copy)
}

fn try_clone_curve(curve: &Curve) -> Result<Curve, TryReserveError> {
    Ok(match curve {
        Curve::OneInput(curve) => Curve::OneInput(OneInputCurve {
            kind: curve.kind,
            x: curve.x,
            y: curve.y,
            points: try_clone_vec(&curve.points)?,
        }),
        Curve::TwoInput(curve) => Curve::TwoInput(TwoInputCurve {
            kind: curve.kind,
            x: curve.x,
            x2: curve.x2,
            y: curve.y,
            grid: CurveGrid {
                columns: try_clone_vec(&curve.grid.columns)?,
                rows: try_clone_vec(&curve.grid.rows)?,
                values: {
                    let mut values = Vec::new();
                    values.try_reserve_exact(curve.grid.values.len())?;
                    for row in &curve.grid.values {
                        values.push(try_clone_vec(row)?);
                    }
                    values
                },
            },
        }),
    })
}

/// Fallible clone of a curve list, for resolution and validation that hold their own copy.
pub fn try_clone_curves(curves: &[Curve]) -> Result<Vec<Curve>, TryReserveError> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(curves.len())?;
    for curve in curves {
        copy.push(try_clone_curve(curve)?);
    }
    Ok(copy)
}

/// Up to 16 ordered channel curves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Curves {
    pub curves: Vec<Curve>,
}

/// Monotone cubic Hermite spline (Fritsch–Butland tangents) through validated points.
/// Monotone data stays monotone, and no segment overshoots its endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spline {
    points: [[f32; 2]; MAX_POINTS],
    tangents: [f32; MAX_POINTS],
    len: usize,
}

impl Spline {
    /// `points` must have strictly increasing x. Two points give a straight line.
    pub fn new(points: &[[f32; 2]]) -> Self {
        let mut tangents = [0.0; MAX_POINTS];
        for (k, value) in tangents.iter_mut().enumerate().take(points.len()) {
            *value = tangent_at(points, k);
        }
        let mut inline = [[0.0; 2]; MAX_POINTS];
        inline[..points.len()].copy_from_slice(points);
        Self {
            points: inline,
            tangents,
            len: points.len(),
        }
    }

    /// Clamps `x` to the first and last point, then evaluates the containing segment.
    pub fn eval(&self, x: f32) -> f32 {
        let last = self.len - 1;
        let x = x.clamp(self.points[0][0], self.points[last][0]);
        let Some(k) = (0..last).find(|&k| x < self.points[k + 1][0]) else {
            return self.points[last][1];
        };
        hermite(&self.points, &self.tangents, k, x)
    }
}

/// Cyclic Fritsch–Butland spline for a hue-input adjustment axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodicSpline {
    points: [[f32; 2]; MAX_POINTS],
    tangents: [f32; MAX_POINTS],
    len: usize,
}

impl PeriodicSpline {
    /// Points must start at zero, end at one, and repeat the seam y value.
    pub fn new(points: &[[f32; 2]]) -> Self {
        let last = points.len() - 1;
        let secant =
            |k: usize| (points[k + 1][1] - points[k][1]) / (points[k + 1][0] - points[k][0]);
        let seam = tangent(
            secant(last - 1),
            secant(0),
            points[last][0] - points[last - 1][0],
            points[1][0] - points[0][0],
        );
        let mut tangents = [0.0; MAX_POINTS];
        for (k, value) in tangents.iter_mut().enumerate().take(points.len()) {
            *value = if k == 0 || k == last {
                seam
            } else {
                tangent(
                    secant(k - 1),
                    secant(k),
                    points[k][0] - points[k - 1][0],
                    points[k + 1][0] - points[k][0],
                )
            };
        }
        let mut inline = [[0.0; 2]; MAX_POINTS];
        inline[..points.len()].copy_from_slice(points);
        Self {
            points: inline,
            tangents,
            len: points.len(),
        }
    }

    /// Wraps x at the hue seam, then evaluates the containing Hermite segment.
    pub fn eval(&self, x: f32) -> f32 {
        let x = x.rem_euclid(1.0);
        let last = self.len - 1;
        let k = (0..last)
            .find(|&k| x < self.points[k + 1][0])
            .unwrap_or(last - 1);
        hermite(&self.points, &self.tangents, k, x)
    }
}

fn tangent_at(points: &[[f32; 2]], k: usize) -> f32 {
    let last = points.len() - 1;
    let secant = |k: usize| (points[k + 1][1] - points[k][1]) / (points[k + 1][0] - points[k][0]);
    if k == 0 {
        return secant(0);
    }
    if k == last {
        return secant(last - 1);
    }
    tangent(
        secant(k - 1),
        secant(k),
        points[k][0] - points[k - 1][0],
        points[k + 1][0] - points[k][0],
    )
}

fn tangent(before: f32, after: f32, h_before: f32, h_after: f32) -> f32 {
    if before * after <= 0.0 {
        return 0.0;
    }
    let w_before = 2.0 * h_after + h_before;
    let w_after = h_after + 2.0 * h_before;
    (w_before + w_after) / (w_before / before + w_after / after)
}

fn hermite(points: &[[f32; 2]], tangents: &[f32], k: usize, x: f32) -> f32 {
    let [x0, y0] = points[k];
    let [x1, y1] = points[k + 1];
    hermite_segment(x0, y0, x1, y1, tangents[k], tangents[k + 1], x)
}

fn hermite_segment(x0: f32, y0: f32, x1: f32, y1: f32, m0: f32, m1: f32, x: f32) -> f32 {
    let (h, secant) = (x1 - x0, (y1 - y0) / (x1 - x0));
    let c2 = (3.0 * secant - 2.0 * m0 - m1) / h;
    let c3 = (m0 + m1 - 2.0 * secant) / (h * h);
    let s = x - x0;
    y0 + s * (m0 + s * (c2 + s * c3))
}

fn sequence_tangent(positions: &[f32], values: &[f32], k: usize) -> f32 {
    let last = positions.len() - 1;
    let secant = |k: usize| (values[k + 1] - values[k]) / (positions[k + 1] - positions[k]);
    if k == 0 {
        return secant(0);
    }
    if k == last {
        return secant(last - 1);
    }
    tangent(
        secant(k - 1),
        secant(k),
        positions[k] - positions[k - 1],
        positions[k + 1] - positions[k],
    )
}

fn eval_open_sequence(positions: &[f32], values: &[f32], x: f32) -> f32 {
    let last = positions.len() - 1;
    let x = x.clamp(positions[0], positions[last]);
    let Some(k) = (0..last).find(|&k| x < positions[k + 1]) else {
        return values[last];
    };
    hermite_segment(
        positions[k],
        values[k],
        positions[k + 1],
        values[k + 1],
        sequence_tangent(positions, values, k),
        sequence_tangent(positions, values, k + 1),
        x,
    )
}

fn eval_closed_sequence(positions: &[f32], values: &[f32], x: f32) -> f32 {
    let count = positions.len();
    let segment = |k: usize| {
        let next = (k + 1) % count;
        let x1 = if next == 0 {
            positions[0] + 1.0
        } else {
            positions[next]
        };
        let h = x1 - positions[k];
        (h, (values[next] - values[k]) / h)
    };
    let tangent_at = |k: usize| {
        let previous = (k + count - 1) % count;
        let (h_before, before) = segment(previous);
        let (h_after, after) = segment(k);
        tangent(before, after, h_before, h_after)
    };

    let mut x = x.rem_euclid(1.0);
    let k = if x < positions[0] {
        x += 1.0;
        count - 1
    } else {
        (0..count - 1)
            .find(|&k| x < positions[k + 1])
            .unwrap_or(count - 1)
    };
    let next = (k + 1) % count;
    let x1 = if next == 0 {
        positions[0] + 1.0
    } else {
        positions[next]
    };
    hermite_segment(
        positions[k],
        values[k],
        x1,
        values[next],
        tangent_at(k),
        tangent_at(next),
        x,
    )
}

fn eval_sequence(positions: &[f32], values: &[f32], x: f32, cyclic: bool) -> f32 {
    if cyclic {
        eval_closed_sequence(positions, values, x)
    } else {
        eval_open_sequence(positions, values, x)
    }
}

/// A one-input curve's spline: open, or cyclic with a duplicate seam point for a hue input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveSpline {
    Linear(Spline),
    Periodic(PeriodicSpline),
}

impl CurveGrid {
    /// Evaluates each row along `columns` at `x`, then the row results along `rows` at `x2`,
    /// clamped to `[0,1]`. Row results use fixed scratch, so evaluation never allocates.
    pub fn eval(&self, x: f32, x2: f32, x_cyclic: bool, x2_cyclic: bool) -> f32 {
        let mut row_results = [0.0; MAX_ROWS];
        for (result, values) in row_results.iter_mut().zip(&self.values) {
            *result = eval_sequence(&self.columns, values, x, x_cyclic);
        }
        eval_sequence(&self.rows, &row_results[..self.rows.len()], x2, x2_cyclic).clamp(0.0, 1.0)
    }
}

/// Scales a curve value's distance from `neutral` by `strength`. Strength 1 keeps it exactly.
fn toward(neutral: f32, value: f32, strength: f32) -> f32 {
    if strength == 1.0 {
        value
    } else {
        neutral + strength * (value - neutral)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PreparedCurve {
    OneInput {
        kind: CurveKind,
        x: ResolvedChannel,
        y: ResolvedChannel,
        spline: CurveSpline,
        identity_remap: bool,
    },
    TwoInput {
        curve_index: usize,
        x: ResolvedChannel,
        x2: ResolvedChannel,
        y: ResolvedChannel,
    },
}

impl PreparedCurve {
    fn map_one_input(
        kind: CurveKind,
        x: ResolvedChannel,
        y: ResolvedChannel,
        spline: &CurveSpline,
        identity_remap: bool,
        source: [f32; 3],
        current: [f32; 3],
        strength: f32,
    ) -> [f32; 3] {
        if identity_remap
            && source
                .iter()
                .zip(current)
                .all(|(source, current)| source.to_bits() == current.to_bits())
        {
            return current;
        }
        let x_coordinates = x.model.to_normalized(source);
        let original = x_coordinates[x.index];
        let curve = spline.eval(original);
        let mut weight: f32 = 1.0;
        if x.kind == ChannelKind::Hue {
            weight = x.model.hue_weight(source, x_coordinates);
            if weight == 0.0 {
                return current;
            }
        }
        let mut y_coordinates = y.model.to_normalized(current);
        match kind {
            CurveKind::Remap => {
                if y.kind == ChannelKind::Hue {
                    let delta = strength * ((curve - original + 0.5).rem_euclid(1.0) - 0.5);
                    y_coordinates[y.index] = (original + weight * delta).rem_euclid(1.0);
                } else {
                    y_coordinates[y.index] = toward(original, curve, strength);
                }
            }
            CurveKind::Adjust => {
                let curve = toward(0.5, curve, strength);
                if y.kind == ChannelKind::Hue {
                    weight = weight.min(y.model.hue_weight(current, y_coordinates));
                    if weight == 0.0 {
                        return current;
                    }
                }
                match y.kind {
                    ChannelKind::Hue => {
                        y_coordinates[y.index] =
                            (y_coordinates[y.index] + weight * (curve - 0.5)).rem_euclid(1.0);
                    }
                    ChannelKind::Chroma => {
                        y_coordinates[y.index] *= 1.0 + weight * (2.0 * curve - 1.0);
                    }
                    ChannelKind::Other => {
                        y_coordinates[y.index] += weight * (curve - 0.5);
                    }
                }
            }
        }
        y.model.from_normalized(y_coordinates)
    }

    fn map_two_input(
        curve: &TwoInputCurve,
        x: ResolvedChannel,
        x2: ResolvedChannel,
        y: ResolvedChannel,
        source: [f32; 3],
        current: [f32; 3],
        strength: f32,
    ) -> [f32; 3] {
        let x_coordinates = x.model.to_normalized(source);
        let x2_coordinates = if x2.model == x.model {
            x_coordinates
        } else {
            x2.model.to_normalized(source)
        };
        let curve_value = curve.grid.eval(
            x_coordinates[x.index],
            x2_coordinates[x2.index],
            x.kind == ChannelKind::Hue,
            x2.kind == ChannelKind::Hue,
        );
        let curve_value = toward(0.5, curve_value, strength);

        let mut weight: f32 = 1.0;
        if x.kind == ChannelKind::Hue {
            weight = x.model.hue_weight(source, x_coordinates);
        }
        if x2.kind == ChannelKind::Hue {
            weight = weight.min(x2.model.hue_weight(source, x2_coordinates));
        }
        if weight == 0.0 {
            return current;
        }

        let mut y_coordinates = y.model.to_normalized(current);
        if y.kind == ChannelKind::Hue {
            weight = weight.min(y.model.hue_weight(current, y_coordinates));
            if weight == 0.0 {
                return current;
            }
        }
        match y.kind {
            ChannelKind::Hue => {
                y_coordinates[y.index] =
                    (y_coordinates[y.index] + weight * (curve_value - 0.5)).rem_euclid(1.0);
            }
            ChannelKind::Chroma => {
                y_coordinates[y.index] *= 1.0 + weight * (2.0 * curve_value - 1.0);
            }
            ChannelKind::Other => {
                y_coordinates[y.index] += weight * (curve_value - 0.5);
            }
        }
        y.model.from_normalized(y_coordinates)
    }
}

/// Up to 16 resolved curves and inline splines, prepared once for one effects call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedCurves {
    curves: [Option<PreparedCurve>; MAX_CURVES],
    len: usize,
}

impl PreparedCurves {
    pub fn new(curves: &[Curve]) -> Self {
        let mut prepared = Self {
            curves: [None; MAX_CURVES],
            len: 0,
        };
        for (curve_index, curve) in curves.iter().enumerate() {
            let entry = match curve {
                Curve::OneInput(curve) => {
                    if curve.is_neutral_adjustment() {
                        continue;
                    }
                    let x = curve.x.resolve().expect("validated curves x channel");
                    PreparedCurve::OneInput {
                        kind: curve.kind,
                        x,
                        y: curve.y.resolve().expect("validated curves y channel"),
                        spline: CurveSpline::new(
                            &curve.points,
                            curve.kind == CurveKind::Adjust && x.kind == ChannelKind::Hue,
                        ),
                        identity_remap: curve.is_identity_remap(),
                    }
                }
                Curve::TwoInput(curve) => {
                    if curve.is_neutral_adjustment() {
                        continue;
                    }
                    PreparedCurve::TwoInput {
                        curve_index,
                        x: curve.x.resolve().expect("validated curves x channel"),
                        x2: curve.x2.resolve().expect("validated curves x2 channel"),
                        y: curve.y.resolve().expect("validated curves y channel"),
                    }
                }
            };
            prepared.curves[prepared.len] = Some(entry);
            prepared.len += 1;
        }
        prepared
    }

    /// `strength` is the step's mask value at this pixel, in `(0, 1]`.
    pub fn map(&self, curves: &[Curve], source: [f32; 3], strength: f32) -> [f32; 3] {
        let mut current = source;
        for prepared in self.curves[..self.len].iter().flatten() {
            current = match prepared {
                PreparedCurve::OneInput {
                    kind,
                    x,
                    y,
                    spline,
                    identity_remap,
                } => PreparedCurve::map_one_input(
                    *kind,
                    *x,
                    *y,
                    spline,
                    *identity_remap,
                    source,
                    current,
                    strength,
                ),
                PreparedCurve::TwoInput {
                    curve_index,
                    x,
                    x2,
                    y,
                } => {
                    let Curve::TwoInput(curve) = &curves[*curve_index] else {
                        unreachable!("prepared grid index resolved to a one-input curve")
                    };
                    PreparedCurve::map_two_input(curve, *x, *x2, *y, source, current, strength)
                }
            };
        }
        current
    }
}

impl CurveSpline {
    pub fn new(points: &[[f32; 2]], periodic: bool) -> Self {
        if periodic {
            Self::Periodic(PeriodicSpline::new(points))
        } else {
            Self::Linear(Spline::new(points))
        }
    }

    pub fn eval(&self, x: f32) -> f32 {
        match self {
            Self::Linear(spline) => spline.eval(x),
            Self::Periodic(spline) => spline.eval(x),
        }
    }
}

impl OneInputCurve {
    fn is_identity_remap(&self) -> bool {
        self.kind == CurveKind::Remap && self.points.as_slice() == [[0.0, 0.0], [1.0, 1.0]]
    }

    fn is_neutral_adjustment(&self) -> bool {
        self.kind == CurveKind::Adjust && self.points.iter().all(|point| point[1] == 0.5)
    }
}

/// A cyclic one-input spline's points start at x = 0, end at x = 1, and repeat the seam's y.
pub fn validate_periodic_points(points: &[[f32; 2]], path: &str) -> Result<(), DitheretteError> {
    let last = points.len() - 1;
    let invalid = |index: usize, coordinate: usize, message| {
        DitheretteError::new(
            ErrorCode::InvalidSettings,
            StackPath::new(format_args!("{path}.{index}.{coordinate}")).as_str(),
            message,
        )
    };
    if points[0][0] != 0.0 {
        return Err(invalid(0, 0, "A hue-input curve must start at x = 0."));
    }
    if points[last][0] != 1.0 {
        return Err(invalid(last, 0, "A hue-input curve must end at x = 1."));
    }
    if points[last][1] != points[0][1] {
        return Err(invalid(
            last,
            1,
            "A hue-input curve must repeat its first y value at x = 1.",
        ));
    }
    Ok(())
}

/// Resolves a channel, or fails at `{path}.channel` when it is not in its model.
pub fn validate_channel(
    channel: ColourChannel,
    path: &str,
) -> Result<ResolvedChannel, DitheretteError> {
    channel.resolve().ok_or_else(|| {
        DitheretteError::new(
            ErrorCode::InvalidSettings,
            StackPath::new(format_args!("{path}.channel")).as_str(),
            "Channel does not belong to the selected colour model.",
        )
    })
}

impl TwoInputCurve {
    fn is_neutral_adjustment(&self) -> bool {
        self.grid.values.iter().flatten().all(|&value| value == 0.5)
    }
}

impl CurveGrid {
    fn validate_axis(positions: &[f32], path: &str, cyclic: bool) -> Result<(), DitheretteError> {
        for (index, &position) in positions.iter().enumerate() {
            if cyclic {
                if !position.is_finite() || !(0.0..1.0).contains(&position) {
                    return Err(DitheretteError::new(
                        ErrorCode::InvalidSettings,
                        StackPath::new(format_args!("{path}.{index}")).as_str(),
                        "Value must be finite and at least 0 but less than 1.",
                    ));
                }
            } else {
                check_bounded(position, 0.0, 1.0, format_args!("{path}.{index}"))?;
            }
            if index > 0 && position - positions[index - 1] < MIN_GAP {
                return Err(DitheretteError::new(
                    ErrorCode::InvalidSettings,
                    StackPath::new(format_args!("{path}.{index}")).as_str(),
                    format!("Positions must increase by at least {MIN_GAP}."),
                ));
            }
        }

        if cyclic {
            let seam_gap = 1.0 - positions[positions.len() - 1] + positions[0];
            if seam_gap < MIN_GAP {
                return Err(DitheretteError::new(
                    ErrorCode::InvalidSettings,
                    StackPath::new(format_args!("{path}.0")).as_str(),
                    format!("The wrapped seam must span at least {MIN_GAP}."),
                ));
            }
        } else {
            if positions[0] != 0.0 {
                return Err(DitheretteError::new(
                    ErrorCode::InvalidSettings,
                    StackPath::new(format_args!("{path}.0")).as_str(),
                    "An open grid axis must start at 0.",
                ));
            }
            let last = positions.len() - 1;
            if positions[last] != 1.0 {
                return Err(DitheretteError::new(
                    ErrorCode::InvalidSettings,
                    StackPath::new(format_args!("{path}.{last}")).as_str(),
                    "An open grid axis must end at 1.",
                ));
            }
        }
        Ok(())
    }

    /// Checks counts, both axes, then every value, naming the failing entry under `path`.
    pub fn validate(
        &self,
        path: &str,
        x_cyclic: bool,
        x2_cyclic: bool,
    ) -> Result<(), DitheretteError> {
        if !(MIN_COLUMNS..=MAX_COLUMNS).contains(&self.columns.len()) {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                StackPath::new(format_args!("{path}.columns")).as_str(),
                format!("Expected {MIN_COLUMNS} to {MAX_COLUMNS} columns."),
            ));
        }
        if !(MIN_ROWS..=MAX_ROWS).contains(&self.rows.len()) {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                StackPath::new(format_args!("{path}.rows")).as_str(),
                format!("Expected {MIN_ROWS} to {MAX_ROWS} rows."),
            ));
        }
        Self::validate_axis(
            &self.columns,
            StackPath::new(format_args!("{path}.columns")).as_str(),
            x_cyclic,
        )?;
        Self::validate_axis(
            &self.rows,
            StackPath::new(format_args!("{path}.rows")).as_str(),
            x2_cyclic,
        )?;
        if self.values.len() != self.rows.len() {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                StackPath::new(format_args!("{path}.values")).as_str(),
                "Grid values must contain one entry per row.",
            ));
        }
        for (row, values) in self.values.iter().enumerate() {
            if values.len() != self.columns.len() {
                return Err(DitheretteError::new(
                    ErrorCode::InvalidSettings,
                    StackPath::new(format_args!("{path}.values.{row}")).as_str(),
                    "Each grid row must contain one value per column.",
                ));
            }
            for (column, &value) in values.iter().enumerate() {
                check_bounded(
                    value,
                    0.0,
                    1.0,
                    format_args!("{path}.values.{row}.{column}"),
                )?;
            }
        }
        Ok(())
    }
}

impl Curve {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        match self {
            Self::OneInput(curve) => {
                let x =
                    validate_channel(curve.x, StackPath::new(format_args!("{path}.x")).as_str())?;
                validate_channel(curve.y, StackPath::new(format_args!("{path}.y")).as_str())?;
                if curve.kind == CurveKind::Remap && curve.x != curve.y {
                    return Err(DitheretteError::new(
                        ErrorCode::InvalidSettings,
                        StackPath::new(format_args!("{path}.y")).as_str(),
                        "A remap must use the same input and output channel.",
                    ));
                }
                let points_path = StackPath::new(format_args!("{path}.points"));
                Curves::validate_points(&curve.points, points_path.as_str())?;
                if curve.kind == CurveKind::Adjust && x.kind == ChannelKind::Hue {
                    validate_periodic_points(&curve.points, points_path.as_str())?;
                }
                Ok(())
            }
            Self::TwoInput(curve) => {
                if curve.kind != CurveKind::Adjust {
                    return Err(DitheretteError::new(
                        ErrorCode::InvalidSettings,
                        StackPath::new(format_args!("{path}.kind")).as_str(),
                        "A two-input curve must be an adjustment.",
                    ));
                }
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
                validate_channel(curve.y, StackPath::new(format_args!("{path}.y")).as_str())?;
                curve.grid.validate(
                    StackPath::new(format_args!("{path}.grid")).as_str(),
                    x.kind == ChannelKind::Hue,
                    x2.kind == ChannelKind::Hue,
                )
            }
        }
    }
}

impl Curves {
    /// A list of curves validates exactly like a `Curves` step's own list.
    pub fn validate_list(curves: &[Curve], path: &str) -> Result<(), DitheretteError> {
        if curves.len() > MAX_CURVES {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                StackPath::new(format_args!("{path}.curves")).as_str(),
                format!("Expected 0 to {MAX_CURVES} curves."),
            ));
        }
        for (index, curve) in curves.iter().enumerate() {
            curve.validate(StackPath::new(format_args!("{path}.curves.{index}")).as_str())?;
        }
        Ok(())
    }

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
            check_bounded(*x, 0.0, 1.0, format_args!("{path}.{index}.0"))?;
            check_bounded(*y, 0.0, 1.0, format_args!("{path}.{index}.1"))?;
            if index > 0 && *x - points[index - 1][0] < MIN_GAP {
                return Err(DitheretteError::new(
                    ErrorCode::InvalidSettings,
                    StackPath::new(format_args!("{path}.{index}.0")).as_str(),
                    format!("Point x values must increase by at least {MIN_GAP}."),
                ));
            }
        }
        Ok(())
    }

    fn prepared(&self) -> PreparedCurves {
        PreparedCurves::new(&self.curves)
    }

    fn table_foldable(&self) -> bool {
        self.curves.iter().all(|curve| {
            let Curve::OneInput(curve) = curve else {
                return false;
            };
            curve.kind == CurveKind::Remap
                && curve.x == curve.y
                && matches!(curve.x.model, ColourModel::Srgb | ColourModel::LinearRgb)
        })
    }
}

impl Effect for Curves {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        Self::validate_list(&self.curves, path)
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        if self.curves.is_empty() {
            return;
        }
        let prepared = self.prepared();
        for rgb in &mut image.rgb {
            *rgb = prepared.map(&self.curves, *rgb, 1.0);
        }
    }

    /// The mask scales every curve's bend from neutral; a zero mask leaves the pixel unchanged.
    fn apply_masked(
        &self,
        image: &mut EffectImage,
        _input: &[[f32; 3]],
        strengths: &[f32],
        _context: &EffectContext<'_>,
    ) {
        if self.curves.is_empty() {
            return;
        }
        let prepared = self.prepared();
        for (rgb, &strength) in image.rgb.iter_mut().zip(strengths) {
            if strength != 0.0 {
                *rgb = prepared.map(&self.curves, *rgb, strength);
            }
        }
    }

    fn per_channel(&self) -> bool {
        self.table_foldable()
    }

    fn map_channel(&self, channel: usize, value: f32) -> f32 {
        self.prepared().map(&self.curves, [value; 3], 1.0)[channel]
    }

    fn pointwise(&self) -> bool {
        true
    }

    fn map_pixel(&self, rgb: [f32; 3], _context: &EffectContext<'_>) -> [f32; 3] {
        self.prepared().map(&self.curves, rgb, 1.0)
    }

    fn prepare_pointwise(&self) -> PreparedPointwise {
        PreparedPointwise::Curves(self.prepared())
    }

    fn map_prepared_channel(
        &self,
        prepared: &PreparedPointwise,
        channel: usize,
        value: f32,
    ) -> f32 {
        let PreparedPointwise::Curves(prepared) = prepared else {
            unreachable!("curves received another effect's prepared state")
        };
        prepared.map(&self.curves, [value; 3], 1.0)[channel]
    }

    fn map_prepared(
        &self,
        prepared: &PreparedPointwise,
        rgb: [f32; 3],
        _context: &EffectContext<'_>,
    ) -> [f32; 3] {
        let PreparedPointwise::Curves(prepared) = prepared else {
            unreachable!("curves received another effect's prepared state")
        };
        prepared.map(&self.curves, rgb, 1.0)
    }

    fn map_prepared_masked(
        &self,
        prepared: &PreparedPointwise,
        rgb: [f32; 3],
        strength: f32,
        _context: &EffectContext<'_>,
    ) -> [f32; 3] {
        let PreparedPointwise::Curves(prepared) = prepared else {
            unreachable!("curves received another effect's prepared state")
        };
        if strength == 0.0 {
            return rgb;
        }
        prepared.map(&self.curves, rgb, strength)
    }
}
