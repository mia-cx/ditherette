//! Executable specs for mapping color-coordinate images to palette indices.
//!
//! Color conversion lives under `spec::color`. This module owns the next
//! semantic step: given input coordinates and palette coordinates, choose the
//! nearest palette entry with a metric-specific distance function. Ties are
//! resolved by stable palette order because the scan only replaces the current
//! best index on strictly smaller distances.

pub mod matcher;
pub mod metric;
pub mod nearest_color;

use super::{
    color::rgb8_to_coordinates,
    contract::{
        error::DitheretteError,
        request::{QuantizeRequest, Request},
    },
    palette::{PalettePixel, PreparedPalette},
};
use crate::image::{contracts::IndexedImage, ImageBuf, PaletteIndex8};
use matcher::PaletteMatcher;

/// Validates and quantizes packed RGBA8 without changing caller-owned storage.
/// Tags are typed at this boundary. Raw tags must first pass `parse_match`/recipe decoding.
/// Every request check finishes before palette preparation or output allocation.
pub fn quantize(request: QuantizeRequest<'_>) -> Result<IndexedImage, DitheretteError> {
    let layout = Request::Quantize(request).validate()?;
    let palette = PreparedPalette::new(request.palette, request.alpha);
    let matcher = PaletteMatcher::new(&palette, request.matching);
    let mut indices = ImageBuf::<PaletteIndex8>::new_packed(layout.output)
        .expect("validated RGBA8 dimensions also fit packed palette indices");

    for (source, output) in request.source.data.chunks_exact(4).zip(indices.data_mut()) {
        let rgba = [source[0], source[1], source[2], source[3]];
        *output = match palette.prepare_pixel(rgba) {
            PalettePixel::Index(index) => index,
            PalettePixel::Color(rgb) => {
                let coordinates = rgb8_to_coordinates(rgb, request.matching.space());
                matcher.nearest(coordinates).index
            }
        };
    }
    Ok(palette.into_indexed(indices))
}
