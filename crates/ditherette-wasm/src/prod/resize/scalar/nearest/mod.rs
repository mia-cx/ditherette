//! Scalar production nearest-neighbor resize.
//!
//! Production nearest-neighbor resize.
//!
//! This module defines the optimized nearest path for production resize while
//! keeping the readable `spec` oracle independent. Inputs cross the shared prod
//! resize boundary as normalized packed RGBA8; this module owns only nearest
//! planning and nearest-specific word-copy kernels.

pub mod alignment;
mod packed;

use crate::{
    image::{rgba8, ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

use alignment::{axis_coordinate_map, AxisAlignment, ResizeAnchor};

// REJECT(perf): Adding an identity-only path was not represented in the default
// `nearest` profile and regressed/noised small cases by up to -9.67% in
// `ditherette-bench run nearest --baseline accepted`.
// REJECT(perf): A generic exact-upscale span-fill path regressed 2x by -21.03%
// in `ditherette-bench run nearest --baseline accepted`; wider upscale gains do
// not justify hurting the common 2x case.
// DEFER(perf): Fractional nearest path splits have no concrete benchmarkable
// shape yet; exact downscale is accepted, while identity-only and exact-upscale
// paths were rejected under the default `nearest` profile.
// REJECT(perf): Adding a same-width RGBA8 row-copy branch before the hot path
// found no represented 1x/same-width case in the current nearest profile and
// regressed most measured cases in `ditherette-bench run nearest --baseline
// accepted`; do not retry without a dedicated identity/same-width subject.
// REJECT(perf): Precomputing packed nearest y/output row byte offsets passed
// correctness but was neutral/noisy and regressed large near-identity downscale
// by -2.06% in `ditherette-bench run nearest --baseline accepted`; keep row
// byte math local to the loops until repeated-y run metadata is actually used.
// REJECT(perf): Routing exact 0.75x downscales through span-copy lowered the
// average-span threshold only for that shape but regressed the large 0.75x case
// by -66.48% in `ditherette-bench run nearest --baseline accepted`; keep the
// stricter near-identity span-copy gate.

/// Reusable nearest-neighbor resize metadata for one source/output shape.
///
/// The plan caches x/y coordinate maps and scale-class decisions that are
/// independent of the input pixels. Benchmarks reuse plans for repeated cases;
/// callers must use a plan whose source dimensions, output dimensions, and
/// anchor match the current resize request.
pub struct NearestResizePlan {
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    anchor: ResizeAnchor,
    x_source_starts: Vec<usize>,
    y_coordinates: Vec<u32>,
    exact_downscale: Option<(u32, u32)>,
    source_x_copy_spans: Vec<SourceXCopySpan>,
    scale_class: NearestScaleClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NearestScaleClass {
    ExactDownscale,
    NearIdentityDownscale,
    OtherDownscale,
    Upscale,
}

#[derive(Debug, Clone, Copy)]
struct SourceXCopySpan {
    source_start: usize,
    output_start: usize,
    byte_len: usize,
}

impl NearestResizePlan {
    /// Build reusable coordinate metadata for one nearest-neighbor resize shape.
    ///
    /// The plan assumes production RGBA8 layout when it computes byte offsets
    /// for x coordinates. Packed-row validation happens at the resize boundary,
    /// not during plan construction.
    pub fn new(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
    ) -> Self {
        let (x_alignment, y_alignment) = anchor.axes();
        let exact_downscale = exact_downscale_factors(
            source_dimensions.width(),
            source_dimensions.height(),
            output_dimensions.width(),
            output_dimensions.height(),
        );
        let x_source_starts = if exact_downscale.is_some() {
            Vec::new()
        } else {
            axis_coordinate_map(
                source_dimensions.width(),
                output_dimensions.width(),
                x_alignment,
            )
            .into_iter()
            .map(|source_x| source_x as usize * rgba8::RGBA8_CHANNELS)
            .collect()
        };
        let y_coordinates = if exact_downscale.is_some() {
            Vec::new()
        } else {
            axis_coordinate_map(
                source_dimensions.height(),
                output_dimensions.height(),
                y_alignment,
            )
        };
        let source_x_copy_spans = source_x_copy_spans(
            source_dimensions.width(),
            source_dimensions.height(),
            output_dimensions.width(),
            output_dimensions.height(),
            &x_source_starts,
        );
        let scale_class = nearest_scale_class(
            source_dimensions.width(),
            source_dimensions.height(),
            output_dimensions.width(),
            output_dimensions.height(),
            exact_downscale,
            &source_x_copy_spans,
        );

        Self {
            source_dimensions,
            output_dimensions,
            anchor,
            x_source_starts,
            y_coordinates,
            exact_downscale,
            source_x_copy_spans,
            scale_class,
        }
    }

    /// Return whether this plan was built for the given shape and anchor.
    pub fn matches(
        &self,
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
    ) -> bool {
        self.source_dimensions == source_dimensions
            && self.output_dimensions == output_dimensions
            && self.anchor == anchor
    }
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with nearest sampling.
///
/// This is the convenience entrypoint for one-off production nearest calls. It
/// builds a plan, validates the shared packed-RGBA8 resize boundary, then copies
/// each output pixel from its nearest source pixel.
pub fn resize_nearest_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
) {
    let plan = NearestResizePlan::new(source.dimensions(), output.dimensions(), anchor);
    resize_nearest_rgba8_with_plan_into(source, output, &plan);
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with a cached plan.
///
/// This is the benchmark and hot-loop entrypoint. The plan must match the input
/// and output dimensions. Packed-row assertions are intentionally kept here so
/// all callers hit the same production resize boundary before entering the
/// nearest word-copy kernel.
pub fn resize_nearest_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &NearestResizePlan,
) {
    debug_assert_eq!(source.dimensions(), plan.source_dimensions);
    debug_assert_eq!(output.dimensions(), plan.output_dimensions);

    common::rgba8::assert_packed_source(source, "nearest");
    common::rgba8::assert_packed_output(&output, "nearest");

    packed::resize_with_plan_into(source.data(), source.dimensions(), output.data_mut(), plan);
}

const MIN_SPAN_COPY_AVERAGE_PIXELS: usize = 10;

fn nearest_scale_class(
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    exact_downscale: Option<(u32, u32)>,
    source_x_copy_spans: &[SourceXCopySpan],
) -> NearestScaleClass {
    if exact_downscale.is_some() {
        return NearestScaleClass::ExactDownscale;
    }
    if !source_x_copy_spans.is_empty() {
        return NearestScaleClass::NearIdentityDownscale;
    }
    if source_width > output_width && source_height > output_height {
        return NearestScaleClass::OtherDownscale;
    }

    NearestScaleClass::Upscale
}

fn source_x_copy_spans(
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

fn exact_downscale_factors(
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Option<(u32, u32)> {
    if source_width <= output_width || source_height <= output_height {
        return None;
    }
    if source_width % output_width != 0 || source_height % output_height != 0 {
        return None;
    }

    Some((source_width / output_width, source_height / output_height))
}

fn alignment_offset(factor: u32, alignment: AxisAlignment) -> u32 {
    match alignment {
        AxisAlignment::Start => 0,
        AxisAlignment::Center => factor / 2,
        AxisAlignment::End => factor - 1,
    }
}
