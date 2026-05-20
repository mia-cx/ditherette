//! Production resize boundary helpers for normalized packed RGBA8 images.
//!
//! All production resize filters operate on packed RGBA8 rows. Filters may use
//! different kernels internally, but padded rows and non-RGBA formats must be
//! normalized before crossing this boundary.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

/// Assert that a production resize source view uses contiguous RGBA8 rows.
///
/// Production resize kernels operate behind a hard normalized-image boundary:
/// callers must convert padded rows, subimage views, and non-RGBA formats into
/// packed RGBA8 before invoking prod resize. Keeping this check in shared prod
/// code makes the boundary consistent across filters.
pub fn assert_packed_source(source: ImageView<'_, Rgba8>, filter: &str) {
    assert!(
        rgba8::is_packed_stride(source.dimensions(), source.stride()),
        "production {filter} resize requires packed RGBA8 source rows"
    );
}

/// Assert that a production resize output view uses contiguous RGBA8 rows.
///
/// A packed output lets filters compute row offsets from dimensions alone.
/// Filters may optimize differently internally, but they all write the same
/// browser-`ImageData` byte layout: `R, G, B, A` repeated with no row padding.
pub fn assert_packed_output(output: &ImageViewMut<'_, Rgba8>, filter: &str) {
    assert!(
        rgba8::is_packed_stride(output.dimensions(), output.stride()),
        "production {filter} resize requires packed RGBA8 output rows"
    );
}
