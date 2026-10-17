//! Production copy of the area selector specified in `spec/smooth_area_v1.md`.

use crate::image::{ImageDimensions, ImageView, Rgba8};
use crate::prod::color::packed::{Converter, PackedSpace};

pub(crate) mod coverage;

const EDGE_PROBE_RADIUS: usize = 4;
const MINIMUM_REGION_DIVISOR: usize = 16;

// Evaluate the cubic before rounding to f32. Intermediate f32 rounding can
// decrease smoothstep near one, removing pixels as tolerance increases.
fn smoothstep(t: f32) -> f32 {
    let t = f64::from(t);
    (t * t * (3.0 - 2.0 * t)) as f32
}

/// Pixel records plus three source colour rows, reused as one line queue.
pub fn scratch_len(dimensions: ImageDimensions) -> usize {
    dimensions.pixel_count().expect("validated dimensions")
        + (3 * dimensions.width_usize()).max(dimensions.height_usize())
}

/// Reconstructs feathered, source-aware regions from squares meeting fine/coarse Oklab contrast.
/// Total colour change across a smooth ramp is unrestricted. Flat plateaus stay included.
pub fn select<E>(
    source: ImageView<'_, Rgba8>,
    sample_size: u32,
    contrast_threshold: f32,
    scratch: &mut [[f32; 3]],
    mut check: impl FnMut() -> Result<(), E>,
) -> Result<(), E> {
    let dimensions = source.dimensions();
    let width = dimensions.width_usize();
    let height = dimensions.height_usize();
    assert_eq!(scratch.len(), scratch_len(dimensions));
    let (cells, queue) = scratch.split_at_mut(width * height);
    // The first pass overwrites every record, including reused scratch.
    let sample_width = (sample_size as usize).min(width);
    let sample_height = (sample_size as usize).min(height);
    let columns = width - sample_width + 1;
    let rows = height - sample_height + 1;
    let radius = sample_size as usize / 2;
    let converter = Converter::new(PackedSpace::Oklab);
    // Quantized ramps and flat fills repeat RGB bytes. Reuse the immediately
    // preceding conversion in registers; nothing survives this selector call.
    let mut previous_rgb = u32::MAX;
    let mut previous_coordinates = [0.0; 3];
    let mut convert = |rgb: [u8; 3]| {
        let key = u32::from_le_bytes([rgb[0], rgb[1], rgb[2], 0]);
        if key != previous_rgb {
            previous_coordinates = converter.coordinates(rgb);
            previous_rgb = key;
        }
        previous_coordinates
    };
    for y in 0..height {
        let row = source.row(y as u32).expect("validated row");
        for (pixel, target) in row
            .chunks_exact(4)
            .zip(&mut cells[y * width..(y + 1) * width])
        {
            *target = convert([pixel[0], pixel[1], pixel[2]]);
        }
        check()?;
    }
    mark_boundaries(dimensions, cells, &mut check)?;
    // The coarse colour is a symmetric 2D box mean. A compact transition differs
    // from its coarse colour; a broad locally linear ramp largely cancels out.
    for y in 0..height {
        average_line(dimensions, false, y, radius, cells, queue);
        check()?;
    }
    for x in 0..width {
        average_line(dimensions, true, x, radius, cells, queue);
        check()?;
    }
    // The one-pixel read halo makes a strong edge fail every containing footprint,
    // including squares ending immediately beside it. It never grows with sample size.
    let mut next_row = 0;
    for y in 0..height {
        // Reconvert each original row once, so coarse means need no second image plane.
        while next_row <= (y + 1).min(height - 1) {
            let row = source.row(next_row as u32).expect("validated row");
            let slot = next_row % 3;
            for (pixel, target) in row
                .chunks_exact(4)
                .zip(&mut queue[slot * width..(slot + 1) * width])
            {
                *target = convert([pixel[0], pixel[1], pixel[2]]);
            }
            next_row += 1;
            check()?;
        }
        for x in 0..width {
            let center = queue[(y % 3) * width + x];
            // Twice the residual uses the full jump magnitude at a symmetric step.
            let mut contrast =
                (2.0 * color_distance(center, unsigned(cells[y * width + x]))).min(1.0);
            let mut fine = 0.0_f32;
            visit_square_neighbours(width, height, x, y, 1, |nx, ny| {
                fine = fine.max(color_distance_squared(center, queue[(ny % 3) * width + nx]));
                true
            });
            // sqrt is monotone on these finite nonnegative sums. Taking it
            // after the maximum keeps the exact f32 result, once per pixel.
            contrast = contrast.max(fine.sqrt().min(1.0));
            cells[y * width + x][1] = if cells[y * width + x][0].is_sign_negative() {
                -contrast - 1.0
            } else {
                contrast
            };
            cells[y * width + x][2] = contrast;
        }
        check()?;
    }
    for y in 0..height {
        maximum_line(dimensions, false, y, sample_width, cells, queue);
        check()?;
    }
    for x in 0..columns {
        maximum_line(dimensions, true, x, sample_height, cells, queue);
        check()?;
    }
    // Qualifying centres remain the accepted detector's seeds. Small unsupported
    // components are removed before source-aware reconstruction, rather than expanded.
    for y in 0..height {
        let top = y.saturating_sub((sample_height - 1) / 2).min(rows - 1);
        for x in 0..width {
            let left = x.saturating_sub((sample_width - 1) / 2).min(columns - 1);
            cells[y * width + x][0] = if cells[top * width + left][2] <= contrast_threshold {
                1.0
            } else {
                0.0
            };
        }
        check()?;
    }
    // A coarse test that reads across lineart can reject its smooth side.
    // Only this hard-edge shadow may bypass local rejection; source-colour
    // barriers below still prevent coverage from crossing the line itself.
    let edge_radius = (sample_size as f32 / 2.0).max(1.0);
    for row in cells.chunks_exact_mut(width) {
        for cell in row {
            cell[2] = if cell[1] < 0.0 {
                0.0
            } else {
                edge_radius + 1.0
            };
        }
        check()?;
    }
    geometric_distance(dimensions, cells, &mut check)?;
    for row in cells.chunks_exact_mut(width) {
        for cell in row {
            let contrast = if cell[1] < 0.0 {
                -cell[1] - 1.0
            } else {
                cell[1]
            };
            let allowed = contrast <= contrast_threshold || cell[2] <= edge_radius;
            cell[1] = if allowed { 1.0 } else { -1.0 };
            cell[2] = if allowed { edge_radius } else { 0.0 };
        }
        check()?;
    }
    // Feather genuine regional/texture rejection, rather than eroding the
    // gradient side of a hard source boundary.
    geometric_distance(dimensions, cells, &mut check)?;
    for row in cells.chunks_exact_mut(width) {
        for cell in row {
            if cell[1] >= 0.0 {
                let t = (cell[2] / edge_radius).clamp(0.0, 1.0);
                cell[1] = smoothstep(t);
            }
        }
        check()?;
    }
    let reach = sample_size as f32;
    retain_regions(
        dimensions,
        (sample_width * sample_height).div_ceil(MINIMUM_REGION_DIVISOR),
        reach * 2.0,
        cells,
        &mut check,
    )?;
    for direction in [(false, false), (true, true), (false, true), (true, false)] {
        spread(
            source,
            contrast_threshold,
            reach,
            direction,
            cells,
            queue,
            &mut convert,
            &mut check,
        )?;
    }
    for row in cells.chunks_exact_mut(width) {
        for cell in row {
            let t = ((2.0 * reach - cell[2]) / reach).clamp(0.0, 1.0);
            cell[2] = if cell[1] < 0.0 {
                0.0
            } else {
                cell[1] * smoothstep(t)
            };
        }
        check()?;
    }
    Ok(())
}

/// Fixed blue-noise coverage keeps each chosen algorithm's complete pixel and phase.
/// Zero coverage remains plain; full coverage retains the ordinary field or diffusion output.
pub fn covers(mask: f32, x: u32, y: u32) -> bool {
    // Separate coverage from the blue-noise field itself.
    const OFFSET: [u32; 2] = [11, 17];
    mask > super::blue_noise::blue_noise_at(y.wrapping_add(OFFSET[0]), x.wrapping_add(OFFSET[1]))
        + 0.5
}

// Queue indices occupy one dead lane as u32 bits, never floating arithmetic.
// Validated dimensions have at most 2^30 pixels; their index bit patterns are finite.
pub(crate) fn retain_regions<E>(
    dimensions: ImageDimensions,
    minimum: usize,
    limit: f32,
    cells: &mut [[f32; 3]],
    check: &mut impl FnMut() -> Result<(), E>,
) -> Result<(), E> {
    let width = dimensions.width_usize();
    let height = dimensions.height_usize();
    for row in cells.chunks_exact_mut(width) {
        for cell in row {
            cell[2] = if cell[0] == 1.0 { 1.0 } else { -1.0 };
        }
        check()?;
    }
    for y in 0..height {
        for x in 0..width {
            if cells[y * width + x][2] != 1.0 {
                continue;
            }
            cells[y * width + x][2] = -2.0;
            cells[0][0] = f32::from_bits((y * width + x) as u32);
            let mut head = 0;
            let mut tail = 1;
            while head < tail {
                let point = cells[head][0].to_bits() as usize;
                let cx = point % width;
                let cy = point / width;
                head += 1;
                for ny in cy.saturating_sub(1)..=(cy + 1).min(height - 1) {
                    for nx in cx.saturating_sub(1)..=(cx + 1).min(width - 1) {
                        if cells[ny * width + nx][2] != 1.0 {
                            continue;
                        }
                        cells[ny * width + nx][2] = -2.0;
                        cells[tail][0] = f32::from_bits((ny * width + nx) as u32);
                        tail += 1;
                    }
                }
                if head % width == 0 {
                    check()?;
                }
            }
            let tag = if tail >= minimum { -3.0 } else { -1.0 };
            for p in 0..tail {
                let point = cells[p][0].to_bits() as usize;
                cells[point][2] = tag;
                if p % width == 0 {
                    check()?;
                }
            }
        }
        check()?;
    }
    for row in cells.chunks_exact_mut(width) {
        for cell in row {
            cell[2] = if cell[1] < 0.0 {
                -1.0
            } else if cell[2] == -3.0 {
                0.0
            } else {
                limit
            };
        }
        check()?;
    }
    Ok(())
}

// Source-independent eight-neighbour distance is used only to identify an edge
// shadow and soften regional boundaries. Image borders are not rejection sites.
pub(crate) fn geometric_distance<E>(
    dimensions: ImageDimensions,
    cells: &mut [[f32; 3]],
    check: &mut impl FnMut() -> Result<(), E>,
) -> Result<(), E> {
    let width = dimensions.width_usize();
    let height = dimensions.height_usize();
    for reverse in [false, true] {
        for sy in 0..height {
            let y = if reverse { height - 1 - sy } else { sy };
            for sx in 0..width {
                let x = if reverse { width - 1 - sx } else { sx };
                let mut value = cells[y * width + x][2];
                if value == 0.0 {
                    continue;
                }
                visit_neighbours(x, y, reverse, reverse, |nx, ny, step| {
                    if nx >= width || ny >= height {
                        return;
                    }
                    let candidate = cells[ny * width + nx][2] + step;
                    if candidate < value {
                        value = candidate;
                    }
                });
                cells[y * width + x][2] = value;
            }
            check()?;
        }
    }
    Ok(())
}

// Four bounded chamfer sweeps reconstruct rounded contours along smooth source
// colours. Hard colour jumps are barriers; local variation makes texture costly.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spread<E>(
    source: ImageView<'_, Rgba8>,
    tolerance: f32,
    reach: f32,
    (reverse_y, reverse_x): (bool, bool),
    cells: &mut [[f32; 3]],
    rows: &mut [[f32; 3]],
    convert: &mut impl FnMut([u8; 3]) -> [f32; 3],
    check: &mut impl FnMut() -> Result<(), E>,
) -> Result<(), E> {
    let width = source.dimensions().width_usize();
    let height = source.dimensions().height_usize();
    let mut tags = [usize::MAX; 3];
    for step_y in 0..height {
        let y = if reverse_y {
            height - 1 - step_y
        } else {
            step_y
        };
        for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
            if tags[ny % 3] == ny {
                continue;
            }
            // Keep the existing three-row colour workspace, but convert only
            // pixels reached by an unsettled path. Seeds need no colour work.
            for target in &mut rows[(ny % 3) * width..(ny % 3 + 1) * width] {
                target[0] = f32::NAN;
            }
            tags[ny % 3] = ny;
            check()?;
        }
        for step_x in 0..width {
            let x = if reverse_x {
                width - 1 - step_x
            } else {
                step_x
            };
            // Seed distance is already the nonnegative path minimum. Avoid
            // revisiting its neighbours in every reconstruction sweep.
            if cells[y * width + x][2] <= 0.0 {
                continue;
            }
            let mut distance = cells[y * width + x][2];
            visit_neighbours(x, y, reverse_x, reverse_y, |nx, ny, step| {
                if nx >= width || ny >= height {
                    return;
                }
                let neighbour = cells[ny * width + nx][2];
                let diagonal = nx != x && ny != y;
                // Colour variation only increases this geometric lower bound.
                // Skip its conversions and square roots when even the cheapest
                // possible path cannot improve the current distance.
                if neighbour < 0.0 || neighbour + step >= distance {
                    return;
                }
                // An unsettled pixel may still have no competitive incoming
                // path. Convert its source only after that cheap rejection.
                let centre = source_color(source, rows, x, y, convert);
                let jump =
                    color_distance(centre, source_color(source, rows, nx, ny, convert)).min(1.0);
                if jump > tolerance {
                    return;
                }
                if diagonal
                    && (color_distance(centre, source_color(source, rows, nx, y, convert)).min(1.0)
                        > tolerance
                        || color_distance(centre, source_color(source, rows, x, ny, convert))
                            .min(1.0)
                            > tolerance)
                {
                    return;
                }
                let variation = if tolerance == 0.0 {
                    0.0
                } else {
                    jump / (step * tolerance)
                };
                let cost = step * (1.0 + reach * variation * variation);
                let candidate = neighbour + cost;
                if candidate < distance {
                    distance = candidate;
                }
            });
            cells[y * width + x][2] = distance;
        }
        check()?;
    }
    Ok(())
}

#[inline(always)]
fn source_color(
    source: ImageView<'_, Rgba8>,
    rows: &mut [[f32; 3]],
    x: usize,
    y: usize,
    convert: &mut impl FnMut([u8; 3]) -> [f32; 3],
) -> [f32; 3] {
    let index = (y % 3) * source.dimensions().width_usize() + x;
    if rows[index][0].is_nan() {
        let row = source.row(y as u32).expect("validated row");
        let start = x * 4;
        rows[index] = convert([row[start], row[start + 1], row[start + 2]]);
    }
    rows[index]
}

// Underflow maps outside the validated image. Each caller performs one bounds
// test, instead of repeating signed overflow checks for every neighbour.
#[inline(always)]
fn visit_neighbours(
    x: usize,
    y: usize,
    reverse_x: bool,
    reverse_y: bool,
    mut visit: impl FnMut(usize, usize, f32),
) {
    let (before, after) = if reverse_x {
        (x + 1, x.wrapping_sub(1))
    } else {
        (x.wrapping_sub(1), x + 1)
    };
    let previous = if reverse_y { y + 1 } else { y.wrapping_sub(1) };
    // Four explicit visits avoid a temporary neighbour array and its loop in
    // the size-optimized Wasm build. Keep the frozen relaxation order.
    visit(before, y, 1.0);
    visit(before, previous, std::f32::consts::SQRT_2);
    visit(x, previous, 1.0);
    visit(after, previous, std::f32::consts::SQRT_2);
}

#[inline(always)]
fn color_distance_squared(left: [f32; 3], right: [f32; 3]) -> f32 {
    let dl = left[0] - right[0];
    let da = left[1] - right[1];
    let db = left[2] - right[2];
    dl * dl + da * da + db * db
}

// This tiny kernel dominates the public full-image profile. Keep the arithmetic
// inline instead of copying both triplets through the size-optimized Wasm ABI.
#[inline(always)]
fn color_distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    color_distance_squared(left, right).sqrt()
}

#[inline(always)]
fn unsigned([l, a, b]: [f32; 3]) -> [f32; 3] {
    [l.abs(), a, b]
}

// A concentrated one/two-pixel jump supplies a boundary witness. This support
// test is independent of tolerance, preserving monotonic inclusion as tolerance
// rises. The sign of nonnegative Oklab L carries the bit through coarse means.
fn mark_boundaries<E>(
    dimensions: ImageDimensions,
    cells: &mut [[f32; 3]],
    check: &mut impl FnMut() -> Result<(), E>,
) -> Result<(), E> {
    let width = dimensions.width_usize();
    let height = dimensions.height_usize();
    for y in 0..height {
        let mut left = column_range(width, height, 0, y, cells);
        let mut middle = left;
        for x in 0..width {
            let right = column_range(width, height, (x + 1).min(width - 1), y, cells);
            let (mut minimum, mut maximum) = left;
            extend_bounds(&mut minimum, &mut maximum, middle.0, middle.1);
            extend_bounds(&mut minimum, &mut maximum, right.0, right.1);
            left = middle;
            middle = right;
            let centre = unsigned(cells[y * width + x]);
            let mut fine = 0.0_f32;
            visit_square_neighbours(width, height, x, y, 1, |nx, ny| {
                let color = unsigned(cells[ny * width + nx]);
                let distance = color_distance_squared(centre, color);
                if distance > fine {
                    fine = distance;
                }
                true
            });
            // A constant fine neighbourhood cannot witness a concentrated edge.
            // Its broader colour range cannot change that decision.
            if fine == 0.0 {
                continue;
            }
            let concentrated = 2.0 * fine.sqrt();
            let concentrated_edge =
                visit_square_neighbours(width, height, x, y, EDGE_PROBE_RADIUS, |nx, ny| {
                    let color = unsigned(cells[ny * width + nx]);
                    extend_range(&mut minimum, &mut maximum, color);
                    // The enclosing colour range can only grow. A smooth
                    // neighbourhood often rules out an edge on the first
                    // coarse probe, so its remaining probes add no information.
                    color_distance(minimum, maximum) <= concentrated
                });
            if concentrated_edge {
                cells[y * width + x][0] = -centre[0];
            }
        }
        check()?;
    }
    Ok(())
}

// Adjacent 3x3 windows share two columns. Carry each column's exact bounds
// forward instead of rebuilding all nine pixels' bounds for every centre.
#[inline(always)]
fn column_range(
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    cells: &[[f32; 3]],
) -> ([f32; 3], [f32; 3]) {
    let mut minimum = unsigned(cells[y.saturating_sub(1) * width + x]);
    let mut maximum = minimum;
    extend_range(&mut minimum, &mut maximum, unsigned(cells[y * width + x]));
    extend_range(
        &mut minimum,
        &mut maximum,
        unsigned(cells[(y + 1).min(height - 1) * width + x]),
    );
    (minimum, maximum)
}

// Expose the fixed stencil to the size-optimized compiler. The centre already
// belongs to each caller's initial bounds. False stops the coarse probe early.
#[inline(always)]
fn visit_square_neighbours(
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    radius: usize,
    mut visit: impl FnMut(usize, usize) -> bool,
) -> bool {
    let left = x.saturating_sub(radius);
    let right = (x + radius).min(width - 1);
    let top = y.saturating_sub(radius);
    let bottom = (y + radius).min(height - 1);
    visit(left, top)
        && visit(x, top)
        && visit(right, top)
        && visit(left, y)
        && visit(right, y)
        && visit(left, bottom)
        && visit(x, bottom)
        && visit(right, bottom)
}

// Oklab coordinates from encoded RGB are finite. Ordered comparisons avoid
// generic min/max's NaN handling in this full-image neighbourhood kernel.
#[inline(always)]
fn extend_range(minimum: &mut [f32; 3], maximum: &mut [f32; 3], color: [f32; 3]) {
    extend_bounds(minimum, maximum, color, color);
}

#[inline(always)]
fn extend_bounds(minimum: &mut [f32; 3], maximum: &mut [f32; 3], lower: [f32; 3], upper: [f32; 3]) {
    // Spell out three channels: the frozen size-oriented compiler otherwise
    // retains a loop here for every neighbour of every image pixel.
    *minimum = [
        if lower[0] < minimum[0] {
            lower[0]
        } else {
            minimum[0]
        },
        if lower[1] < minimum[1] {
            lower[1]
        } else {
            minimum[1]
        },
        if lower[2] < minimum[2] {
            lower[2]
        } else {
            minimum[2]
        },
    ];
    *maximum = [
        if upper[0] > maximum[0] {
            upper[0]
        } else {
            maximum[0]
        },
        if upper[1] > maximum[1] {
            upper[1]
        } else {
            maximum[1]
        },
        if upper[2] > maximum[2] {
            upper[2]
        } else {
            maximum[2]
        },
    ];
}

// Symmetrically shrink the box at image boundaries, so truncation cannot create
// a false residual on a linear ramp. Both window endpoints only move forward.
fn average_line(
    dimensions: ImageDimensions,
    vertical: bool,
    line: usize,
    radius: usize,
    cells: &mut [[f32; 3]],
    queue: &mut [[f32; 3]],
) {
    let width = dimensions.width_usize();
    let length = if vertical {
        dimensions.height_usize()
    } else {
        width
    };
    let index = |position| {
        if vertical {
            position * width + line
        } else {
            line * width + position
        }
    };
    for position in 0..length {
        queue[position] = unsigned(cells[index(position)]);
    }
    let mut sum = [0.0_f64; 3];
    let mut left = 0;
    let mut right = 0;
    for position in 0..length {
        let span = radius.min(position).min(length - 1 - position);
        let start = position - span;
        let end = position + span + 1;
        while right < end {
            let color = queue[right];
            sum[0] += f64::from(color[0]);
            sum[1] += f64::from(color[1]);
            sum[2] += f64::from(color[2]);
            right += 1;
        }
        while left < start {
            let color = queue[left];
            sum[0] -= f64::from(color[0]);
            sum[1] -= f64::from(color[1]);
            sum[2] -= f64::from(color[2]);
            left += 1;
        }
        let marked = cells[index(position)][0].is_sign_negative();
        let length = (end - start) as f64;
        let mut mean = [
            (sum[0] / length) as f32,
            (sum[1] / length) as f32,
            (sum[2] / length) as f32,
        ];
        if marked {
            mean[0] = -mean[0];
        }
        cells[index(position)] = mean;
    }
}

// Each line index enters and leaves the monotone queue once. Validated image
// sides are at most 32768, so the f32 queue indices are exact integers.
fn maximum_line(
    dimensions: ImageDimensions,
    vertical: bool,
    line: usize,
    span: usize,
    cells: &mut [[f32; 3]],
    queue: &mut [[f32; 3]],
) {
    let width = dimensions.width_usize();
    let length = if vertical {
        dimensions.height_usize()
    } else {
        width
    };
    let mut head = 0;
    let mut tail = 0;
    for position in 0..length {
        let current = line_value(cells, width, vertical, line, position);
        while head < tail && queue[head][0] as usize + span <= position {
            head += 1;
        }
        while head < tail
            && line_value(cells, width, vertical, line, queue[tail - 1][0] as usize) <= current
        {
            tail -= 1;
        }
        queue[tail][0] = position as f32;
        tail += 1;
        if position + 1 < span {
            continue;
        }
        let maximum = line_value(cells, width, vertical, line, queue[head][0] as usize);
        let start = position + 1 - span;
        if vertical {
            cells[start * width + line][2] = maximum;
        } else {
            cells[line * width + start][0] = maximum;
        }
    }
}

#[inline(always)]
fn line_value(
    cells: &[[f32; 3]],
    width: usize,
    vertical: bool,
    line: usize,
    position: usize,
) -> f32 {
    if vertical {
        cells[position * width + line][0]
    } else {
        cells[line * width + position][2]
    }
}
