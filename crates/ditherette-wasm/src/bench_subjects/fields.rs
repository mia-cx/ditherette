//! Allocation-free component batches. Frozen input preparation and serialization are untimed.

use super::{reference::ReferenceRequest, verification::color_space, BenchSubject};
use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod, spec,
};
use ditherette_bench_api::{
    verification::*, BenchSubjectError, ConformanceBenchSubject, ParamSchema, PixelFormat,
    SubjectCapabilities, SubjectDescriptor, SubjectId,
};
use serde::{Deserialize, Serialize};
use spec::contract::request::{BayerSize, Field, PerturbPolicy, Placement, Source, WorkingSpace};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "component", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Component {
    Inverse {
        space: WorkingSpace,
    },
    Field {
        field: Field,
    },
    Placement {
        space: WorkingSpace,
        placement: Placement,
    },
    SourceConversion {
        space: WorkingSpace,
    },
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Inverse,
    Field,
    Placement,
    SourceConversion,
}

impl Component {
    fn kind(self) -> Kind {
        match self {
            Self::Inverse { .. } => Kind::Inverse,
            Self::Field { .. } => Kind::Field,
            Self::Placement { .. } => Kind::Placement,
            Self::SourceConversion { .. } => Kind::SourceConversion,
        }
    }

    pub fn reference_subject(self) -> &'static str {
        match self.kind() {
            Kind::Inverse => "spec:color:inverse:f32-image-v1",
            Kind::Field => "spec:field:thresholds:global-v1",
            Kind::Placement => "spec:placement:adaptive:mask-v1",
            Kind::SourceConversion => "spec:color:source:construction-inclusive-v1",
        }
    }

    pub fn prod_subject(self) -> &'static str {
        match self.kind() {
            Kind::Inverse => "prod:color:inverse:f32-image-v1",
            Kind::Field => "prod:field:thresholds:global-v1",
            Kind::Placement => "prod:placement:adaptive:mask-v1",
            Kind::SourceConversion => "prod:color:source:construction-inclusive-v1",
        }
    }

    pub fn semantics(self) -> SemanticIdentity {
        let (operation, recipe, space) = match self {
            Self::Inverse { space } => (
                Operation::ColorInverse,
                "f32-image-inverse-frozen-forward-byte-alpha",
                Some(space),
            ),
            Self::Field { .. } => (
                Operation::FieldEvaluation,
                "global-row-major-field-thresholds",
                None,
            ),
            Self::Placement { space, .. } => (
                Operation::PlacementMask,
                "source-adaptive-placement-mask",
                Some(space),
            ),
            Self::SourceConversion { space } => (
                Operation::Color,
                "source-conversion-construction-inclusive",
                Some(space),
            ),
        };
        SemanticIdentity {
            operation,
            recipe: recipe.into(),
            version: 1,
            space: space.map(color_space),
        }
    }

    pub fn validate(self, source: Source<'_>) -> Result<(), BenchSubjectError> {
        let (field, space, placement) = match self {
            Self::Field { field } => (field, WorkingSpace::Srgb, Placement::Everywhere {}),
            Self::Placement { space, placement } => (Field::Random { seed: 0 }, space, placement),
            Self::Inverse { space } | Self::SourceConversion { space } => {
                (Field::Random { seed: 0 }, space, Placement::Everywhere {})
            }
        };
        spec::contract::request::Request::Perturb(spec::contract::request::PerturbRequest {
            version: 1,
            source,
            perturb: PerturbPolicy {
                field,
                space,
                strength: 0.0,
                placement,
            },
        })
        .validate()
        .map_err(|e| BenchSubjectError::new(e.to_string()))?;
        Ok(())
    }
}

/// Owns only untimed input/output storage. Every run overwrites a complete component batch.
pub struct PreparedComponent<'a> {
    component: Component,
    source: Source<'a>,
    coordinates: Vec<f32>,
    values: Vec<f32>,
    rgba: Vec<u8>,
}

impl<'a> PreparedComponent<'a> {
    pub fn new(component: Component, source: Source<'a>) -> Result<Self, BenchSubjectError> {
        component.validate(source)?;
        let count = source.data.len() / 4;
        let coordinates = if let Component::Inverse { space } = component {
            source
                .data
                .chunks_exact(4)
                .flat_map(|p| spec::color::rgb8_to_coordinates([p[0], p[1], p[2]], space))
                .collect()
        } else {
            Vec::new()
        };
        Ok(Self {
            component,
            source,
            coordinates,
            values: if matches!(component, Component::Inverse { .. }) {
                Vec::new()
            } else {
                vec![
                    0.0;
                    count
                        * if matches!(component, Component::SourceConversion { .. }) {
                            3
                        } else {
                            1
                        }
                ]
            },
            rgba: if matches!(component, Component::Inverse { .. }) {
                vec![0; source.data.len()]
            } else {
                Vec::new()
            },
        })
    }

    pub fn run_production(&mut self) {
        self.run(true);
        std::hint::black_box(self.values.as_slice());
        std::hint::black_box(self.rgba.as_slice());
    }

    fn run(&mut self, production: bool) {
        let dimensions = ImageDimensions::new(self.source.width, self.source.height)
            .expect("validated dimensions");
        let source =
            ImageView::<Rgba8>::packed(self.source.data, dimensions).expect("validated source");
        if let Component::Inverse { space } = self.component {
            inverse_image(&self.coordinates, source, &mut self.rgba, space, production);
            return;
        }
        for y in 0..self.source.height {
            for x in 0..self.source.width {
                let index = u64::from(y) * u64::from(self.source.width) + u64::from(x);
                match self.component {
                    Component::Field { field } => {
                        self.values[index as usize] = threshold(field, x, y, index, production)
                    }
                    Component::Placement { space, placement } => {
                        self.values[index as usize] = if production {
                            prod::dither::placement::placement_mask_at(
                                source,
                                x,
                                y,
                                prod_space(space),
                                prod_placement(placement),
                            )
                        } else {
                            spec::dither::placement::placement_mask_at(
                                source, x, y, space, placement,
                            )
                        }
                    }
                    Component::SourceConversion { space } => {
                        let pixel = source.pixel(x, y).expect("validated pixel");
                        let rgb = [pixel[0], pixel[1], pixel[2]];
                        // Baseline intentionally includes the real per-pixel Converter construction.
                        let converted = if production {
                            prod::color::packed::rgb8_to_coordinates(rgb, prod_space(space))
                        } else {
                            spec::color::rgb8_to_coordinates(rgb, space)
                        };
                        self.values[index as usize * 3..index as usize * 3 + 3]
                            .copy_from_slice(&converted);
                    }
                    Component::Inverse { .. } => unreachable!(),
                }
            }
        }
    }

    pub fn output(&self) -> VerificationOutput {
        let pixels = match self.component {
            Component::Inverse { .. } => Pixels::Rgba8 {
                data: self.rgba.clone(),
            },
            Component::SourceConversion { space } => Pixels::Color {
                space: color_space(space),
                coordinates: self.values.clone(),
                alpha: self.source.data.chunks_exact(4).map(|p| p[3]).collect(),
                rendered_rgba: None,
            },
            Component::Field { .. } | Component::Placement { .. } => Pixels::Scores {
                values: self.values.clone(),
            },
        };
        VerificationOutput {
            dimensions: Dimensions {
                width: self.source.width,
                height: self.source.height,
            },
            pixels,
            warnings: vec![],
        }
    }
}

fn threshold(field: Field, x: u32, y: u32, index: u64, production: bool) -> f32 {
    match field {
        Field::Random { seed } => {
            if production {
                prod::dither::random_noise::random_noise_at(seed, index)
            } else {
                spec::dither::random_noise::random_noise_at(seed, index)
            }
        }
        Field::Bayer { size } => {
            if production {
                use prod::dither::ordered::BayerSize as P;
                prod::dither::ordered::bayer_noise_at(
                    x,
                    y,
                    match size {
                        BayerSize::Two => P::Two,
                        BayerSize::Four => P::Four,
                        BayerSize::Eight => P::Eight,
                        BayerSize::Sixteen => P::Sixteen,
                    },
                )
            } else {
                use spec::dither::ordered::BayerSize as S;
                spec::dither::ordered::bayer_noise_at(
                    x,
                    y,
                    match size {
                        BayerSize::Two => S::Two,
                        BayerSize::Four => S::Four,
                        BayerSize::Eight => S::Eight,
                        BayerSize::Sixteen => S::Sixteen,
                    },
                )
            }
        }
        Field::BlueNoise {} => {
            if production {
                prod::dither::blue_noise::blue_noise_at(x, y)
            } else {
                spec::dither::blue_noise::blue_noise_at(x, y)
            }
        }
    }
}

fn inverse_image(
    coordinates: &[f32],
    source: ImageView<'_, Rgba8>,
    rgba: &mut [u8],
    space: WorkingSpace,
    production: bool,
) {
    let dimensions = source.dimensions();
    macro_rules! inverse {
        ($module:ident, $function:ident) => {
            if production {
                prod::color::$module::$function(
                    ImageView::packed(coordinates, dimensions).expect("prepared coordinates"),
                    source,
                    ImageViewMut::packed(rgba, dimensions).expect("prepared output"),
                );
            } else {
                spec::color::$module::$function(
                    ImageView::packed(coordinates, dimensions).expect("prepared coordinates"),
                    source,
                    ImageViewMut::packed(rgba, dimensions).expect("prepared output"),
                );
            }
        };
    }
    match space {
        WorkingSpace::Srgb => inverse!(srgb, srgb32_to_rgba8_into),
        WorkingSpace::LinearRgb => inverse!(linear, linear_rgb32_to_rgba8_into),
        WorkingSpace::Oklab => inverse!(oklab, oklab32_to_rgba8_into),
        WorkingSpace::Oklch => inverse!(oklch, oklch32_to_rgba8_into),
        WorkingSpace::Cielab => inverse!(cielab, cielab32_to_rgba8_into),
        WorkingSpace::Cielch => inverse!(cielch, cielch32_to_rgba8_into),
        WorkingSpace::Ycbcr => inverse!(ycbcr, ycbcr32_to_rgba8_into),
    }
}

pub fn prod_space(space: WorkingSpace) -> prod::contract::request::WorkingSpace {
    use prod::contract::request::WorkingSpace as P;
    match space {
        WorkingSpace::Srgb => P::Srgb,
        WorkingSpace::LinearRgb => P::LinearRgb,
        WorkingSpace::Oklab => P::Oklab,
        WorkingSpace::Oklch => P::Oklch,
        WorkingSpace::Cielab => P::Cielab,
        WorkingSpace::Cielch => P::Cielch,
        WorkingSpace::Ycbcr => P::Ycbcr,
    }
}

pub fn prod_placement(placement: Placement) -> prod::contract::request::Placement {
    use prod::contract::request::Placement as P;
    match placement {
        Placement::Everywhere {} => P::Everywhere {},
        Placement::Adaptive {
            radius,
            threshold,
            softness,
        } => P::Adaptive {
            radius,
            threshold,
            softness,
        },
    }
}

pub fn prod_policy(policy: PerturbPolicy) -> prod::contract::request::PerturbPolicy {
    use prod::contract::request::{BayerSize as B, Field as F};
    prod::contract::request::PerturbPolicy {
        space: prod_space(policy.space),
        strength: policy.strength,
        placement: prod_placement(policy.placement),
        field: match policy.field {
            Field::Random { seed } => F::Random { seed },
            Field::BlueNoise {} => F::BlueNoise {},
            Field::Bayer { size } => F::Bayer {
                size: match size {
                    BayerSize::Two => B::Two,
                    BayerSize::Four => B::Four,
                    BayerSize::Eight => B::Eight,
                    BayerSize::Sixteen => B::Sixteen,
                },
            },
        },
    }
}

fn output(
    request: &ReferenceRequest<'_>,
    kind: Kind,
    production: bool,
) -> Result<VerificationOutput, BenchSubjectError> {
    let ReferenceRequest::FieldComponent { source, component } = *request else {
        return Err(BenchSubjectError::new(
            "component requires its typed request",
        ));
    };
    if component.kind() != kind {
        return Err(BenchSubjectError::new(
            "component and registered family differ",
        ));
    }
    let mut batch = PreparedComponent::new(component, source)?;
    batch.run(production);
    Ok(batch.output())
}

pub(super) fn subjects() -> Vec<BenchSubject> {
    use super::reference::ReferenceFn;
    let entries: [(Component, ReferenceFn, ReferenceFn); 4] = [
        (
            Component::Inverse {
                space: WorkingSpace::Srgb,
            },
            |r| output(r, Kind::Inverse, false),
            |r| output(r, Kind::Inverse, true),
        ),
        (
            Component::Field {
                field: Field::Random { seed: 0 },
            },
            |r| output(r, Kind::Field, false),
            |r| output(r, Kind::Field, true),
        ),
        (
            Component::Placement {
                space: WorkingSpace::Srgb,
                placement: Placement::Everywhere {},
            },
            |r| output(r, Kind::Placement, false),
            |r| output(r, Kind::Placement, true),
        ),
        (
            Component::SourceConversion {
                space: WorkingSpace::Srgb,
            },
            |r| output(r, Kind::SourceConversion, false),
            |r| output(r, Kind::SourceConversion, true),
        ),
    ];
    entries
        .into_iter()
        .flat_map(|(component, reference, production)| {
            [(false, reference), (true, production)].map(move |(prod, run)| {
                let id = if prod {
                    component.prod_subject()
                } else {
                    component.reference_subject()
                };
                BenchSubject::Conformance(ConformanceBenchSubject {
                    descriptor: SubjectDescriptor {
                        id: SubjectId::parse(id).expect("literal ID"),
                        display_name: id.into(),
                        source_file: "crates/ditherette-wasm/src/bench_subjects/fields.rs".into(),
                        source_line: 1,
                        default_oracle: prod.then(|| {
                            SubjectId::parse(component.reference_subject()).expect("literal ID")
                        }),
                        capabilities: SubjectCapabilities {
                            pixel_formats: vec![match component.kind() {
                                Kind::Inverse => PixelFormat::Rgba8,
                                Kind::SourceConversion => PixelFormat::Color32,
                                _ => PixelFormat::Score32,
                            }],
                            supports_strided_io: false,
                            supports_tiling_params: false,
                            scalar_control: None,
                        },
                        params_schema: ParamSchema::default(),
                    },
                    operation: component.semantics().operation,
                    run,
                })
            })
        })
        .collect()
}
