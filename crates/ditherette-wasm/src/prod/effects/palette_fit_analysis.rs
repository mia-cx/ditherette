//! Palette-fit analysis: derive a curve list from an image and a palette.
//!
//! Mirrors `spec::effects::palette_fit_analysis` with fallible allocation and cached-friendly
//! inputs. Sampling, palette, tone, shift, and flat reach follow `recolour_analysis` exactly.

use std::collections::TryReserveError;

use crate::prod::contract::request::WorkingSpace;

use super::{
    chain::EffectContext,
    curves::{
        ColourChannel, Curve, CurveGrid, CurveKind, ModelChannel, OneInputCurve, Spline,
        TwoInputCurve, MIN_GAP,
    },
    image::EffectImage,
    palette_fit::{FitLook, FitSpace},
    recolour::{window, Group, NEUTRAL_CHROMA},
    space::to_opponent,
};

/// Most pixels analysis reads. Larger images are read on a regular grid.
pub const MAX_SAMPLES: u64 = 1 << 18;
/// Tone quantiles mapped onto the palette's lightness range. The ends ignore 1% outliers.
const TONE_QUANTILES: [f32; 5] = [0.01, 0.25, 0.5, 0.75, 0.99];
/// Share of each tone target taken from the palette's own lightness distribution.
const PALETTE_TONE_SHARE: f32 = 0.5;
/// Palettes with this many lightness levels or fewer take the full share; richer ones less.
const SPARSE_LEVELS: f32 = 4.0;
/// The shift moves image colours this share of the way toward the palette centroid.
const SHIFT_SHARE: f32 = 0.5;
const MAX_SHIFT_LENGTH: f32 = 0.1;
/// Colours this close to neutral count as exactly neutral, in the image and the palette.
const NEUTRAL_SNAP: f32 = 0.001;
/// Hue columns and lightness rows of the cell grid, evenly spaced.
const COLUMNS: usize = 12;
const ROWS: usize = 5;
const COLUMN_DEGREES: f32 = 360.0 / COLUMNS as f32;
const ROW_STEP: f32 = 1.0 / (ROWS - 1) as f32;
/// A column below this share of coloured mass keeps a zero turn.
const MIN_COLUMN_MASS: f32 = 0.02;
/// A cell below this share of coloured mass borrows its row's mean gain.
const MIN_CELL_MASS: f32 = 0.005;
/// A direction counts as reachable when the palette's hull extends this far relative to the cell.
const REACH_SHARE: f32 = 0.5;
/// Largest hue turn toward a reachable direction, searched in 5-degree steps.
const MAX_TURN: f32 = 45.0;
/// Each look's chroma gain cap: `natural` never grows chroma, `vivid` reaches the palette's
/// full slice reach, and `fitted` sits between.
fn gain_cap(look: FitLook) -> f32 {
    match look {
        FitLook::Natural => 1.0,
        FitLook::Fitted => 1.25,
        FitLook::Vivid => 2.0,
    }
}

/// Bytes analysis may hold: the sample list, its sort copy, the palette, cells, and the emitted
/// list, which is at most five curves with 16 points or a 5-by-12 grid each.
pub const ANALYSIS_BYTES: u64 = MAX_SAMPLES
    * (size_of::<Sample>() + size_of::<(f32, f32)>()) as u64
    + size_of::<[f32; 3]>() as u64 * 256
    + 5 * size_of::<Curve>() as u64
    + (16 + 2 * 2 + 13 + 48 + 16 + 60) * size_of::<f32>() as u64
    + 1024;

/// One read pixel: working-space coordinates and its alpha weight in `(0, 1]`.
struct Sample {
    opponent: [f32; 3],
    weight: f32,
}

/// Derives the curve list for `image` under the step's own `space` and `look`.
/// The context supplies the palette; its space is ignored.
pub fn analyze(
    image: &EffectImage,
    context: &EffectContext<'_>,
    space: FitSpace,
    look: FitLook,
) -> Result<Vec<Curve>, TryReserveError> {
    let working = space.working_space();
    let mut colors: Vec<[u8; 3]> = Vec::new();
    colors.try_reserve_exact(context.colors().count())?;
    for color in context.colors() {
        if !colors.contains(&color) {
            colors.push(color);
        }
    }
    let mut palette: Vec<[f32; 3]> = Vec::new();
    palette.try_reserve_exact(colors.len())?;
    palette.extend(colors.iter().map(|rgb| {
        snap_neutral(to_opponent(
            rgb.map(|channel| channel as f32 / 255.0),
            working,
        ))
    }));
    let samples = sample(image, working)?;
    if samples.is_empty() || palette.is_empty() {
        return Ok(Vec::new());
    }
    let tone = tone(&samples, &palette)?;
    // `natural` measures the cells with no shift and never turns a column.
    let shift = if matches!(look, FitLook::Natural) {
        [0.0, 0.0]
    } else {
        shift(&palette)
    };
    let cells = cells(&samples, shift);
    let turns = if matches!(look, FitLook::Natural) {
        [0.0; COLUMNS]
    } else {
        turns(&cells, &palette)
    };
    let gains = gains(&cells, &palette, &tone, &turns, look);
    emit(space, tone, shift, gains, turns)
}

/// Byte greys carry `f32` residue off neutral in perceptual spaces; treat them as grey.
fn snap_neutral([lightness, u, v]: [f32; 3]) -> [f32; 3] {
    if u.hypot(v) < NEUTRAL_SNAP {
        [lightness, 0.0, 0.0]
    } else {
        [lightness, u, v]
    }
}

/// Visible pixels on the coarsest grid step `s` with `ceil(w/s) * ceil(h/s) <= MAX_SAMPLES`,
/// in row-major order. Zero-alpha pixels are skipped; others weigh `alpha / 255`.
fn sample(image: &EffectImage, space: WorkingSpace) -> Result<Vec<Sample>, TryReserveError> {
    let width = image.dimensions.width_usize();
    let height = image.dimensions.height() as usize;
    let mut step = 1;
    while (width.div_ceil(step) * height.div_ceil(step)) as u64 > MAX_SAMPLES {
        step += 1;
    }
    let mut samples = Vec::new();
    samples.try_reserve_exact(
        (width.div_ceil(step) * height.div_ceil(step)).min(MAX_SAMPLES as usize),
    )?;
    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let index = y * width + x;
            let alpha = image.alpha[index];
            if alpha > 0 {
                samples.push(Sample {
                    opponent: snap_neutral(to_opponent(image.rgb[index], space)),
                    weight: alpha as f32 / 255.0,
                });
            }
        }
    }
    Ok(samples)
}

/// Lightness at weighted quantile `p`: samples sorted by lightness, the first whose
/// cumulative weight reaches `p` of the total.
fn quantile(sorted: &[(f32, f32)], total: f32, p: f32) -> f32 {
    let target = p * total;
    let mut cumulative = 0.0;
    for &(lightness, weight) in sorted {
        cumulative += weight;
        if cumulative >= target {
            return lightness;
        }
    }
    sorted.last().expect("samples are nonempty").0
}

/// Linear interpolation across sorted distinct palette lightness values at rank `p * (n - 1)`.
fn palette_quantile(levels: &[f32], p: f32) -> f32 {
    let position = p * (levels.len() - 1) as f32;
    let below = position.floor() as usize;
    let above = (below + 1).min(levels.len() - 1);
    levels[below] + (position - below as f32) * (levels[above] - levels[below])
}

/// Exactly `recolour_analysis`'s tone rule: the image's 1%–99% lightness range fitted into the
/// palette's, with quartiles pulled toward sparse palettes' own levels, and identity kept when
/// either side is flat or the result stays within 1e-4 of identity.
fn tone(samples: &[Sample], palette: &[[f32; 3]]) -> Result<Vec<[f32; 2]>, TryReserveError> {
    let identity = vec![[0.0, 0.0], [1.0, 1.0]];
    let mut sorted: Vec<(f32, f32)> = Vec::new();
    sorted.try_reserve_exact(samples.len())?;
    sorted.extend(
        samples
            .iter()
            .map(|sample| (sample.opponent[0], sample.weight)),
    );
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total: f32 = sorted.iter().map(|(_, weight)| weight).sum();
    let xs = TONE_QUANTILES.map(|p| quantile(&sorted, total, p).clamp(0.0, 1.0));
    let mut levels: Vec<f32> = Vec::new();
    levels.try_reserve_exact(palette.len())?;
    levels.extend(palette.iter().map(|color| color[0].clamp(0.0, 1.0)));
    levels.sort_by(f32::total_cmp);
    levels.dedup();
    let (low, high) = (xs[0], xs[4]);
    let (palette_low, palette_high) = (levels[0], levels[levels.len() - 1]);
    if high - low < MIN_GAP || palette_high - palette_low < MIN_GAP {
        return Ok(identity);
    }
    let (mut target_low, mut target_high) = (
        low.clamp(palette_low, palette_high),
        high.clamp(palette_low, palette_high),
    );
    if target_high - target_low < MIN_GAP {
        (target_low, target_high) = (palette_low, palette_high);
    }
    let linear = |x: f32| {
        (target_low + (x - low) / (high - low) * (target_high - target_low))
            .clamp(palette_low, palette_high)
    };
    let share = PALETTE_TONE_SHARE * (SPARSE_LEVELS / levels.len() as f32).min(1.0);
    let ranks = [0.0, 0.25, 0.5, 0.75, 1.0];
    let mut points = Vec::new();
    points.try_reserve_exact(TONE_QUANTILES.len() + 2)?;
    points.push([0.0, linear(0.0)]);
    points.extend(xs.iter().zip(ranks).map(|(&x, rank)| {
        let fitted = linear(x);
        [
            x,
            fitted + share * (palette_quantile(&levels, rank) - fitted),
        ]
    }));
    points.push([1.0, linear(1.0)]);
    let mut curve: Vec<[f32; 2]> = Vec::new();
    curve.try_reserve_exact(points.len())?;
    for [x, y] in points {
        match curve.last() {
            Some(&[last_x, last_y]) if x - last_x >= MIN_GAP => {
                curve.push([x, y.max(last_y).clamp(0.0, 1.0)])
            }
            Some(_) => {}
            None => curve.push([x, y.clamp(0.0, 1.0)]),
        }
    }
    if curve.last().is_some_and(|&[x, _]| x < 1.0) {
        // The 99% quantile sat within one gap of 1; extend flat so the curve spans [0, 1].
        let &[_, y] = curve.last().unwrap();
        curve.pop();
        curve.push([1.0, y]);
    }
    if curve.iter().all(|[x, y]| (x - y).abs() < 1e-4) {
        return Ok(identity);
    }
    Ok(curve)
}

/// How far the palette's convex hull reaches from neutral in hue direction `degrees`.
/// Negative when every palette colour lies on the other side of neutral.
fn reach(palette: &[[f32; 3]], degrees: f32) -> f32 {
    let (sin, cos) = degrees.to_radians().sin_cos();
    palette
        .iter()
        .map(|color| color[1] * cos + color[2] * sin)
        .fold(f32::NEG_INFINITY, f32::max)
}

/// The hull's reach at one lightness: the support function of the slice through it at `l`,
/// which spans every palette colour at `l` plus every segment between colours bracketing it.
fn slice_reach(palette: &[[f32; 3]], l: f32, degrees: f32) -> f32 {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let low = palette
        .iter()
        .map(|color| color[0])
        .fold(f32::INFINITY, f32::min);
    let high = palette
        .iter()
        .map(|color| color[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let l = l.clamp(low, high);
    let mut best = f32::NEG_INFINITY;
    // Palette colours at exactly this lightness.
    for color in palette {
        if color[0] == l {
            best = best.max(color[1] * cos + color[2] * sin);
        }
    }
    // Points where segments between lightness levels cross the slice.
    for (i, a) in palette.iter().enumerate() {
        for b in palette.iter().skip(i + 1) {
            if (a[0] < l && l < b[0]) || (b[0] < l && l < a[0]) {
                let (low_end, high_end) = if a[0] < b[0] { (a, b) } else { (b, a) };
                let t = (l - low_end[0]) / (high_end[0] - low_end[0]);
                let u = low_end[1] + t * (high_end[1] - low_end[1]);
                let v = low_end[2] + t * (high_end[2] - low_end[2]);
                best = best.max(u * cos + v * sin);
            }
        }
    }
    best
}

/// Zero when neutral is inside the palette hull (checked every 5 degrees). Otherwise greys cannot
/// be mixed, so colours move part of the way toward the palette centroid, at most `MAX_SHIFT_LENGTH`.
fn shift(palette: &[[f32; 3]]) -> [f32; 2] {
    if (0..72).all(|step| reach(palette, step as f32 * 5.0) >= 0.0) {
        return [0.0, 0.0];
    }
    let count = palette.len() as f32;
    let centroid = [
        palette.iter().map(|color| color[1]).sum::<f32>() / count,
        palette.iter().map(|color| color[2]).sum::<f32>() / count,
    ];
    let length = centroid[0].hypot(centroid[1]) * SHIFT_SHARE;
    let scale = SHIFT_SHARE * (MAX_SHIFT_LENGTH / length).min(1.0);
    centroid.map(|axis| axis * scale)
}

/// Coloured mass and mean post-shift chroma per cell of the hue-by-lightness grid,
/// indexed `[column][row]`, with column and total sums.
struct Cells {
    mass: [[f32; ROWS]; COLUMNS],
    chroma: [[f32; ROWS]; COLUMNS],
    column_mass: [f32; COLUMNS],
    column_chroma: [f32; COLUMNS],
    /// Total coloured mass; a grey image has none.
    total: f32,
}

/// Each sample lands in every column and row by its weights: a raised cosine of half-width 30
/// degrees on its post-shift hue, a tent of half-width 0.25 on its source lightness, times its
/// alpha weight and the neutral ramp on its post-shift chroma.
fn cells(samples: &[Sample], shift: [f32; 2]) -> Cells {
    let mut cells = Cells {
        mass: [[0.0; ROWS]; COLUMNS],
        chroma: [[0.0; ROWS]; COLUMNS],
        column_mass: [0.0; COLUMNS],
        column_chroma: [0.0; COLUMNS],
        total: 0.0,
    };
    for sample in samples {
        let u = sample.opponent[1] + shift[0];
        let v = sample.opponent[2] + shift[1];
        let chroma = u.hypot(v);
        let ramp = (chroma / NEUTRAL_CHROMA).min(1.0);
        if ramp == 0.0 {
            continue;
        }
        let hue = v.atan2(u).to_degrees().rem_euclid(360.0);
        let lightness = sample.opponent[0].clamp(0.0, 1.0);
        for (j, column) in cells.mass.iter_mut().enumerate() {
            let weight = window(
                hue,
                &Group {
                    hue: j as f32 * COLUMN_DEGREES,
                    width: COLUMN_DEGREES,
                    turn: 0.0,
                    chroma: 1.0,
                },
            ) * ramp
                * sample.weight;
            if weight == 0.0 {
                continue;
            }
            for (r, cell) in column.iter_mut().enumerate() {
                let tent = (1.0 - (lightness - r as f32 * ROW_STEP).abs() / ROW_STEP).max(0.0);
                let cell_weight = weight * tent;
                *cell += cell_weight;
                cells.chroma[j][r] += cell_weight * chroma;
            }
        }
    }
    for j in 0..COLUMNS {
        for r in 0..ROWS {
            cells.column_mass[j] += cells.mass[j][r];
        }
        cells.column_chroma[j] = if cells.column_mass[j] > 0.0 {
            cells.chroma[j].iter().sum::<f32>() / cells.column_mass[j]
        } else {
            0.0
        };
        for r in 0..ROWS {
            if cells.mass[j][r] > 0.0 {
                cells.chroma[j][r] /= cells.mass[j][r];
            }
        }
        cells.total += cells.column_mass[j];
    }
    cells
}

/// A turn per column: toward the nearest direction the palette can reach, searched in 5-degree
/// steps, positive first on ties. Columns too light to matter stay at 0.
fn turns(cells: &Cells, palette: &[[f32; 3]]) -> [f32; COLUMNS] {
    let mut turns = [0.0; COLUMNS];
    if cells.total == 0.0 {
        return turns;
    }
    for (j, turn) in turns.iter_mut().enumerate() {
        let mass = cells.column_mass[j];
        if mass < MIN_COLUMN_MASS * cells.total {
            continue;
        }
        let target = REACH_SHARE * cells.column_chroma[j];
        let hue = j as f32 * COLUMN_DEGREES;
        if reach(palette, hue) >= target {
            continue;
        }
        *turn = (1..=(MAX_TURN / 5.0) as usize)
            .flat_map(|step| [step as f32 * 5.0, -(step as f32) * 5.0])
            .find(|&turn| reach(palette, hue + turn) >= target)
            .unwrap_or(0.0);
    }
    turns
}

/// A gain per cell: the palette's slice reach at the cell's turned direction over its mean
/// chroma, clamped by the look. Sparse cells borrow their row's mean gain, then everyone's.
fn gains(
    cells: &Cells,
    palette: &[[f32; 3]],
    tone: &[[f32; 2]],
    turns: &[f32; COLUMNS],
    look: FitLook,
) -> Option<[[f32; ROWS]; COLUMNS]> {
    if cells.total == 0.0 {
        return None;
    }
    let cap = gain_cap(look);
    let tone = Spline::new(tone);
    let mut gains = [[0.0; ROWS]; COLUMNS];
    let mut qualifies = [[false; ROWS]; COLUMNS];
    for j in 0..COLUMNS {
        for (r, cell) in gains[j].iter_mut().enumerate() {
            let qualifying =
                cells.mass[j][r] >= MIN_CELL_MASS * cells.total && cells.chroma[j][r] != 0.0;
            qualifies[j][r] = qualifying;
            if qualifying {
                let l = tone.eval(r as f32 * ROW_STEP);
                let direction = j as f32 * COLUMN_DEGREES + turns[j];
                *cell = (slice_reach(palette, l, direction) / cells.chroma[j][r]).clamp(0.0, cap);
            }
        }
    }
    for j in 0..COLUMNS {
        for r in 0..ROWS {
            if qualifies[j][r] {
                continue;
            }
            gains[j][r] = mean_gain(&gains, &qualifies, cells, Some(r))
                .or_else(|| mean_gain(&gains, &qualifies, cells, None))
                .unwrap_or(1.0);
        }
    }
    let mut any = false;
    for j in 0..COLUMNS {
        for r in 0..ROWS {
            // Within 2% of neutral is no change worth a round trip.
            if (gains[j][r] - 1.0).abs() < 0.02 {
                gains[j][r] = 1.0;
            }
            any |= gains[j][r] != 1.0;
        }
    }
    any.then_some(gains)
}

/// Mass-weighted mean of the qualifying cells' gains in `row`, or in the whole grid when `None`.
fn mean_gain(
    gains: &[[f32; ROWS]; COLUMNS],
    qualifies: &[[bool; ROWS]; COLUMNS],
    cells: &Cells,
    row: Option<usize>,
) -> Option<f32> {
    let (mut mass, mut weighted) = (0.0, 0.0);
    for j in 0..COLUMNS {
        for r in 0..ROWS {
            if row.is_some_and(|row| row != r) || !qualifies[j][r] {
                continue;
            }
            mass += cells.mass[j][r];
            weighted += cells.mass[j][r] * gains[j][r];
        }
    }
    (mass > 0.0).then(|| weighted / mass)
}

/// Writes the analysis as a curve list in emission order: tone, shift a, shift b, chroma gain,
/// hue turn. Omitted curves are absent.
fn emit(
    space: FitSpace,
    tone: Vec<[f32; 2]>,
    shift: [f32; 2],
    gains: Option<[[f32; ROWS]; COLUMNS]>,
    turns: [f32; COLUMNS],
) -> Result<Vec<Curve>, TryReserveError> {
    let (lab, lch) = (space.lab(), space.lch());
    let lightness = ColourChannel {
        model: lab,
        channel: ModelChannel::Lightness,
    };
    let hue = ColourChannel {
        model: lch,
        channel: ModelChannel::Hue,
    };
    let chroma = ColourChannel {
        model: lch,
        channel: ModelChannel::Chroma,
    };
    let mut curves = Vec::new();
    curves.try_reserve_exact(5)?;
    if tone != [[0.0, 0.0], [1.0, 1.0]] {
        curves.push(Curve::OneInput(OneInputCurve {
            kind: CurveKind::Remap,
            x: lightness,
            y: lightness,
            points: tone,
        }));
    }
    if shift != [0.0, 0.0] {
        let scale = space.opponent_scale();
        for (name, offset) in [(ModelChannel::A, shift[0]), (ModelChannel::B, shift[1])] {
            let k = 0.5 + offset * scale;
            curves.push(Curve::OneInput(OneInputCurve {
                kind: CurveKind::Adjust,
                x: lightness,
                y: ColourChannel {
                    model: lab,
                    channel: name,
                },
                points: vec![[0.0, k], [1.0, k]],
            }));
        }
    }
    if let Some(gains) = gains {
        curves.push(Curve::TwoInput(TwoInputCurve {
            kind: CurveKind::Adjust,
            x: hue,
            x2: lightness,
            y: chroma,
            grid: CurveGrid {
                columns: (0..COLUMNS).map(|j| j as f32 / COLUMNS as f32).collect(),
                rows: (0..ROWS).map(|r| r as f32 * ROW_STEP).collect(),
                values: (0..ROWS)
                    .map(|r| (0..COLUMNS).map(|j| gains[j][r] / 2.0).collect())
                    .collect(),
            },
        }));
    }
    if turns.iter().any(|&turn| turn != 0.0) {
        let mut points: Vec<[f32; 2]> = Vec::new();
        points.try_reserve_exact(COLUMNS + 1)?;
        points.extend((0..COLUMNS).map(|j| [j as f32 / COLUMNS as f32, 0.5 + turns[j] / 360.0]));
        points.push([1.0, points[0][1]]);
        curves.push(Curve::OneInput(OneInputCurve {
            kind: CurveKind::Adjust,
            x: hue,
            y: hue,
            points,
        }));
    }
    debug_assert!(curves.len() <= 5);
    Ok(curves)
}
