//! Eight declared complete pipelines. Generating plans or frozen fixtures never measures calls.
use ditherette_bench::paired::{
    browser::{BrowserBackend, BrowserCase, BrowserPreparation, CacheCapability, PublicOperation},
    coordinator::validate_experiment,
    native::NativeOperation,
    process::ProcessSettings,
    *,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use ditherette_wasm::{
    bench_subjects::{
        self,
        process::{PROCESS_SUBJECT, STAGED_SUBJECT},
        BenchSubject,
    },
    image::contracts::PaletteEntry,
    spec::contract::request::*,
};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn experiment(public: bool, notes: String) -> io::Result<Experiment> {
    let everywhere = Placement::Everywhere {};
    let field = |field| DitherPolicy::Separable {
        perturb: PerturbPolicy {
            field,
            space: WorkingSpace::Srgb,
            strength: 0.7,
            placement: everywhere,
        },
    };
    let diffusion = |kernel| DitherPolicy::Diffusion {
        kernel,
        strength: 0.7,
        placement: everywhere,
        feedback: DiffusionFeedback::SrgbBytes,
        serpentine: true,
    };
    let recipes = [
        (
            "nearest-none",
            ResizePolicy::Nearest {
                anchor: Anchor::Center,
            },
            DitherPolicy::None {},
        ),
        (
            "area-bayer",
            ResizePolicy::Area {},
            field(Field::Bayer {
                size: BayerSize::Four,
            }),
        ),
        (
            "bilinear-random",
            ResizePolicy::Bilinear {
                anchor: Anchor::Center,
            },
            field(Field::Random { seed: 42 }),
        ),
        (
            "bicubic-blue-noise",
            ResizePolicy::Bicubic {
                anchor: Anchor::Center,
                support: Support::Fixed,
            },
            field(Field::BlueNoise {}),
        ),
        (
            "lanczos2-floyd-steinberg",
            ResizePolicy::Lanczos2 {
                anchor: Anchor::Center,
                support: Support::ScaleAware,
            },
            diffusion(Diffusion::FloydSteinberg),
        ),
        (
            "lanczos3-atkinson",
            ResizePolicy::Lanczos3 {
                anchor: Anchor::Center,
                support: Support::ScaleAware,
            },
            diffusion(Diffusion::Atkinson),
        ),
        (
            "trilinear-sierra-lite",
            ResizePolicy::Trilinear {
                anchor: Anchor::Center,
            },
            diffusion(Diffusion::SierraLite),
        ),
        (
            "nearest-yliluoma",
            ResizePolicy::Nearest {
                anchor: Anchor::BottomRight,
            },
            DitherPolicy::Yliluoma {
                size: BayerSize::Four,
                placement: everywhere,
            },
        ),
    ];
    let mut cases = Vec::new();
    for (name, resize, dither) in recipes {
        let mixing = matches!(dither, DitherPolicy::Yliluoma { .. });
        // Single-call latency has no batching. Larger cheap-filter inputs avoid tiny timer quanta.
        let (source, output) = match resize {
            ResizePolicy::Nearest { .. } if !mixing => (
                Dimensions {
                    width: 512,
                    height: 384,
                },
                Dimensions {
                    width: 257,
                    height: 193,
                },
            ),
            ResizePolicy::Area {} | ResizePolicy::Bilinear { .. } => (
                Dimensions {
                    width: 256,
                    height: 192,
                },
                Dimensions {
                    width: 129,
                    height: 97,
                },
            ),
            ResizePolicy::Trilinear { .. } => (
                Dimensions {
                    width: 256,
                    height: 192,
                },
                Dimensions {
                    width: 65,
                    height: 49,
                },
            ),
            _ if mixing => (
                Dimensions {
                    width: 8,
                    height: 6,
                },
                Dimensions {
                    width: 4,
                    height: 3,
                },
            ),
            _ => (
                Dimensions {
                    width: 129,
                    height: 97,
                },
                Dimensions {
                    width: 65,
                    height: 49,
                },
            ),
        };
        let rgba: Vec<u8> = (0..source.width * source.height)
            .flat_map(|i| {
                [
                    (i * 73 + 17) as u8,
                    (i * 31 + 71) as u8,
                    (i * 43 + 113) as u8,
                    if i % 17 == 0 { 0 } else { 255 },
                ]
            })
            .collect();
        let settings = ProcessSettings {
            palette: (0..if mixing { 4 } else { 16 })
                .map(|i| PaletteEntry::Color {
                    rgb: [(i * 73) as u8, (i * 31 + 17) as u8, (i * 43 + 119) as u8],
                })
                .collect(),
            recipe: RecipeV1 {
                version: 1,
                output: Output {
                    width: output.width,
                    height: output.height,
                    resize,
                },
                alpha: AlphaPolicy::Matte { rgb: [29, 71, 211] },
                matching: MatchPolicy::SrgbEuclidean,
                dither,
            },
        };
        let native = NativeOperation::Process {
            settings: settings.clone(),
        };
        let operation = PublicOperation::Process { settings };
        cases.push(PairCase {
            name: name.into(),
            identity: native.identity(source, &rgba)?,
            source,
            rgba,
            reference_subject: "spec:process:request:v1".into(),
            accepted_subject: if public {
                operation.subject(BrowserBackend::PackageStaged)
            } else {
                STAGED_SUBJECT
            }
            .into(),
            candidate_subject: if public {
                operation.subject(BrowserBackend::Package)
            } else {
                PROCESS_SUBJECT
            }
            .into(),
            measurement: Measurement {
                mode: SampleMode::SingleCall,
                scope: if public {
                    CallScope::CompleteCall
                } else {
                    CallScope::NativeCompleteCall
                },
                application_cache: ApplicationCache::NotApplicable,
                samples: 20,
                warmup_ms: 50,
                measurement_ms: 10_000,
                target_sample_ms: 2,
            },
            browser: public.then_some(BrowserCase {
                operation,
                accepted: BrowserBackend::PackageStaged,
                candidate: BrowserBackend::Package,
                preparation: BrowserPreparation::PrimedInstance,
                cache: CacheCapability::None,
                // Approved inherited area rounding diagnostic. The frozen gate remains nonexact.
                measure_nonexact: matches!(resize, ResizePolicy::Area {}),
            }),
            native: (!public).then_some(native),
        });
    }
    let experiment = Experiment { label: "S30 actual staged production versus complete Process; frozen drift diagnostic, staged equality mandatory".into(), reference_state: ReferenceState::Frozen, pairs: 2, host_load_notes: notes, cases };
    validate_experiment(&experiment)?;
    Ok(experiment)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [kind, output, notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: process_integration_plan native|public|conformance NEW_JSON HOST_NOTES",
        ));
    };
    let value = match kind.as_str() {
        "native" | "public" => serde_json::to_value(experiment(kind == "public", notes.clone())?)
            .map_err(io::Error::other)?,
        "conformance" => {
            let registry = bench_subjects::bench_subjects();
            let fixtures: Vec<_> = experiment(true, notes.clone())?.cases.into_iter().map(|case| {
                let operation = case.browser.as_ref().unwrap().operation.clone();
                let request = operation.processing_request(case.source, &case.rgba)?.unwrap();
                let run = |id| registry.iter().find_map(|s| match s { BenchSubject::Conformance(s) if s.descriptor.id.as_str() == id => Some(s), _ => None }).map(|s| (s.run)(&request)).unwrap().map_err(io::Error::other);
                let reference = run("spec:process:request:v1")?;
                let production = run(PROCESS_SUBJECT)?;
                let staged = run(STAGED_SUBJECT)?;
                if production != staged { return Err(io::Error::other("Process differs from staged production")); }
                let resize_attribution = if production != reference {
                    let PublicOperation::Process { settings } = &operation else { unreachable!("Process matrix") };
                    let source = Source { width: case.source.width, height: case.source.height, data: &case.rgba };
                    let resize_request = ResizeRequest { version: 1, source, output: settings.recipe.output };
                    let frozen_resize = bench_subjects::verification::reference_resize(&resize_request).map_err(io::Error::other)?;
                    let production_resize = bench_subjects::verification::production_resize(&resize_request).map_err(io::Error::other)?;
                    let ditherette_bench_api::verification::Pixels::Rgba8 { data } = &production_resize.pixels else { unreachable!("resize RGBA8") };
                    let indexed = ditherette_wasm::spec::pipeline::dither_and_quantize(DitherQuantizeRequest {
                        quantize: QuantizeRequest { version: 1, source: Source { width: settings.recipe.output.width, height: settings.recipe.output.height, data }, palette: &settings.palette, alpha: settings.recipe.alpha, matching: settings.recipe.matching },
                        dither: settings.recipe.dither,
                    }).map_err(io::Error::other)?;
                    let frozen_dither_after_production_resize = bench_subjects::verification::indexed_output(&indexed);
                    if production != frozen_dither_after_production_resize { return Err(io::Error::other("frozen difference is not explained by inherited resize alone")); }
                    let output = case.identity.output;
                    let resize_operation = PublicOperation::ResizeArea {};
                    if !matches!(settings.recipe.output.resize, ResizePolicy::Area {}) {
                        return Err(io::Error::other("only inherited area drift is approved"));
                    }
                    let mut post_settings = settings.clone();
                    post_settings.recipe.output.resize = ResizePolicy::Nearest { anchor: Anchor::Center };
                    let post_operation = PublicOperation::Process { settings: post_settings };
                    let resize_probe = serde_json::json!({"name":"area-resize-attribution","source":case.source,"rgba":case.rgba,"identity":resize_operation.identity(case.source,&case.rgba,output)?,"operation":resize_operation,"reference":frozen_resize});
                    let post_resize_probe = serde_json::json!({"name":"area-post-resize-attribution","source":output,"rgba":data,"identity":post_operation.identity(output,data,output)?,"operation":post_operation,"reference":frozen_dither_after_production_resize});
                    Some(serde_json::json!({"frozen_resize":frozen_resize,"production_resize":production_resize,"frozen_dither_after_production_resize":frozen_dither_after_production_resize,"resize_probe":resize_probe,"post_resize_probe":post_resize_probe}))
                } else { None };
                Ok(serde_json::json!({"name":case.name,"identity":case.identity,"source":case.source,"rgba":case.rgba,"operation":operation,"reference":reference,"production":production,"staged":staged,"resize_attribution":resize_attribution}))
            }).collect::<io::Result<_>>()?;
            serde_json::to_value(fixtures).map_err(io::Error::other)?
        }
        _ => return Err(io::Error::other("unknown plan kind")),
    };
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?
        .write_all(&serde_json::to_vec_pretty(&value).map_err(io::Error::other)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eight_complete_pipeline_cases_bind_staged_and_process_to_one_recipe() {
        let native = experiment(false, "test".into()).unwrap();
        let public = experiment(true, "test".into()).unwrap();
        assert_eq!((native.cases.len() + 3 * public.cases.len()) * 2 * 2, 128);
        for (n, p) in native.cases.iter().zip(&public.cases) {
            assert_eq!(n.identity, p.identity);
            assert_ne!(n.identity.output, n.source);
            ditherette_bench::paired::browser::validate_case(n).unwrap();
            ditherette_bench::paired::browser::validate_case(p).unwrap();
            assert_eq!(p.measurement.samples, 20);
            assert_eq!(p.measurement.measurement_ms, 10_000);
            assert_eq!(
                p.browser.as_ref().unwrap().measure_nonexact,
                p.name == "area-bayer"
            );
        }
    }
}
