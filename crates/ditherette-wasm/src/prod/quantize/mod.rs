//! Native direct quantization baseline. Public processor integration is separate.

pub mod matcher;

use crate::{
    image::{contracts::IndexedImage, ImageBuf, PaletteIndex8},
    prod::{
        color::packed::{Converter, OrdinarySpace},
        contract::{
            error::{DitheretteError, ErrorCode},
            request::{QuantizeRequest, Request},
        },
        palette::{PalettePixel, PreparedPalette},
    },
};
use matcher::PaletteMatcher;

/// Validates the complete request before palette preparation or output allocation.
pub fn quantize(request: QuantizeRequest<'_>) -> Result<IndexedImage, DitheretteError> {
    let layout = Request::Quantize(request).validate()?;
    let space = OrdinarySpace::from_matching(request.matching).ok_or_else(|| {
        DitheretteError::new(
            ErrorCode::UnsupportedOperation,
            "matching",
            "This native slice supports ordinary Euclidean matching only.",
        )
    })?;
    let palette = PreparedPalette::new(request.palette, request.alpha);
    let converter = Converter::new(space);
    let matcher = PaletteMatcher::new(&palette, &converter);
    let mut indices = ImageBuf::<PaletteIndex8>::new_packed(layout.output)
        .expect("validated RGBA8 dimensions also fit packed palette indices");
    for (source, output) in request.source.data.chunks_exact(4).zip(indices.data_mut()) {
        let rgba = [source[0], source[1], source[2], source[3]];
        *output = match palette.prepare_pixel(rgba) {
            PalettePixel::Index(index) => index,
            PalettePixel::Color(rgb) => matcher.nearest(converter.coordinates(rgb)).index,
        };
    }
    Ok(palette.into_indexed(indices))
}
