//! Readable composition of the v1 selector with frozen field and diffusion arithmetic.
use crate::{
    image::{contracts::IndexedImage, ImageBuf, ImageView, PaletteIndex8, Rgba8},
    spec::{
        color::rgb8_to_coordinates,
        contract::{
            error::{DitheretteError, ErrorCode},
            request::*,
        },
        dither::perturb::perturb_by_field_rows_into,
        dither_modes::diffusion::DiffusionPolicy,
        palette::{PalettePixel, PreparedPalette},
        quantize::{
            matcher::{PaletteColor, PaletteMatcher},
            metric::distance_score,
        },
        tiling::contract::RowBand,
    },
};

/// Applies full ordinary field pixels with smoothly varying regional coverage.
/// Coverage never scales a selected pixel's field, preserving phase and reconstruction boundaries.
pub fn perturb(
    source: ImageView<'_, Rgba8>,
    space: WorkingSpace,
    strength: f32,
    sample_size: u32,
    contrast_threshold: f32,
    field: impl Fn(u32, u32, u64) -> f32,
) -> ImageBuf<Rgba8> {
    let mut mask = vec![[0.0; 3]; super::scratch_len(source.dimensions())];
    super::select(source, sample_size, contrast_threshold, &mut mask, || {
        Ok::<_, ()>(())
    })
    .unwrap();
    let mut output = ImageBuf::new_packed(source.dimensions()).unwrap();
    perturb_by_field_rows_into(
        source,
        output.as_view_mut(),
        space,
        strength,
        Placement::Everywhere {},
        RowBand::new(0, source.dimensions().height()).unwrap(),
        |x, y, i| {
            let value = field(x, y, i);
            if super::covers(mask[i as usize][2], x, y) {
                value
            } else {
                0.0
            }
        },
    );
    output
}

/// Masks visible output while preserving the ordinary full-image diffusion recurrence.
/// Selected pixels retain the kernel's texture; excluded pixels use original prepared colours.
pub fn diffuse(
    quantize: QuantizeRequest<'_>,
    policy: DiffusionPolicy,
    sample_size: u32,
    contrast_threshold: f32,
) -> Result<IndexedImage, DitheretteError> {
    let layout = Request::DitherAndQuantize(DitherQuantizeRequest {
        quantize,
        dither: DitherPolicy::Diffusion {
            kernel: Diffusion::FloydSteinberg,
            strength: policy.strength,
            placement: Placement::Everywhere {},
            serpentine: policy.serpentine,
            feedback: policy.feedback,
        },
    })
    .validate()?;
    let palette = PreparedPalette::new(quantize.palette, quantize.alpha);
    let matcher = PaletteMatcher::new(&palette, quantize.matching);
    let space = quantize.matching.space();
    let pixels: Vec<_> = quantize
        .source
        .data
        .chunks_exact(4)
        .map(|p| palette.prepare_pixel([p[0], p[1], p[2], p[3]]))
        .collect();
    let mut work: Vec<[f32; 3]> = pixels
        .iter()
        .map(|&pixel| match pixel {
            PalettePixel::Index(_) => [0.0; 3],
            PalettePixel::Color(rgb) => match policy.feedback {
                DiffusionFeedback::SrgbBytes => rgb.map(f32::from),
                DiffusionFeedback::Matching => rgb8_to_coordinates(rgb, space),
            },
        })
        .collect();
    let mut mask = vec![[0.0; 3]; super::scratch_len(layout.source.dimensions())];
    super::select(
        layout.source,
        sample_size,
        contrast_threshold,
        &mut mask,
        || Ok::<_, ()>(()),
    )
    .unwrap();
    let mut indices = ImageBuf::<PaletteIndex8>::new_packed(layout.output).unwrap();
    let width = layout.output.width_usize();
    let height = layout.output.height_usize();
    for y in 0..height {
        let reverse = policy.serpentine && y % 2 == 1;
        for step in 0..width {
            let x = if reverse { width - 1 - step } else { step };
            let offset = y * width + x;
            let rgb = match pixels[offset] {
                PalettePixel::Index(index) => {
                    indices.data_mut()[offset] = index;
                    continue;
                }
                PalettePixel::Color(rgb) => rgb,
            };
            let current = work[offset];
            if current.iter().any(|v| !v.is_finite()) {
                return Err(arithmetic());
            }
            let (selected, error) = match policy.feedback {
                DiffusionFeedback::SrgbBytes => {
                    let rgb = current.map(|v| f64::from(v).round().clamp(0.0, 255.0) as u8);
                    let selected = nearest(&matcher, rgb8_to_coordinates(rgb, space))?;
                    let start = usize::from(selected.index) * 4;
                    let error: [f64; 3] = std::array::from_fn(|axis| {
                        f64::from(rgb[axis]) - f64::from(palette.palette.rgba[start + axis])
                    });
                    (selected, error)
                }
                DiffusionFeedback::Matching => {
                    let selected = nearest(&matcher, current)?;
                    let error = std::array::from_fn(|axis| {
                        f64::from(current[axis]) - f64::from(selected.coordinates[axis])
                    });
                    (selected, error)
                }
            };
            indices.data_mut()[offset] = if !super::covers(mask[offset][2], x as u32, y as u32) {
                nearest(&matcher, rgb8_to_coordinates(rgb, space))?.index
            } else {
                selected.index
            };
            for tap in policy.kernel.taps() {
                let dx = if reverse { -tap.dx } else { tap.dx };
                let Some(tx) = x.checked_add_signed(dx as isize) else {
                    continue;
                };
                let ty = y + tap.dy as usize;
                if tx >= width || ty >= height {
                    continue;
                }
                let target = ty * width + tx;
                if matches!(pixels[target], PalettePixel::Index(_)) {
                    continue;
                }
                let weight = f64::from(tap.weight) * f64::from(policy.strength);
                for axis in 0..3 {
                    let updated = (f64::from(work[target][axis]) + error[axis] * weight) as f32;
                    if !updated.is_finite() {
                        return Err(arithmetic());
                    }
                    work[target][axis] = updated;
                }
            }
        }
    }
    Ok(palette.into_indexed(indices))
}

fn nearest(
    matcher: &PaletteMatcher,
    coordinates: [f32; 3],
) -> Result<PaletteColor, DitheretteError> {
    let mut best = matcher.colors[0];
    let mut score = f32::INFINITY;
    for &candidate in &matcher.colors {
        let distance = distance_score(coordinates, candidate.coordinates, matcher.matching);
        if !distance.is_finite() {
            return Err(arithmetic());
        }
        if distance < score {
            best = candidate;
            score = distance;
        }
    }
    Ok(best)
}

fn arithmetic() -> DitheretteError {
    DitheretteError::new(
        ErrorCode::Runtime,
        "dither.arithmetic",
        "Gradient diffusion exceeded finite arithmetic.",
    )
}
