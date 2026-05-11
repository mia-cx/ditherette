use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        area::resize_rgba_area_reference,
        bilinear::resize_rgba_bilinear_reference_into,
        buffers::{allocate_output_rgba, validate_resize_buffers},
    },
};

/// Resizes RGBA with mipmapped trilinear interpolation.
///
/// Trilinear filtering is primarily a texture-sampling technique: build a
/// mipmap pyramid, bilinearly sample the two mip levels nearest the requested
/// minification, then blend between those levels. It is included for comparison,
/// but area or scale-aware Lanczos are more direct choices for one-shot CPU image
/// resizing.
// TODO(perf): Fast-path identity and non-minifying resizes before allocating the
// output Vec. Upscales can dispatch directly to optimized bilinear once that path
// exists.
// TODO(perf): Accept a caller-owned mip pyramid for repeated preview sizes of
// the same source so this allocating API does not rebuild levels each call.
pub fn resize_rgba_trilinear(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_trilinear_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with mipmapped trilinear filtering.
pub fn resize_rgba_trilinear_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    // TODO(perf): Cache mip pyramids for repeated previews of the same source.
    // TODO(perf): Build only the two mip levels needed for the target scale.
    // TODO(perf): Build mip levels incrementally in reusable scratch buffers to
    // avoid allocating and copying a fresh Vec at each level.
    // TODO(perf): Downsample mip levels with exact 2x box fast paths instead of
    // the general area reference path.
    // TODO(perf): Sample the two selected mip levels into one output pass. The
    // current reference path bilinearly resizes both levels into full buffers,
    // then walks the output a third time to blend them.
    // TODO(perf): Fuse bilinear sampling and LOD blending per pixel so the lower
    // and upper samples share coordinate math and avoid two intermediate images.
    // TODO(perf): Blend mip levels in a fixed-point pass instead of f64 per byte.
    // TODO(perf): Precompute bilinear source indices and weights per output axis
    // once for both mip levels; the output grid is shared.
    // TODO(perf): Special-case exact power-of-two minification. If blend is zero,
    // only one mip level is needed and the final LOD blend can be skipped.
    // TODO(perf): Add anisotropic handling instead of using max-axis LOD for both
    // axes; resizing width-only or height-only should not over-blur the other
    // axis and may need fewer mip levels.
    // TODO(perf): Store mip levels in a compact pyramid object with dimensions
    // and byte offsets to improve locality and reduce Vec metadata churn.
    resize_rgba_trilinear_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

/// Straightforward reference implementation for mipmapped trilinear filtering.
#[doc(hidden)]
pub fn resize_rgba_trilinear_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_trilinear_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Allocation-free form of the trilinear reference implementation.
#[doc(hidden)]
pub fn resize_rgba_trilinear_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    validate_resize_buffers(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )?;

    if source_dimensions == output_dimensions {
        output_rgba.copy_from_slice(source_rgba);
        return Ok(());
    }

    let minification = minification_factor(source_dimensions, output_dimensions);
    if minification <= 1.0 {
        return resize_rgba_bilinear_reference_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        );
    }

    let lod = minification.log2();
    let lower_lod = lod.floor() as usize;
    let upper_lod = lod.ceil() as usize;
    let blend = lod - lower_lod as f64;
    let lower_level = mip_level(source_rgba, source_dimensions, lower_lod)?;

    if lower_lod == upper_lod
        || lower_level.dimensions.width() == 1 && lower_level.dimensions.height() == 1
    {
        return resize_rgba_bilinear_reference_into(
            &lower_level.rgba,
            lower_level.dimensions,
            output_dimensions,
            output_rgba,
        );
    }

    let upper_level = mip_level(source_rgba, source_dimensions, upper_lod)?;
    let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions)?;
    let mut lower_rgba = vec![0; output_byte_len];
    let mut upper_rgba = vec![0; output_byte_len];

    resize_rgba_bilinear_reference_into(
        &lower_level.rgba,
        lower_level.dimensions,
        output_dimensions,
        &mut lower_rgba,
    )?;
    resize_rgba_bilinear_reference_into(
        &upper_level.rgba,
        upper_level.dimensions,
        output_dimensions,
        &mut upper_rgba,
    )?;

    for (output, (lower, upper)) in output_rgba
        .iter_mut()
        .zip(lower_rgba.iter().zip(&upper_rgba))
    {
        *output = (f64::from(*lower) * (1.0 - blend) + f64::from(*upper) * blend)
            .round()
            .clamp(0.0, 255.0) as u8;
    }

    Ok(())
}

#[derive(Debug)]
struct RgbaLevel {
    dimensions: ImageDimensions,
    rgba: Vec<u8>,
}

fn minification_factor(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> f64 {
    let x_factor = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_factor = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());

    x_factor.max(y_factor)
}

fn mip_level(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    target_level: usize,
) -> Result<RgbaLevel, ProcessingError> {
    let mut current_rgba = source_rgba.to_vec();
    let mut current_dimensions = source_dimensions;

    for _ in 0..target_level {
        if current_dimensions.width() == 1 && current_dimensions.height() == 1 {
            break;
        }

        let next_dimensions = ImageDimensions::new(
            half_rounded_up(current_dimensions.width()),
            half_rounded_up(current_dimensions.height()),
        )?;
        let next_rgba =
            resize_rgba_area_reference(&current_rgba, current_dimensions, next_dimensions)?;

        current_rgba = next_rgba;
        current_dimensions = next_dimensions;
    }

    Ok(RgbaLevel {
        dimensions: current_dimensions,
        rgba: current_rgba,
    })
}

fn half_rounded_up(value: u32) -> u32 {
    value.div_ceil(2).max(1)
}

#[cfg(test)]
mod tests {
    use super::{minification_factor, resize_rgba_trilinear, resize_rgba_trilinear_reference};
    use crate::image::ImageDimensions;

    #[test]
    fn computes_largest_axis_minification() {
        assert_eq!(minification_factor(dimensions(8, 4), dimensions(2, 4)), 4.0);
    }

    #[test]
    fn identity_resize_returns_same_bytes() {
        let source_rgba: Vec<u8> = (0..4)
            .flat_map(|value| [value * 40, value * 20, value * 10, 255])
            .collect();
        let dimensions = dimensions(2, 2);

        assert_eq!(
            resize_rgba_trilinear(&source_rgba, dimensions, dimensions).unwrap(),
            source_rgba
        );
        assert_eq!(
            resize_rgba_trilinear_reference(&source_rgba, dimensions, dimensions).unwrap(),
            source_rgba
        );
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
