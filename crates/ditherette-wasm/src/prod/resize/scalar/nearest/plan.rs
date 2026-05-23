//! Nearest-neighbor resize planning.
//!
//! Planning is separated from the packed RGBA8 kernel so the public nearest API
//! can stay small: build or reuse a plan, validate the shared production resize
//! boundary, then execute a nearest kernel. The plan stores only shape-derived
//! metadata; it does not inspect pixels or own row-layout validation.

use crate::image::{rgba8, ImageDimensions};

use super::{
    alignment::{axis_coordinate_map, ResizeAnchor},
    scale::{
        exact_downscale_factors, exact_upscale_factors, nearest_scale_class, source_x_copy_spans,
        NearestScaleClass, SourceXCopySpan,
    },
};

// ACCEPT(perf): Identity pass-through is now represented in the default
// `nearest` profile and one-shot public subject; copying directly improved
// identity cases by roughly 75-180% with other cases neutral/noisy in
// `ditherette-bench run nearest`.
// REJECT(perf): A generic exact-upscale span-fill path regressed 2x by -21.03%
// in `ditherette-bench run nearest`; wider upscale gains do
// not justify hurting the common 2x case.
// ACCEPT(perf): `nearest-anisotropic` now covers width-only and height-only
// nearest profiles with exact correctness and manifest fixtures before testing
// scalar path splits separately from `ditherette-bench run nearest`.
// TODO(perf:path, rank=18, after perf:harness nearest-anisotropic-profile):
// Test scalar width-only and height-only nearest paths once an anisotropic
// nearest profile exists; exact downscale is accepted, while identity-only and
// exact-upscale paths were rejected under the default `nearest` profile.
// REJECT(perf): Adding a same-width RGBA8 row-copy branch before the hot path
// found no represented 1x/same-width case in the current nearest profile and
// regressed most measured cases in `ditherette-bench run nearest`; do not retry
// without a dedicated identity/same-width subject.
// REJECT(perf): Precomputing packed nearest y/output row byte offsets passed
// correctness but was neutral/noisy and regressed large near-identity downscale
// by -2.06% in `ditherette-bench run nearest`; keep row byte math local to the
// loops until repeated-y run metadata is actually used.
// REJECT(perf): Routing exact 0.75x downscales through span-copy lowered the
// average-span threshold only for that shape but regressed represented large
// downscales by -66.48% in `ditherette-bench run nearest`; keep the stricter
// near-identity span-copy gate.
// NOTE(perf): A separable nearest two-pass is not a current TODO. X/Y mapping is
// already planned independently, and materializing an intermediate image would
// add a full write/read without reducing sampling work. Revisit only if a future
// `nearest` profile exposes a shape where axis-only materialization avoids more
// work than it adds.

/// Reusable nearest-neighbor resize metadata for one source/output shape.
///
/// The plan caches x/y coordinate maps and scale-class decisions that are
/// independent of the input pixels. Benchmarks reuse plans for repeated cases;
/// callers must use a plan whose source dimensions, output dimensions, and
/// anchor match the current resize request.
pub struct NearestResizePlan {
    pub(super) source_dimensions: ImageDimensions,
    pub(super) output_dimensions: ImageDimensions,
    pub(super) anchor: ResizeAnchor,
    pub(super) x_source_starts: Vec<usize>,
    pub(super) y_coordinates: Vec<u32>,
    pub(super) exact_downscale: Option<(u32, u32)>,
    pub(super) exact_upscale: Option<(u32, u32)>,
    pub(super) source_x_copy_spans: Vec<SourceXCopySpan>,
    pub(super) scale_class: NearestScaleClass,
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
        let exact_upscale = exact_upscale_factors(
            source_dimensions.width(),
            source_dimensions.height(),
            output_dimensions.width(),
            output_dimensions.height(),
        );
        let has_factor_fast_path = exact_downscale.is_some() || exact_upscale.is_some();
        let x_source_starts = coordinate_byte_map(
            source_dimensions.width(),
            output_dimensions.width(),
            x_alignment,
            has_factor_fast_path,
        );
        let y_coordinates = coordinate_map(
            source_dimensions.height(),
            output_dimensions.height(),
            y_alignment,
            has_factor_fast_path,
        );
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
            exact_upscale,
            &source_x_copy_spans,
        );

        Self {
            source_dimensions,
            output_dimensions,
            anchor,
            x_source_starts,
            y_coordinates,
            exact_downscale,
            exact_upscale,
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

fn coordinate_byte_map(
    source_len: u32,
    output_len: u32,
    alignment: super::alignment::AxisAlignment,
    skip: bool,
) -> Vec<usize> {
    if skip {
        return Vec::new();
    }

    axis_coordinate_map(source_len, output_len, alignment)
        .into_iter()
        .map(|source_x| source_x as usize * rgba8::RGBA8_CHANNELS)
        .collect()
}

fn coordinate_map(
    source_len: u32,
    output_len: u32,
    alignment: super::alignment::AxisAlignment,
    skip: bool,
) -> Vec<u32> {
    if skip {
        Vec::new()
    } else {
        axis_coordinate_map(source_len, output_len, alignment)
    }
}
