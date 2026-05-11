use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
};

/// Allocates a tightly packed RGBA output buffer after validating the source.
///
/// Resize algorithms share this helper so boundary validation and Wasm memory
/// limits stay consistent across sampling modes.
// TODO(perf): Consider `Vec::with_capacity` plus resize/unsafe initialization
// for filters that overwrite every byte. `vec![0; len]` eagerly clears memory,
// which is wasted for full-frame resize kernels after validation proves length.
pub(crate) fn allocate_output_rgba(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, source_dimensions)?;
    // TODO(perf): Expose reusable scratch/output buffer helpers for repeated UI
    // preview resizes so callers can avoid allocation churn across frames.
    Ok(vec![0; rgba::checked_rgba_byte_len(output_dimensions)?])
}

/// Validates source and output buffers for an RGBA resize operation.
// TODO(perf): For internal pipelines that already validated RGBA dimensions,
// add a narrow unchecked/private entry point so chained filters do not repeat
// source and destination length checks at every stage.
pub(crate) fn validate_resize_buffers(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &[u8],
) -> Result<(), ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, source_dimensions)?;

    // TODO(perf): Cache checked byte lengths on a resize plan when benchmarking
    // repeated same-dimension kernels; length math is small but completely
    // invariant for each fixture/scale pair.
    let expected = rgba::checked_rgba_byte_len(output_dimensions)?;
    let actual = output_rgba.len();

    if actual != expected {
        return Err(ProcessingError::InvalidBufferLength { expected, actual });
    }

    // TODO(perf): Add debug-only assertions for hot internal callers and keep
    // full error construction at Wasm/API boundaries if validation ever shows up
    // in tiny-output benchmarks.

    Ok(())
}
