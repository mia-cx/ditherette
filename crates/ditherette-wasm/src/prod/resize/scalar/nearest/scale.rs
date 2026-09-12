//! Nearest scale classification and factor-derived metadata.
//!
//! This module has no pixel access. It classifies one resize shape and builds
//! source-x copy spans used by packed kernels.

use crate::image::rgba8;

use super::alignment::AxisAlignment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NearestScaleClass {
    ExactDownscale,
    ExactUpscale,
    NearIdentityDownscale,
    OtherDownscale,
    Upscale,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SourceXCopySpan {
    pub(super) source_start: usize,
    pub(super) output_start: usize,
    pub(super) byte_len: usize,
}

const MIN_SPAN_COPY_AVERAGE_PIXELS: usize = 10;

pub(super) fn nearest_scale_class(
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    exact_downscale: Option<(u32, u32)>,
    exact_upscale: Option<(u32, u32)>,
    source_x_copy_spans: &[SourceXCopySpan],
) -> NearestScaleClass {
    if exact_downscale.is_some() {
        return NearestScaleClass::ExactDownscale;
    }
    if exact_upscale.is_some() {
        return NearestScaleClass::ExactUpscale;
    }
    if !source_x_copy_spans.is_empty() {
        return NearestScaleClass::NearIdentityDownscale;
    }
    if source_width > output_width && source_height > output_height {
        return NearestScaleClass::OtherDownscale;
    }

    NearestScaleClass::Upscale
}

pub(super) fn source_x_copy_spans(
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    x_source_starts: &[usize],
) -> Vec<SourceXCopySpan> {
    if source_width <= output_width || source_height <= output_height || x_source_starts.is_empty()
    {
        return Vec::new();
    }

    let skipped_source_columns = source_width as usize - output_width as usize;
    let average_span_pixels = output_width as usize / skipped_source_columns.max(1);
    if average_span_pixels < MIN_SPAN_COPY_AVERAGE_PIXELS {
        return Vec::new();
    }

    build_source_x_copy_spans(x_source_starts)
}

fn build_source_x_copy_spans(x_source_starts: &[usize]) -> Vec<SourceXCopySpan> {
    let mut spans = Vec::new();
    let mut span_output_start = 0;
    let mut span_source_start = x_source_starts[0];
    let mut previous_source_start = span_source_start;

    for (output_x, source_start) in x_source_starts.iter().copied().enumerate().skip(1) {
        if source_start != previous_source_start + rgba8::RGBA8_CHANNELS {
            spans.push(SourceXCopySpan {
                source_start: span_source_start,
                output_start: span_output_start * rgba8::RGBA8_CHANNELS,
                byte_len: (output_x - span_output_start) * rgba8::RGBA8_CHANNELS,
            });
            span_output_start = output_x;
            span_source_start = source_start;
        }
        previous_source_start = source_start;
    }

    spans.push(SourceXCopySpan {
        source_start: span_source_start,
        output_start: span_output_start * rgba8::RGBA8_CHANNELS,
        byte_len: (x_source_starts.len() - span_output_start) * rgba8::RGBA8_CHANNELS,
    });
    spans
}

pub(super) fn exact_downscale_factors(
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Option<(u32, u32)> {
    if output_width == 0 || output_height == 0 {
        return None;
    }
    if source_width <= output_width || source_height <= output_height {
        return None;
    }
    if source_width % output_width != 0 || source_height % output_height != 0 {
        return None;
    }

    Some((source_width / output_width, source_height / output_height))
}

// Exact integer upscales map each source coordinate to a fixed run of output
// coordinates for every anchor: start, center, and end all reduce to
// `source = output / factor` when `output_len == source_len * factor`.
pub(super) fn exact_upscale_factors(
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Option<(u32, u32)> {
    if source_width == 0 || source_height == 0 {
        return None;
    }
    if source_width >= output_width || source_height >= output_height {
        return None;
    }
    if output_width % source_width != 0 || output_height % source_height != 0 {
        return None;
    }

    Some((output_width / source_width, output_height / source_height))
}

pub(super) fn alignment_offset(factor: u32, alignment: AxisAlignment) -> u32 {
    assert!(factor > 0, "alignment factor must be non-zero");
    match alignment {
        AxisAlignment::Start => 0,
        AxisAlignment::Center => factor / 2,
        AxisAlignment::End => factor - 1,
    }
}
