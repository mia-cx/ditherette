use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
};

/// Allocates a tightly packed RGBA output buffer after validating the source.
///
/// Resize algorithms share this helper so boundary validation and Wasm memory
/// limits stay consistent across sampling modes.
// NOTE(perf): Keep zero-initialized allocation here. Unsafe uninitialized Vec
// construction would need per-kernel overwrite proofs and is not worth the
// shared boundary helper risk.
pub(crate) fn allocate_output_rgba(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, source_dimensions)?;
    // NOTE(perf): Reusable preview buffers belong in a higher-level plan/cache
    // API, not this simple allocation helper.
    Ok(vec![0; rgba::checked_rgba_byte_len(output_dimensions)?])
}

/// Validates source and output buffers for an RGBA resize operation.
// NOTE(perf): Validation stays centralized at this boundary; unchecked chained
// filter entry points should be added with a concrete pipeline plan type.
pub(crate) fn validate_resize_buffers(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &[u8],
) -> Result<(), ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, source_dimensions)?;

    // NOTE(perf): Checked byte lengths are tiny beside resize kernels; cache
    // them only if a future resize plan owns repeated same-dimension calls.
    let expected = rgba::checked_rgba_byte_len(output_dimensions)?;
    let actual = output_rgba.len();

    if actual != expected {
        return Err(ProcessingError::InvalidBufferLength { expected, actual });
    }

    // NOTE(perf): Keep full errors here until validation shows up in profiles.

    Ok(())
}
