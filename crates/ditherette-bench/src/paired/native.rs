//! Center-anchor native recipes bind support policy independently of implementation IDs.

use crate::verification::{input_digest, settings_digest};
use ditherette_bench_api::{verification::*, SubjectId};
use ditherette_wasm::bench_subjects::reference::ReferenceRequest;
pub use ditherette_wasm::bench_subjects::scores::MetricFamily;
pub use ditherette_wasm::spec::contract::request::WorkingSpace;
use std::io;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum NativeOperation {
    Processor {
        settings: super::preparation::ProcessorSettings,
        cache: super::browser::CacheCapability,
    },
    Process {
        settings: super::process::ProcessSettings,
    },
    Diffusion {
        settings: super::diffusion::DiffusionSettings,
    },
    Yliluoma {
        settings: super::yliluoma::YliluomaSettings,
    },
    FieldComponent {
        component: ditherette_wasm::bench_subjects::fields::Component,
    },
    Perturb {
        settings: super::fields::PerturbPolicy,
    },
    Separable {
        settings: super::fields::SeparableSettings,
    },
    Quantize {
        settings: super::quantize::QuantizeSettings,
    },
    ColorForward {
        space: WorkingSpace,
    },
    MetricScores {
        metric: MetricFamily,
    },
}

impl NativeOperation {
    pub fn reference_request<'a>(
        &'a self,
        source: Dimensions,
        rgba: &'a [u8],
    ) -> io::Result<ReferenceRequest<'a>> {
        match self {
            Self::Processor { settings, .. } => settings.reference_request(source, rgba),
            Self::Process { settings } => settings.reference_request(source, rgba),
            Self::Diffusion { settings } => settings.reference_request(source, rgba),
            Self::Yliluoma { settings } => settings.reference_request(source, rgba),
            Self::Perturb { settings } => super::fields::perturb_request(*settings, source, rgba),
            Self::Separable { settings } => settings.reference_request(source, rgba),
            Self::FieldComponent { component } => {
                let request = ReferenceRequest::FieldComponent {
                    source: ditherette_wasm::spec::contract::request::Source {
                        width: source.width,
                        height: source.height,
                        data: rgba,
                    },
                    component: *component,
                };
                request.dimensions().map_err(io::Error::other)?;
                Ok(request)
            }
            Self::Quantize { settings } => settings.reference_request(source, rgba),
            Self::MetricScores { metric } => {
                let request = ReferenceRequest::MetricScores {
                    source: ditherette_wasm::spec::contract::request::Source {
                        width: source.width,
                        height: source.height,
                        data: rgba,
                    },
                    metric: *metric,
                };
                request.dimensions().map_err(io::Error::other)?;
                Ok(request)
            }
            Self::ColorForward { space } => {
                let request = ReferenceRequest::Color {
                    source: ditherette_wasm::spec::contract::request::Source {
                        width: source.width,
                        height: source.height,
                        data: rgba,
                    },
                    space: *space,
                };
                request.dimensions().map_err(io::Error::other)?;
                Ok(request)
            }
        }
    }

    pub fn identity(&self, source: Dimensions, rgba: &[u8]) -> io::Result<CaseIdentity> {
        let request = self.reference_request(source, rgba)?;
        Ok(CaseIdentity {
            semantics: request.semantics(),
            input: input_digest(source, rgba),
            settings: settings_digest(&request).map_err(io::Error::other)?,
            output: request.dimensions().map_err(io::Error::other)?,
        })
    }

    pub fn reference_subject(&self) -> &'static str {
        match self {
            Self::Processor { settings, .. } => settings.reference_subject(),
            Self::Process { .. } => "spec:process:request:v1",
            Self::Diffusion { .. } => "spec:dither-and-quantize:request:v1",
            Self::Yliluoma { .. } => "spec:dither-and-quantize:request:v1",
            Self::Perturb { .. } => "spec:perturb:request:v1",
            Self::Separable { .. } => "spec:dither-and-quantize:request:v1",
            Self::FieldComponent { component } => component.reference_subject(),
            Self::Quantize { .. } => "spec:quantize:request:v1",
            Self::MetricScores { metric } => metric.reference_subject(),
            Self::ColorForward { space } => match space {
                WorkingSpace::Srgb => "spec:color:srgb:f32-roundtrip-v1",
                WorkingSpace::LinearRgb => "spec:color:linear-rgb:f32-roundtrip-v1",
                WorkingSpace::Oklab => "spec:color:oklab:f32-roundtrip-v1",
                WorkingSpace::Cielab => "spec:color:cielab:f32-roundtrip-v1",
                WorkingSpace::Ycbcr => "spec:color:ycbcr:f32-roundtrip-v1",
                WorkingSpace::Oklch => "spec:color:oklch:f32-roundtrip-v1",
                WorkingSpace::Cielch => "spec:color:cielch:f32-roundtrip-v1",
            },
        }
    }

    pub fn scope(&self) -> super::CallScope {
        match self {
            Self::Processor { .. } => super::CallScope::NativeCompleteCall,
            Self::Process { .. } => super::CallScope::NativeCompleteCall,
            Self::Diffusion { .. } => super::CallScope::NativeCompleteCall,
            Self::Yliluoma { .. } => super::CallScope::NativeCompleteCall,
            Self::Perturb { .. } | Self::Separable { .. } => super::CallScope::NativeCompleteCall,
            Self::FieldComponent { component } => match component {
                ditherette_wasm::bench_subjects::fields::Component::Inverse { .. } => {
                    super::CallScope::NativeInverseConversion
                }
                ditherette_wasm::bench_subjects::fields::Component::Field { .. } => {
                    super::CallScope::NativeFieldEvaluation
                }
                ditherette_wasm::bench_subjects::fields::Component::Placement { .. } => {
                    super::CallScope::NativePlacementMask
                }
                ditherette_wasm::bench_subjects::fields::Component::SourceConversion { .. } => {
                    super::CallScope::NativeSourceConversion
                }
            },
            Self::Quantize { .. } => super::CallScope::NativeCompleteCall,
            Self::ColorForward { .. } => super::CallScope::NativeForwardConversion,
            Self::MetricScores { .. } => super::CallScope::NativeMetricScores,
        }
    }
}

/// Build a normalized identity from the exact frozen recipe, including support policy.
pub fn identity(
    reference: &str,
    source: Dimensions,
    rgba: &[u8],
    output: Dimensions,
) -> io::Result<CaseIdentity> {
    let id = SubjectId::parse(reference).map_err(io::Error::other)?;
    if id.module() != "spec" || id.domain() != "resize" {
        return Err(io::Error::other(
            "native reference must name a frozen spec subject",
        ));
    }
    let recipe = if id.variant() == "scalar" {
        // Preserve existing nearest/area/bilinear identities, which have no support option.
        format!("{}-center-default", id.filter())
    } else {
        format!("{}-{}-center-default", id.filter(), id.variant())
    };
    let semantics = SemanticIdentity {
        operation: Operation::Resize,
        recipe,
        version: 1,
        space: None,
    };
    Ok(CaseIdentity {
        input: input_digest(source, rgba),
        settings: settings_digest(&(semantics.clone(), output, "center-default"))
            .map_err(io::Error::other)?,
        semantics,
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_policy_changes_the_semantic_identity() {
        let dimensions = Dimensions {
            width: 1,
            height: 1,
        };
        for filter in ["bicubic", "lanczos2", "lanczos3"] {
            let fixed = if filter == "bicubic" {
                "catmull-rom"
            } else {
                "fixed"
            };
            let scaled = if filter == "bicubic" {
                "catmull-rom-scale-aware"
            } else {
                "scale-aware"
            };
            let a = identity(
                &format!("spec:resize:{filter}:{fixed}"),
                dimensions,
                &[1, 2, 3, 255],
                dimensions,
            )
            .unwrap();
            let b = identity(
                &format!("spec:resize:{filter}:{scaled}"),
                dimensions,
                &[1, 2, 3, 255],
                dimensions,
            )
            .unwrap();
            assert_ne!(a.semantics, b.semantics);
            assert_ne!(a.settings, b.settings);
        }
        assert!(identity(
            "prod:resize:bicubic:catmull-rom",
            dimensions,
            &[1, 2, 3, 255],
            dimensions
        )
        .is_err());
    }
}
