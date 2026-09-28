//! Ordered colour-model channel remaps and adjustments.

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

/// One curve. The input always comes from the effect's original pixel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Curve {
    pub kind: CurveKind,
    pub x: ColourChannel,
    pub y: ColourChannel,
    pub points: Vec<[f32; 2]>,
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

/// Cyclic Fritsch–Butland spline for a hue input axis.
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
    let (h, secant) = (x1 - x0, (y1 - y0) / (x1 - x0));
    let (m0, m1) = (tangents[k], tangents[k + 1]);
    let c2 = (3.0 * secant - 2.0 * m0 - m1) / h;
    let c3 = (m0 + m1 - 2.0 * secant) / (h * h);
    let s = x - x0;
    y0 + s * (m0 + s * (c2 + s * c3))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CurveSpline {
    Linear(Spline),
    Periodic(PeriodicSpline),
}

/// One validated curve with its channels and spline resolved for repeated pixel work.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PreparedCurve {
    kind: CurveKind,
    x: ResolvedChannel,
    y: ResolvedChannel,
    spline: CurveSpline,
}

impl PreparedCurve {
    fn map(&self, source: [f32; 3], current: [f32; 3]) -> [f32; 3] {
        let x_coordinates = self.x.model.to_normalized(source);
        let curve = self.spline.eval(x_coordinates[self.x.index]);
        let mut weight: f32 = 1.0;
        if self.x.kind == ChannelKind::Hue {
            weight = self.x.model.hue_weight(source, x_coordinates);
            if weight == 0.0 {
                return current;
            }
        }
        let mut y_coordinates = self.y.model.to_normalized(current);
        match self.kind {
            CurveKind::Remap => {
                if self.y.kind == ChannelKind::Hue {
                    let original = x_coordinates[self.x.index];
                    let delta = (curve - original + 0.5).rem_euclid(1.0) - 0.5;
                    y_coordinates[self.y.index] = (original + weight * delta).rem_euclid(1.0);
                } else {
                    y_coordinates[self.y.index] = curve;
                }
            }
            CurveKind::Adjust => {
                if self.y.kind == ChannelKind::Hue {
                    weight = weight.min(self.y.model.hue_weight(current, y_coordinates));
                    if weight == 0.0 {
                        return current;
                    }
                }
                match self.y.kind {
                    ChannelKind::Hue => {
                        y_coordinates[self.y.index] =
                            (y_coordinates[self.y.index] + weight * (curve - 0.5)).rem_euclid(1.0);
                    }
                    ChannelKind::Chroma => {
                        y_coordinates[self.y.index] *= 1.0 + weight * (2.0 * curve - 1.0);
                    }
                    ChannelKind::Other => {
                        y_coordinates[self.y.index] += weight * (curve - 0.5);
                    }
                }
            }
        }
        self.y.model.from_normalized(y_coordinates)
    }
}

/// Up to 16 resolved curves and inline splines, prepared once for one effects call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedCurves {
    curves: [Option<PreparedCurve>; MAX_CURVES],
    len: usize,
}

impl PreparedCurves {
    fn new(effect: &Curves) -> Self {
        let mut prepared = Self {
            curves: [None; MAX_CURVES],
            len: 0,
        };
        for curve in &effect.curves {
            if curve.is_neutral_adjustment() {
                continue;
            }
            let x = curve.x.resolve().expect("validated curves x channel");
            let y = curve.y.resolve().expect("validated curves y channel");
            prepared.curves[prepared.len] = Some(PreparedCurve {
                kind: curve.kind,
                x,
                y,
                spline: CurveSpline::new(&curve.points, x.kind == ChannelKind::Hue),
            });
            prepared.len += 1;
        }
        prepared
    }

    fn map(&self, source: [f32; 3]) -> [f32; 3] {
        self.curves[..self.len]
            .iter()
            .flatten()
            .fold(source, |current, curve| curve.map(source, current))
    }
}

impl CurveSpline {
    fn new(points: &[[f32; 2]], periodic: bool) -> Self {
        if periodic {
            Self::Periodic(PeriodicSpline::new(points))
        } else {
            Self::Linear(Spline::new(points))
        }
    }

    fn eval(&self, x: f32) -> f32 {
        match self {
            Self::Linear(spline) => spline.eval(x),
            Self::Periodic(spline) => spline.eval(x),
        }
    }
}

impl Curve {
    fn is_neutral_adjustment(&self) -> bool {
        self.kind == CurveKind::Adjust && self.points.iter().all(|point| point[1] == 0.5)
    }

    fn validate_channel(
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

    fn validate_periodic_points(&self, path: &str) -> Result<(), DitheretteError> {
        let last = self.points.len() - 1;
        let invalid = |index: usize, coordinate: usize, message| {
            DitheretteError::new(
                ErrorCode::InvalidSettings,
                StackPath::new(format_args!("{path}.{index}.{coordinate}")).as_str(),
                message,
            )
        };
        if self.points[0][0] != 0.0 {
            return Err(invalid(0, 0, "A hue-input curve must start at x = 0."));
        }
        if self.points[last][0] != 1.0 {
            return Err(invalid(last, 0, "A hue-input curve must end at x = 1."));
        }
        if self.points[last][1] != self.points[0][1] {
            return Err(invalid(
                last,
                1,
                "A hue-input curve must repeat its first y value at x = 1.",
            ));
        }
        Ok(())
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
        PreparedCurves::new(self)
    }

    fn table_foldable(&self) -> bool {
        self.curves.iter().all(|curve| {
            curve.kind == CurveKind::Remap
                && curve.x == curve.y
                && matches!(curve.x.model, ColourModel::Srgb | ColourModel::LinearRgb)
        })
    }
}

impl Effect for Curves {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        if self.curves.len() > MAX_CURVES {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                StackPath::new(format_args!("{path}.curves")).as_str(),
                format!("Expected 0 to {MAX_CURVES} curves."),
            ));
        }
        for (index, curve) in self.curves.iter().enumerate() {
            let curve_path = StackPath::new(format_args!("{path}.curves.{index}"));
            let x_path = StackPath::new(format_args!("{}.x", curve_path.as_str()));
            let y_path = StackPath::new(format_args!("{}.y", curve_path.as_str()));
            let x = Curve::validate_channel(curve.x, x_path.as_str())?;
            Curve::validate_channel(curve.y, y_path.as_str())?;
            if curve.kind == CurveKind::Remap && curve.x != curve.y {
                return Err(DitheretteError::new(
                    ErrorCode::InvalidSettings,
                    y_path.as_str(),
                    "A remap must use the same input and output channel.",
                ));
            }
            let points_path = StackPath::new(format_args!("{}.points", curve_path.as_str()));
            Self::validate_points(&curve.points, points_path.as_str())?;
            if x.kind == ChannelKind::Hue {
                curve.validate_periodic_points(points_path.as_str())?;
            }
        }
        Ok(())
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        if self.curves.is_empty() {
            return;
        }
        let prepared = self.prepared();
        for rgb in &mut image.rgb {
            *rgb = prepared.map(*rgb);
        }
    }

    fn per_channel(&self) -> bool {
        self.table_foldable()
    }

    fn map_channel(&self, channel: usize, value: f32) -> f32 {
        self.prepared().map([value; 3])[channel]
    }

    fn pointwise(&self) -> bool {
        true
    }

    fn map_pixel(&self, rgb: [f32; 3], _context: &EffectContext<'_>) -> [f32; 3] {
        self.prepared().map(rgb)
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
        prepared.map([value; 3])[channel]
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
        prepared.map(rgb)
    }
}
