//! Frozen scalar timing adapters. Verification rendering and serialization stay outside calls.

use crate::{
    image::{contracts::IndexedImage, ImageView, ImageViewMut, Rgba8},
    spec::{self, contract::request},
};
use ditherette_bench_api::BenchSubjectError;

pub const PERTURB_SUBJECT: &str = "prod:perturb:request:into-v1";

/// Full field composition into preallocated RGBA8. Validation and policy mapping precede timing.
pub struct PerturbBatch<'a> {
    source: ImageView<'a, Rgba8>,
    policy: request::PerturbPolicy,
    prod_policy: crate::prod::contract::request::PerturbPolicy,
    rgba: Vec<u8>,
}

impl<'a> PerturbBatch<'a> {
    pub fn new(input: &super::reference::ReferenceRequest<'a>) -> Result<Self, BenchSubjectError> {
        input.dimensions()?;
        let super::reference::ReferenceRequest::Processing(request::Request::Perturb(input)) =
            *input
        else {
            return Err(BenchSubjectError::new(
                "perturb batch requires a perturb request",
            ));
        };
        let dimensions =
            crate::image::ImageDimensions::new(input.source.width, input.source.height)
                .expect("validated dimensions");
        Ok(Self {
            source: ImageView::packed(input.source.data, dimensions).expect("validated source"),
            policy: input.perturb,
            prod_policy: super::fields::prod_policy(input.perturb),
            rgba: vec![0; input.source.data.len()],
        })
    }

    /// Execute the real complete perturb loop, including its converter setup and adaptive reads.
    pub fn run(&mut self, production: bool) {
        let dimensions = self.source.dimensions();
        let output = ImageViewMut::packed(&mut self.rgba, dimensions).expect("prepared output");
        if production {
            crate::prod::dither::perturb::perturb_by_field_rows_into(
                self.source,
                output,
                self.prod_policy.space,
                self.prod_policy.strength,
                self.prod_policy.placement,
                crate::prod::tiling::RowBand::new(0, dimensions.height()).expect("nonempty rows"),
                |x, y, index| super::fields::threshold(self.policy.field, x, y, index, true),
            );
        } else {
            spec::dither::perturb::perturb_into(self.source, output, self.policy);
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
    vec![super::BenchSubject::Conformance(ConformanceBenchSubject {
        descriptor: SubjectDescriptor {
            id: SubjectId::parse(PERTURB_SUBJECT).expect("literal ID"),
            display_name: "complete scalar perturb into caller-owned RGBA8".into(),
            source_file: "crates/ditherette-wasm/src/prod/dither/perturb.rs".into(),
            source_line: 1,
            default_oracle: Some(
                SubjectId::parse("spec:perturb:request:v1").expect("literal oracle"),
            ),
            capabilities: SubjectCapabilities::rgba8_packed(),
            params_schema: ParamSchema::default(),
        },
        operation: verification::Operation::Perturb,
        run: |input| {
            let mut batch = PerturbBatch::new(input)?;
            batch.run(true);
            Ok(batch.output())
        },
    })]
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
