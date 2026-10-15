//! Gradient-only placement over unchanged encoded RGB8. See `smooth_gradient_v1.md`.
use crate::image::{ImageView, Rgba8};

pub mod render;

// Two-byte colour quantization can shift each endpoint by one byte in opposite directions.
const FIT_ERROR: f64 = 2.0;
const CHUNK_CHANGE: i16 = 4;
const MAX_STEP: i16 = 8;
const MAX_SLOPE_RATIO: f64 = 4.0;
const QUANTIZATION_BACKTRACK: i16 = 2;

/// Writes horizontal support, horizontal slope, and the final binary mask into caller-owned scratch.
/// The two scans are linear in pixel count, independent of the requested minimum size.
/// Alpha and padding never enter selection. A failed callback exposes no partial render result.
pub fn select<E>(
    source: ImageView<'_, Rgba8>,
    min_gradient_size: u32,
    scratch: &mut [[f32; 3]],
    mut check: impl FnMut() -> Result<(), E>,
) -> Result<(), E> {
    let width = source.dimensions().width_usize();
    let height = source.dimensions().height_usize();
    assert_eq!(scratch.len(), width * height);
    scratch.fill([0.0; 3]);
    for y in 0..height {
        scan_line(
            width,
            |x| rgb(source, x, y),
            |start, end, slope| {
                for x in start..=end {
                    let span = (end - start + 1) as f32;
                    scratch[y * width + x] = [
                        span,
                        slope,
                        if span >= min_gradient_size as f32 {
                            1.0
                        } else {
                            0.0
                        },
                    ];
                }
            },
        );
        check()?;
    }
    for x in 0..width {
        scan_line(
            height,
            |y| rgb(source, x, y),
            |start, end, slope| {
                for y in start..=end {
                    let cell = &mut scratch[y * width + x];
                    let norm = f64::from(cell[1]).hypot(f64::from(slope));
                    let vertical_span = (end - start + 1) as f32;
                    let (span, dominant) = if cell[1] > slope {
                        (cell[0], cell[1])
                    } else if cell[1] == slope {
                        // Equal slopes have no preferred axis, including in rectangular crops.
                        (cell[0].max(vertical_span), slope)
                    } else {
                        (vertical_span, slope)
                    };
                    let size = f64::from(span) * f64::from(dominant) / norm;
                    cell[2] = if size >= f64::from(min_gradient_size) {
                        1.0
                    } else {
                        0.0
                    };
                }
            },
        );
        check()?;
    }
    for y in 0..height {
        for x in 0..width {
            let cell = &mut scratch[y * width + x];
            cell[2] = if cell[2] > 0.0 && locally_smooth(source, x, y) {
                1.0
            } else {
                0.0
            };
        }
        check()?;
    }
    Ok(())
}

fn rgb(source: ImageView<'_, Rgba8>, x: usize, y: usize) -> [i16; 3] {
    let pixel = source
        .pixel(x as u32, y as u32)
        .expect("in-bounds source pixel");
    [
        i16::from(pixel[0]),
        i16::from(pixel[1]),
        i16::from(pixel[2]),
    ]
}

fn difference(a: [i16; 3], b: [i16; 3]) -> [i16; 3] {
    std::array::from_fn(|axis| a[axis] - b[axis])
}

fn magnitude(v: [f64; 3]) -> f64 {
    v.into_iter().map(|v| v * v).sum::<f64>().sqrt()
}

// Fit short colour intervals, then join coherent intervals. Fitting the entire run would
// turn a larger spatial threshold into an unintended stricter curvature threshold.
fn scan_line(
    length: usize,
    pixel: impl Fn(usize) -> [i16; 3],
    mut emit: impl FnMut(usize, usize, f32),
) {
    let mut start = 0;
    let mut first_change = None;
    let mut last_change = 0;
    let mut chunk = 0;
    let mut lower = [f64::NEG_INFINITY; 3];
    let mut upper = [f64::INFINITY; 3];
    let mut direction = [0; 3];
    let mut extreme = pixel(0);
    let mut previous_slope: Option<[f64; 3]> = None;
    let finish = |start: usize,
                  first: Option<usize>,
                  last: usize,
                  emit: &mut dyn FnMut(usize, usize, f32)| {
        let Some(first) = first else {
            return;
        };
        let begin = first.max(start);
        let delta = difference(pixel(last), pixel(begin));
        if last < begin + 3 || delta.iter().all(|v| v.abs() < CHUNK_CHANGE) {
            return;
        }
        let slope = magnitude(delta.map(f64::from)) / (last - begin) as f64;
        emit(begin, last, slope as f32);
    };
    for i in 1..length {
        let color = pixel(i);
        let step = difference(pixel(i), pixel(i - 1));
        let delta = difference(pixel(i), pixel(chunk));
        let distance = (i - chunk) as f64;
        let next_lower = std::array::from_fn(|axis| {
            lower[axis].max((f64::from(delta[axis]) - FIT_ERROR) / distance)
        });
        let next_upper = std::array::from_fn(|axis| {
            upper[axis].min((f64::from(delta[axis]) + FIT_ERROR) / distance)
        });
        let invalid = (0..3).any(|axis| {
            step[axis].abs() > MAX_STEP
                || (direction[axis] != 0
                    && (color[axis] - extreme[axis]) * direction[axis] < -QUANTIZATION_BACKTRACK)
                || next_lower[axis] > next_upper[axis]
        });
        if invalid {
            finish(start, first_change, last_change, &mut emit);
            // A sharp step belongs to neither neighboring gradient.
            start = if step.iter().any(|v| v.abs() > MAX_STEP) {
                i
            } else {
                i - 1
            };
            chunk = start;
            first_change = None;
            last_change = start;
            direction = [0; 3];
            extreme = pixel(start);
            lower = [f64::NEG_INFINITY; 3];
            upper = [f64::INFINITY; 3];
            previous_slope = None;
        } else {
            lower = next_lower;
            upper = next_upper;
        }
        if i <= start {
            continue;
        }
        if step != [0; 3] {
            first_change.get_or_insert(i - 1);
            last_change = i;
        }
        // A byte-level reversal is quantization noise until it exceeds the source tolerance.
        // Lock direction from an actual colour interval, rather than its first noisy step.
        let run_delta = difference(color, pixel(start));
        for axis in 0..3 {
            if direction[axis] == 0 && run_delta[axis].abs() >= CHUNK_CHANGE {
                direction[axis] = run_delta[axis].signum();
            }
            extreme[axis] = match direction[axis] {
                1 => extreme[axis].max(color[axis]),
                -1 => extreme[axis].min(color[axis]),
                _ => color[axis],
            };
        }
        let delta = difference(pixel(i), pixel(chunk));
        if delta.iter().any(|v| v.abs() >= CHUNK_CHANGE) {
            let slope = delta.map(|v| f64::from(v) / (i - chunk) as f64);
            if let Some(previous) = previous_slope {
                let a = magnitude(previous);
                let b = magnitude(slope);
                let dot = (0..3).map(|axis| previous[axis] * slope[axis]).sum::<f64>();
                if a > b * MAX_SLOPE_RATIO
                    || b > a * MAX_SLOPE_RATIO
                    || dot < a * b * std::f64::consts::FRAC_1_SQRT_2
                {
                    finish(start, first_change, chunk, &mut emit);
                    start = chunk;
                    first_change = Some(chunk);
                }
            }
            previous_slope = Some(slope);
            chunk = i;
            lower = [f64::NEG_INFINITY; 3];
            upper = [f64::INFINITY; 3];
        }
    }
    finish(start, first_change, last_change, &mut emit);
}

// A fixed one-pixel guard rejects boundaries in any direction. It never grows with minimum size.
fn locally_smooth(source: ImageView<'_, Rgba8>, x: usize, y: usize) -> bool {
    let width = source.dimensions().width_usize();
    let height = source.dimensions().height_usize();
    let x0 = x.saturating_sub(1);
    let x1 = (x + 1).min(width - 1);
    let y0 = y.saturating_sub(1);
    let y1 = (y + 1).min(height - 1);
    let center = rgb(source, x, y);
    let dx = difference(rgb(source, x1, y), rgb(source, x0, y))
        .map(|v| f64::from(v) / (x1 - x0).max(1) as f64);
    let dy = difference(rgb(source, x, y1), rgb(source, x, y0))
        .map(|v| f64::from(v) / (y1 - y0).max(1) as f64);
    for ny in y0..=y1 {
        for nx in x0..=x1 {
            let color = rgb(source, nx, ny);
            for axis in 0..3 {
                if dx[axis].abs() > f64::from(MAX_STEP) || dy[axis].abs() > f64::from(MAX_STEP) {
                    return false;
                }
                let expected = f64::from(center[axis])
                    + dx[axis] * (nx as f64 - x as f64)
                    + dy[axis] * (ny as f64 - y as f64);
                if (f64::from(color[axis]) - expected).abs() > FIT_ERROR {
                    return false;
                }
            }
        }
    }
    true
}
