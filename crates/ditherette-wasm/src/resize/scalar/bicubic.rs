use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        scalar::convolution::{resize_with_convolution, resize_with_convolution_into},
        shared::bicubic::Bicubic,
    },
};

#[allow(dead_code)]
pub(crate) fn resize_rgba_bicubic(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    resize_with_convolution(
        source_rgba,
        source_dimensions,
        output_dimensions,
        Bicubic,
        true,
    )
}

pub(crate) fn resize_rgba_bicubic_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    // REJECT(perf): Bicubic-specific fixed-tap/planner variants overlapped the
    // convolution trials for kernel metadata, flat weights, byte offsets,
    // edge/interior splitting, duplicate edge taps, exact-ratio patterns, and
    // one-axis paths. Those either regressed `pnpm bench:resize:bicubic`, failed
    // byte-exact correctness, or duplicate the accepted generic convolution
    // fast paths. Keep bicubic on the shared convolution base until a dedicated
    // end-to-end replacement is justified by a new benchmark target.
    resize_with_convolution_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        Bicubic,
        true,
    )
}
