//! Development tripwires for the production packed-RGBA8 resize invariant.
//!
//! These helpers are not public input validation. Production resize is an
//! internal subsystem whose callers must normalize inputs before reaching prod
//! kernels. The debug assertions make architecture violations obvious during
//! development without adding release-mode hot-path checks.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

/// Debug-assert that a production resize source view uses contiguous RGBA8 rows.
///
/// Use this at prod resize entrypoints while APIs still accept generic image
/// views. A failure means code crossed the prod boundary without normalization;
/// it is a development-time architecture bug, not recoverable user input.
pub fn assert_packed_source(source: ImageView<'_, Rgba8>, filter: &str) {
    debug_assert!(
        rgba8::is_packed_stride(source.dimensions(), source.stride()),
        "production {filter} resize requires packed RGBA8 source rows"
    );
}

/// Debug-assert that a production resize output view uses contiguous RGBA8 rows.
///
/// A packed output lets filters compute row offsets from dimensions alone.
/// Filters may optimize differently internally, but they all write the same
/// browser-`ImageData` byte layout: `R, G, B, A` repeated with no row padding.
pub fn assert_packed_output(output: &ImageViewMut<'_, Rgba8>, filter: &str) {
    debug_assert!(
        rgba8::is_packed_stride(output.dimensions(), output.stride()),
        "production {filter} resize requires packed RGBA8 output rows"
    );
}
