use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        convolution::{resize_with_convolution, resize_with_convolution_into, sinc, Kernel},
        convolution_reference::{
            resize_with_convolution_reference, resize_with_convolution_reference_into,
        },
    },
};

/// Shared implementation for Lanczos resize variants.
///
/// The public mode modules (`lanczos2`, `lanczos3`, and their scale-aware
/// variants) own naming and API docs; this module owns the common windowed-sinc
/// kernel and scale-aware dispatch.
// TODO(perf): Route identity, same-width, and same-height cases around the
// generic convolution planner so trivial Lanczos calls do not allocate axis
// contribution tables.
// TODO(perf): Cache contribution plans for repeated previews at the same source
// and output dimensions; Lanczos weight planning is independent of pixel bytes.
pub(crate) fn resize_rgba_lanczos(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    lobes: f64,
    scale_aware: bool,
) -> Result<Vec<u8>, ProcessingError> {
    resize_with_convolution(
        source_rgba,
        source_dimensions,
        output_dimensions,
        Lanczos::new(lobes),
        scale_aware,
    )
}

/// Shared allocation-free implementation for Lanczos resize variants.
// TODO(perf): Add Lanczos2/Lanczos3-specific dispatch so fixed lobe counts can
// use unrolled 4-tap/6-tap separable loops instead of the dynamic Kernel trait
// path.
// TODO(perf): Approximate or table sinc weights during contribution planning
// for Lanczos filters; sin() is expensive but only depends on axis samples.
// TODO(perf): Store normalized weights as fixed-point integers once planned;
// hot sampling can then use integer multiply-adds and deterministic rounding.
// TODO(perf): Specialize scale-aware minification separately from enlargement:
// downscales have wider support and benefit more from separable scratch buffers,
// while upscales can keep compact fixed tap counts.
pub(crate) fn resize_rgba_lanczos_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    lobes: f64,
    scale_aware: bool,
) -> Result<(), ProcessingError> {
    resize_with_convolution_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        Lanczos::new(lobes),
        scale_aware,
    )
}

/// Shared implementation for scale-aware Lanczos resize variants.
///
/// Scale-aware Lanczos widens the source footprint during minification so the
/// filter acts as an antialiasing low-pass. The Lanczos2/Lanczos3 public modules
/// should only choose the lobe count; shared scale-aware behavior belongs here.
// TODO(perf): Fast-path identity and non-minifying resizes. Scale-aware Lanczos
// only needs widened footprints when at least one axis is shrinking.
// TODO(perf): Dispatch pure enlargement to fixed-radius Lanczos so it can use
// compact 4-tap/6-tap planning instead of the widened contribution builder.
// TODO(perf): Cache widened contribution plans by source/output dimensions; wide
// support radii make plan construction and weight normalization expensive.
pub(crate) fn resize_rgba_lanczos_scale_aware(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    lobes: f64,
) -> Result<Vec<u8>, ProcessingError> {
    resize_rgba_lanczos(
        source_rgba,
        source_dimensions,
        output_dimensions,
        lobes,
        true,
    )
}

/// Shared allocation-free implementation for scale-aware Lanczos variants.
// TODO(perf): Use a separable downscale pipeline with horizontal scratch rows;
// wide scale-aware footprints are too costly as direct x*y sampling.
// TODO(perf): Keep a ring buffer of horizontal scratch rows so adjacent output
// scanlines reuse heavily overlapping widened vertical footprints.
// TODO(perf): Split axes by scale. If only one axis shrinks, widen that axis and
// keep the other on the fixed-radius Lanczos path.
// TODO(perf): Add exact-ratio minification plans for common 2x/4x/8x cases;
// support widths and phase patterns repeat and can be precomputed once.
// TODO(perf): Merge duplicate clamped edge taps when planning so wide footprints
// do not repeatedly sample the same border pixels.
// TODO(perf): Quantize normalized weights to fixed-point per axis to remove f64
// multiply-adds from large-footprint sampling loops.
// TODO(perf): Accumulate RGBA together for each tap so wide footprints do not
// multiply source loads by channel count.
// TODO(perf): Consider a staged minifier for huge reductions: area/box prefilter
// to an intermediate image, then scale-aware Lanczos for final quality.
pub(crate) fn resize_rgba_lanczos_scale_aware_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    lobes: f64,
) -> Result<(), ProcessingError> {
    resize_rgba_lanczos_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        lobes,
        true,
    )
}

/// Shared reference implementation for Lanczos resize variants.
pub(crate) fn resize_rgba_lanczos_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    lobes: f64,
    scale_aware: bool,
) -> Result<Vec<u8>, ProcessingError> {
    resize_with_convolution_reference(
        source_rgba,
        source_dimensions,
        output_dimensions,
        Lanczos::new(lobes),
        scale_aware,
    )
}

/// Shared allocation-free reference implementation for Lanczos resize variants.
pub(crate) fn resize_rgba_lanczos_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    lobes: f64,
    scale_aware: bool,
) -> Result<(), ProcessingError> {
    resize_with_convolution_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        Lanczos::new(lobes),
        scale_aware,
    )
}

/// Shared reference implementation for scale-aware Lanczos resize variants.
pub(crate) fn resize_rgba_lanczos_scale_aware_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    lobes: f64,
) -> Result<Vec<u8>, ProcessingError> {
    resize_rgba_lanczos_reference(
        source_rgba,
        source_dimensions,
        output_dimensions,
        lobes,
        true,
    )
}

/// Shared allocation-free reference implementation for scale-aware Lanczos variants.
pub(crate) fn resize_rgba_lanczos_scale_aware_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    lobes: f64,
) -> Result<(), ProcessingError> {
    resize_rgba_lanczos_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        lobes,
        true,
    )
}

#[derive(Debug, Clone, Copy)]
struct Lanczos {
    lobes: f64,
}

impl Lanczos {
    const fn new(lobes: f64) -> Self {
        Self { lobes }
    }
}

impl Kernel for Lanczos {
    fn radius(self) -> f64 {
        self.lobes
    }

    fn weight(self, distance: f64) -> f64 {
        // TODO(perf): Split the x==0 and x>=lobes guards into contribution-table
        // construction so the inner kernel can assume valid non-zero support.
        // TODO(perf): Replace two sinc calls with a small polynomial/table
        // approximation once visual error is benchmarked against the reference.
        let x = distance.abs();

        if x < f64::EPSILON {
            1.0
        } else if x >= self.lobes {
            0.0
        } else {
            sinc(x) * sinc(x / self.lobes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resize_rgba_lanczos;
    use crate::image::ImageDimensions;

    #[test]
    fn identity_resize_returns_same_bytes_for_supported_lobe_counts() {
        let source_rgba: Vec<u8> = (0..9)
            .flat_map(|value| [value * 13, value * 7, value * 3, 255])
            .collect();
        let dimensions = dimensions(3, 3);

        for lobes in [2.0, 3.0] {
            assert_eq!(
                resize_rgba_lanczos(&source_rgba, dimensions, dimensions, lobes, false).unwrap(),
                source_rgba
            );
            assert_eq!(
                resize_rgba_lanczos(&source_rgba, dimensions, dimensions, lobes, true).unwrap(),
                source_rgba
            );
        }
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
