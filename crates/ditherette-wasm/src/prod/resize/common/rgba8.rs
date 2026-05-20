//! Production resize boundary helpers for normalized packed RGBA8 images.
//!
//! All production resize filters operate on packed RGBA8 rows. Filters may use
//! different kernels internally, but padded rows and non-RGBA formats must be
//! normalized before crossing this boundary.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

pub fn assert_packed_source(source: ImageView<'_, Rgba8>, filter: &str) {
    assert!(
        rgba8::is_packed_stride(source.dimensions(), source.stride()),
        "production {filter} resize requires packed RGBA8 source rows"
    );
}

pub fn assert_packed_output(output: &ImageViewMut<'_, Rgba8>, filter: &str) {
    assert!(
        rgba8::is_packed_stride(output.dimensions(), output.stride()),
        "production {filter} resize requires packed RGBA8 output rows"
    );
}
