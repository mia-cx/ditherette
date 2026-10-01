//! Frozen scalar timing adapters. Verification rendering and serialization stay outside calls.

use crate::{
    image::{contracts::IndexedImage, ImageView, ImageViewMut, Rgba8},
    spec::{self, contract::request},
};
use ditherette_bench_api::BenchSubjectError;

pub const PERTURB_SUBJECT: &str = "prod:perturb:request:into-v1";
pub const ORDERED_PERTURB_SUBJECT: &str = "prod:dither-modes:ordered-perturb:into-v1";

/// The field half of a perturb batch: a v1 field, or a rectangular tile from `dither_modes`.
#[derive(Clone, Copy)]
enum BatchField {
    V1(request::PerturbPolicy),
    Tile(
        spec::dither_modes::ordered::TilePerturbPolicy,
        crate::prod::contract::request::OrderedTile,
    ),
}

/// Full field composition into preallocated RGBA8. Validation and policy mapping precede timing.
pub struct PerturbBatch<'a> {
    source: ImageView<'a, Rgba8>,
    field: BatchField,
    prod_policy: crate::prod::contract::request::PerturbPolicy,
    rgba: Vec<u8>,
}

impl<'a> PerturbBatch<'a> {
    pub fn new(input: &super::reference::ReferenceRequest<'a>) -> Result<Self, BenchSubjectError> {
        use super::reference::{DitherModesRequest, ReferenceRequest};
        use crate::prod::contract::request as prod;
        input.dimensions()?;
        let (source, field, prod_policy) = match *input {
            ReferenceRequest::Processing(request::Request::Perturb(input)) => (
                input.source,
                BatchField::V1(input.perturb),
                super::fields::prod_policy(input.perturb),
            ),
            ReferenceRequest::DitherModes(DitherModesRequest::Perturb { source, policy }) => {
                let retag = |value: serde_json::Value| {
                    serde_json::from_value::<prod::PerturbPolicy>(value)
                        .map_err(|error| BenchSubjectError::new(error.to_string()))
                };
                let prod_policy = retag(serde_json::json!({
                    "field": { "algorithm": "ordered", "tile": policy.tile },
                    "space": policy.space,
                    "strength": policy.strength,
                    "placement": policy.placement,
                }))?;
                let prod::Field::Ordered { tile } = prod_policy.field else {
                    unreachable!("ordered tag maps to the ordered field")
                };
                (source, BatchField::Tile(policy, tile), prod_policy)
            }
            _ => {
                return Err(BenchSubjectError::new(
                    "perturb batch requires a perturb request",
                ))
            }
        };
        let dimensions = crate::image::ImageDimensions::new(source.width, source.height)
            .expect("validated dimensions");
        Ok(Self {
            source: ImageView::packed(source.data, dimensions).expect("validated source"),
            field,
            prod_policy,
            rgba: vec![0; source.data.len()],
        })
    }

    /// Execute the real complete perturb loop, including its converter setup and adaptive reads.
    pub fn run(&mut self, production: bool) {
        let dimensions = self.source.dimensions();
        let output = ImageViewMut::packed(&mut self.rgba, dimensions).expect("prepared output");
        let rows =
            || crate::prod::tiling::RowBand::new(0, dimensions.height()).expect("nonempty rows");
        let policy = self.prod_policy;
        match (self.field, production) {
            (BatchField::V1(field), true) => {
                crate::prod::dither::perturb::perturb_by_field_rows_into(
                    self.source,
                    output,
                    policy.space,
                    policy.strength,
                    policy.placement,
                    rows(),
                    |x, y, index| super::fields::threshold(field.field, x, y, index, true),
                )
            }
            (BatchField::V1(policy), false) => {
                spec::dither::perturb::perturb_into(self.source, output, policy)
            }
            (BatchField::Tile(_, tile), true) => {
                crate::prod::dither::perturb::perturb_by_field_rows_into(
                    self.source,
                    output,
                    policy.space,
                    policy.strength,
                    policy.placement,
                    rows(),
                    |x, y, _| crate::prod::dither::ordered::tile_noise_at(x, y, tile),
                )
            }
            (BatchField::Tile(policy, _), false) => {
                // The reference's own perturb minus validation, matching v1 `perturb_into`.
                spec::dither::perturb::perturb_by_field_rows_into(
                    self.source,
                    output,
                    policy.space,
                    policy.strength,
                    policy.placement,
                    spec::tiling::contract::RowBand::new(0, dimensions.height())
                        .expect("nonempty rows"),
                    |x, y, _| spec::dither_modes::ordered::tile_noise_at(x, y, policy.tile),
                )
            }
        }
        std::hint::black_box(self.rgba.as_slice());
    }

    /// Copy diagnostic bytes only after the timer stops.
    pub fn output(&self) -> ditherette_bench_api::verification::VerificationOutput {
        use ditherette_bench_api::verification::*;
        VerificationOutput {
            dimensions: Dimensions {
                width: self.source.dimensions().width(),
                height: self.source.dimensions().height(),
            },
            pixels: Pixels::Rgba8 {
                data: self.rgba.clone(),
            },
            warnings: vec![],
        }
    }
}

pub(super) fn subjects() -> Vec<super::BenchSubject> {
    use ditherette_bench_api::*;
    [
        (PERTURB_SUBJECT, "spec:perturb:request:v1"),
        (
            ORDERED_PERTURB_SUBJECT,
            super::reference::MODES_PERTURB_SUBJECT,
        ),
    ]
    .into_iter()
    .map(|(id, oracle)| {
        super::BenchSubject::Conformance(ConformanceBenchSubject {
            descriptor: SubjectDescriptor {
                id: SubjectId::parse(id).expect("literal ID"),
                display_name: "complete scalar perturb into caller-owned RGBA8".into(),
                source_file: "crates/ditherette-wasm/src/prod/dither/perturb.rs".into(),
                source_line: 1,
                default_oracle: Some(SubjectId::parse(oracle).expect("literal oracle")),
                capabilities: SubjectCapabilities::rgba8_packed(),
                params_schema: ParamSchema::default(),
            },
            operation: verification::Operation::Perturb,
            run: |input| {
                let mut batch = PerturbBatch::new(input)?;
                batch.run(true);
                Ok(batch.output())
            },
        })
    })
    .collect()
}

/// Borrowed-source complete operation, including validation, preparation and result allocation.
/// This deliberately excludes Processor boundaries, which copy inputs and retain application caches.
pub fn indexed_call(request: request::Request<'_>) -> Result<IndexedImage, BenchSubjectError> {
    let output = match request {
        request::Request::Quantize(request) => spec::quantize::quantize(request),
        request::Request::DitherAndQuantize(request) => match request.dither {
            request::DitherPolicy::Diffusion { .. } => {
                spec::dither::error_diffusion::diffuse(request)
            }
            request::DitherPolicy::Yliluoma { .. } => {
                spec::dither::yiluoma::dither_yiluoma(request)
            }
            _ => return Err(BenchSubjectError::new("unsupported scalar indexed recipe")),
        },
        _ => {
            return Err(BenchSubjectError::new(
                "unsupported scalar indexed operation",
            ))
        }
    };
    output.map_err(|error| BenchSubjectError::new(error.to_string()))
}

/// Real frozen forward image export into caller-owned storage, without inverse rendering.
pub fn forward_into(
    source: ImageView<'_, Rgba8>,
    coordinates: &mut [f32],
    space: request::WorkingSpace,
) {
    let dimensions = source.dimensions();
    macro_rules! forward {
        ($module:ident, $function:ident) => {
            spec::color::$module::$function(
                source,
                ImageViewMut::packed(coordinates, dimensions).expect("prepared coordinates"),
            )
        };
    }
    match space {
        request::WorkingSpace::Srgb => forward!(srgb, rgba8_to_srgb32_into),
        request::WorkingSpace::LinearRgb => forward!(linear, rgba8_to_linear_rgb32_into),
        request::WorkingSpace::Oklab => forward!(oklab, rgba8_to_oklab32_into),
        request::WorkingSpace::Oklch => forward!(oklch, rgba8_to_oklch32_into),
        request::WorkingSpace::Cielab => forward!(cielab, rgba8_to_cielab32_into),
        request::WorkingSpace::Cielch => forward!(cielch, rgba8_to_cielch32_into),
        request::WorkingSpace::Ycbcr => forward!(ycbcr, rgba8_to_ycbcr32_into),
    }
}
