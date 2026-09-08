//! Test-only, same-target frozen oracle. Never linked into the published package.
//! These are the only semantic modules; no production/core crate dependency exists.

#[path = "../../ditherette-bench/src/verification/identity.rs"]
mod identity;
#[path = "../../ditherette-wasm/src/image/mod.rs"]
pub mod image;
#[path = "../../ditherette-wasm/src/spec/mod.rs"]
pub mod spec;

use ditherette_bench_api::verification::*;
use identity::{input_digest, settings_digest};
use serde::{Deserialize, Serialize};
use spec::contract::request::*;
use wasm_bindgen::prelude::*;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRequest {
    pub source: Dimensions,
    pub rgba: Vec<u8>,
    pub output: Dimensions,
    pub operation: PublicOperation,
    pub identity: CaseIdentity,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantizeSettings {
    pub palette: Vec<image::contracts::PaletteEntry>,
    pub alpha: AlphaPolicy,
    pub matching: MatchPolicy,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeparableSettings {
    pub quantize: QuantizeSettings,
    pub perturb: PerturbPolicy,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YliluomaSettings {
    pub quantize: QuantizeSettings,
    pub size: BayerSize,
    pub placement: Placement,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffusionSettings {
    pub quantize: QuantizeSettings,
    pub kernel: Diffusion,
    pub feedback: DiffusionFeedback,
    pub strength: f32,
    pub serpentine: bool,
    pub placement: Placement,
}

/// Complete frozen recipe; output dimensions belong to the recipe, not the source.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessSettings {
    pub palette: Vec<image::contracts::PaletteEntry>,
    pub recipe: RecipeV1,
}

/// Wire tags mirror the benchmark protocol; settings use only frozen contract types.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PublicOperation {
    Process { settings: ProcessSettings },
    Perturb { settings: PerturbPolicy },
    Separable { settings: SeparableSettings },
    Quantize { settings: QuantizeSettings },
    Yliluoma { settings: YliluomaSettings },
    Diffusion { settings: DiffusionSettings },
    ResizeNearest { anchor: Anchor },
    ResizeArea {},
    ResizeBilinear { anchor: Anchor },
    ResizeBicubic { anchor: Anchor, support: Support },
    ResizeLanczos2 { anchor: Anchor, support: Support },
    ResizeLanczos3 { anchor: Anchor, support: Support },
    ResizeTrilinear { anchor: Anchor },
}

fn quantize<'a>(settings: &'a QuantizeSettings, source: Source<'a>) -> QuantizeRequest<'a> {
    QuantizeRequest {
        version: 1,
        source,
        palette: &settings.palette,
        alpha: settings.alpha,
        matching: settings.matching,
    }
}

impl OracleRequest {
    fn request(&self) -> Request<'_> {
        let source = Source {
            width: self.source.width,
            height: self.source.height,
            data: &self.rgba,
        };
        let resize = match &self.operation {
            PublicOperation::Process { settings } => {
                return Request::Process(ProcessRequest {
                    source,
                    palette: &settings.palette,
                    recipe: settings.recipe,
                })
            }
            PublicOperation::Perturb { settings } => {
                return Request::Perturb(PerturbRequest {
                    version: 1,
                    source,
                    perturb: *settings,
                })
            }
            PublicOperation::Quantize { settings } => {
                return Request::Quantize(quantize(settings, source))
            }
            PublicOperation::Separable { settings } => {
                return Request::DitherAndQuantize(DitherQuantizeRequest {
                    quantize: quantize(&settings.quantize, source),
                    dither: DitherPolicy::Separable {
                        perturb: settings.perturb,
                    },
                })
            }
            PublicOperation::Yliluoma { settings } => {
                return Request::DitherAndQuantize(DitherQuantizeRequest {
                    quantize: quantize(&settings.quantize, source),
                    dither: DitherPolicy::Yliluoma {
                        size: settings.size,
                        placement: settings.placement,
                    },
                })
            }
            PublicOperation::Diffusion { settings } => {
                return Request::DitherAndQuantize(DitherQuantizeRequest {
                    quantize: quantize(&settings.quantize, source),
                    dither: DitherPolicy::Diffusion {
                        kernel: settings.kernel,
                        feedback: settings.feedback,
                        strength: settings.strength,
                        serpentine: settings.serpentine,
                        placement: settings.placement,
                    },
                })
            }
            PublicOperation::ResizeNearest { anchor } => ResizePolicy::Nearest { anchor: *anchor },
            PublicOperation::ResizeArea {} => ResizePolicy::Area {},
            PublicOperation::ResizeBilinear { anchor } => {
                ResizePolicy::Bilinear { anchor: *anchor }
            }
            PublicOperation::ResizeBicubic { anchor, support } => ResizePolicy::Bicubic {
                anchor: *anchor,
                support: *support,
            },
            PublicOperation::ResizeLanczos2 { anchor, support } => ResizePolicy::Lanczos2 {
                anchor: *anchor,
                support: *support,
            },
            PublicOperation::ResizeLanczos3 { anchor, support } => ResizePolicy::Lanczos3 {
                anchor: *anchor,
                support: *support,
            },
            PublicOperation::ResizeTrilinear { anchor } => {
                ResizePolicy::Trilinear { anchor: *anchor }
            }
        };
        Request::Resize(ResizeRequest {
            version: 1,
            source,
            output: Output {
                width: self.output.width,
                height: self.output.height,
                resize,
            },
        })
    }

    /// Compute identities from validated typed settings, not caller-supplied digests.
    pub fn case_identity(&self) -> Result<CaseIdentity, String> {
        let request = self.request();
        let dimensions = request.validate().map_err(|e| e.to_string())?.output;
        if matches!(request, Request::Process(_))
            && (self.output.width != dimensions.width()
                || self.output.height != dimensions.height())
        {
            return Err("Wasm oracle Process output differs from recipe dimensions".into());
        }
        let (operation, recipe, space, settings) = match request {
            Request::Perturb(p) => (
                Operation::Perturb,
                "public-perturb",
                Some(p.perturb.space),
                settings_digest(&("perturb", p.version, p.perturb)),
            ),
            Request::Quantize(q) => (
                Operation::Quantize,
                "public-quantize",
                Some(q.matching.space()),
                settings_digest(&("quantize", q.version, q.palette, q.alpha, q.matching)),
            ),
            Request::DitherAndQuantize(d) => (
                Operation::DitherAndQuantize,
                "public-dither-and-quantize",
                Some(d.quantize.matching.space()),
                settings_digest(&(
                    "dither-and-quantize",
                    d.quantize.version,
                    d.quantize.palette,
                    d.quantize.alpha,
                    d.quantize.matching,
                    d.dither,
                )),
            ),
            Request::Resize(r) => (
                Operation::Resize,
                match r.output.resize {
                    ResizePolicy::Nearest { .. } => "nearest-public-v1",
                    ResizePolicy::Area {} => "area-public-v1",
                    ResizePolicy::Bilinear { .. } => "bilinear-public-v1",
                    ResizePolicy::Bicubic { .. } => "bicubic-public-v1",
                    ResizePolicy::Lanczos2 { .. } => "lanczos2-public-v1",
                    ResizePolicy::Lanczos3 { .. } => "lanczos3-public-v1",
                    ResizePolicy::Trilinear { .. } => "trilinear-public-v1",
                },
                None,
                settings_digest(&(&self.operation, self.output)),
            ),
            Request::Process(p) => (
                Operation::Process,
                "public-process",
                Some(p.recipe.matching.space()),
                settings_digest(&("process", p.palette, p.recipe)),
            ),
        };
        let space = space.map(|value| {
            serde_json::from_value(serde_json::to_value(value).expect("space tag"))
                .expect("shared space tag")
        });
        Ok(CaseIdentity {
            semantics: SemanticIdentity {
                operation,
                recipe: recipe.into(),
                version: 1,
                space,
            },
            input: input_digest(self.source, &self.rgba),
            settings: settings.map_err(|e| e.to_string())?,
            output: Dimensions {
                width: dimensions.width(),
                height: dimensions.height(),
            },
        })
    }

    /// Reject identity/input/settings drift before invoking the frozen pipeline.
    pub fn execute(&self) -> Result<OracleOutput, String> {
        let case = self.case_identity()?;
        if case != self.identity {
            return Err("Wasm oracle case identity differs".into());
        }
        let output = self.execute_request(self.request())?;
        Ok(OracleOutput { case, output })
    }

    /// Derive the declared prime from a validated measured request, using frozen methods only.
    pub fn prime_output(&self, prime: &str) -> Result<VerificationOutput, String> {
        if self.case_identity()? != self.identity {
            return Err("Wasm oracle case identity differs".into());
        }
        let request = match (prime, self.request()) {
            ("same-call", request) => request,
            ("resize", Request::Process(input)) => Request::Resize(ResizeRequest {
                version: 1,
                source: input.source,
                output: input.recipe.output,
            }),
            ("perturb", Request::DitherAndQuantize(input)) => {
                let DitherPolicy::Separable { perturb } = input.dither else {
                    return Err("prime requires separable settings".into());
                };
                Request::Perturb(PerturbRequest {
                    version: 1,
                    source: input.quantize.source,
                    perturb,
                })
            }
            ("no-dither", Request::Quantize(quantize)) => {
                Request::DitherAndQuantize(DitherQuantizeRequest {
                    quantize,
                    dither: DitherPolicy::None {},
                })
            }
            _ => return Err("stage prime does not match the measured operation".into()),
        };
        self.execute_request(request)
    }

    fn execute_request(&self, request: Request<'_>) -> Result<VerificationOutput, String> {
        let dimensions = request.validate().map_err(|e| e.to_string())?.output;
        let dimensions = Dimensions {
            width: dimensions.width(),
            height: dimensions.height(),
        };
        let output = match spec::pipeline::execute(request).map_err(|e| e.to_string())? {
            spec::pipeline::ProcessedImage::Rgba8(image) => VerificationOutput {
                dimensions,
                pixels: Pixels::Rgba8 {
                    data: image.into_vec(),
                },
                warnings: Vec::new(),
            },
            spec::pipeline::ProcessedImage::Indexed(image) => VerificationOutput {
                dimensions,
                pixels: Pixels::Indexed8 {
                    indices: image.indices.into_vec(),
                    palette_rgba: image.palette.rgba,
                    transparent_index: image.palette.transparent_index,
                },
                warnings: image
                    .warnings
                    .into_iter()
                    .map(|w| Warning {
                        code: match w.code {
                            image::contracts::WarningCode::PaletteTruncated => {
                                WarningCode::PaletteTruncated
                            }
                            image::contracts::WarningCode::TransparentOnly => {
                                WarningCode::TransparentOnly
                            }
                            image::contracts::WarningCode::TransparentFallback => {
                                WarningCode::TransparentFallback
                            }
                        },
                        message: w.message,
                    })
                    .collect(),
            },
        };
        Ok(output)
    }
}

/// One untimed JSON request/result boundary. No package code or caller output enters the oracle.
#[wasm_bindgen]
pub fn evaluate(request: &str) -> Result<String, JsError> {
    let request: OracleRequest = serde_json::from_str(request)?;
    let output = request.execute().map_err(|e| JsError::new(&e))?;
    Ok(serde_json::to_string(&output)?)
}

/// Untimed same-target frozen prime output; never executes production or benchmark timers.
#[wasm_bindgen]
pub fn evaluate_prime(request: &str, prime: &str) -> Result<String, JsError> {
    let request: OracleRequest = serde_json::from_str(request)?;
    let output = request.prime_output(prime).map_err(|e| JsError::new(&e))?;
    Ok(serde_json::to_string(&output)?)
}
