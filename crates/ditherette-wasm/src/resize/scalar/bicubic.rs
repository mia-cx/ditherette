use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        scalar::{
            convolution::{resize_with_convolution, resize_with_convolution_into},
            convolution_2::resize_with_convolution_2_into,
        },
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

pub(crate) fn resize_rgba_bicubic_2_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_bicubic_2_scale_aware_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

pub(crate) fn resize_rgba_bicubic_2_fixed_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_bicubic_2_with_scale_policy_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        false,
    )
}

pub(crate) fn resize_rgba_bicubic_2_scale_aware_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_bicubic_2_with_scale_policy_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        true,
    )
}

fn resize_rgba_bicubic_2_with_scale_policy_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    scale_aware: bool,
) -> Result<(), ProcessingError> {
    // NOTE(perf): convolution_2 source-row vertical accumulation improved bicubic_2
    // downscales by ~27-59% in `pnpm crit:resize:convolution_2 --baseline conv2_accepted`.
    // Small-source 2x upscale stays on the original vertical loop to avoid a ~8% regression.
    resize_with_convolution_2_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        Bicubic,
        scale_aware,
    )
}
