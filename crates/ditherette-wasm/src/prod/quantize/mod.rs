//! Native direct quantization. Public processor integration is separate.

pub mod matcher;
pub mod prepared;
pub use prepared::PreparedQuantizer;

use crate::{
    image::{contracts::IndexedImage, ImageBuf, PaletteIndex8},
    prod::{
        contract::{
            error::DitheretteError,
            request::{QuantizeRequest, Request},
        },
        palette::{allocation::Budget, PreparationError},
    },
};
use std::mem::size_of;

/// Request diagnostics retain the existing typed contract; reservation failures allocate no text.
#[derive(Debug)]
pub enum QuantizeError {
    Request(DitheretteError),
    Preparation(PreparationError),
}

/// Validates a typed native request, then prepares and allocates within the supplied budget.
/// The budget counts owned records, tables, palette/warnings/matcher capacity and indices.
/// Source storage is borrowed. The future public adapter separately counts its owned boundary copy.
/// Public integration may use PreparedQuantizer directly after allocation-free raw validation.
pub fn quantize(
    request: QuantizeRequest<'_>,
    memory_limit: u64,
) -> Result<IndexedImage, QuantizeError> {
    let layout = Request::Quantize(request)
        .validate()
        .map_err(QuantizeError::Request)?;
    let count = layout.output.pixel_count().expect("validated dimensions");
    PreparedQuantizer::required_capacity_bytes(request.palette, request.alpha, request.matching)
        .map_err(QuantizeError::Preparation)?;
    let output_bytes = count as u64 + size_of::<ImageBuf<PaletteIndex8>>() as u64;
    let preparation_limit = memory_limit
        .checked_sub(output_bytes)
        .ok_or(QuantizeError::Preparation(PreparationError::memory()))?;
    let prepared = PreparedQuantizer::try_new(
        request.palette,
        request.alpha,
        request.matching,
        preparation_limit,
    )
    .map_err(QuantizeError::Preparation)?;
    let mut budget = Budget::new(
        memory_limit,
        prepared.capacity_bytes() + size_of::<ImageBuf<PaletteIndex8>>() as u64,
    )
    .map_err(QuantizeError::Preparation)?;
    let mut indices = Vec::new();
    budget
        .reserve(&mut indices, count)
        .map_err(QuantizeError::Preparation)?;
    indices.resize(count, 0);
    prepared.quantize_into(layout.source, &mut indices);
    let indices = ImageBuf::<PaletteIndex8>::from_vec_packed(indices, layout.output)
        .expect("validated dimensions and reserved index length");
    Ok(prepared.into_indexed(indices))
}
