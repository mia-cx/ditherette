//! S37 workload fragment. The coordinator adds execution-policy roles and measurement budgets.
//! Generating this file runs no timing. Required-thread trials use Chromium and Firefox host workers.
#[allow(dead_code)]
#[path = "preparation_integration_plan.rs"]
mod preparation_plan;

use ditherette_bench::paired::{
    browser::{
        BrowserBackend, BrowserCase, BrowserExecution, BrowserPreparation, CacheCapability,
        PreparationCapability, PublicOperation, ThreadRoles, Threads,
    },
    preparation::SamplePrime,
    process::ProcessSettings,
    quantize::{AlphaPolicy, MatchPolicy, QuantizeSettings},
    yliluoma::YliluomaSettings,
    ApplicationCache, CallScope, Measurement, PairCase, SampleMode,
};
use ditherette_bench_api::verification::Dimensions;
use ditherette_wasm::spec::contract::request as spec;
use std::{env, fs::OpenOptions, io, io::Write};

/// Bounded cold calls and warm final-output controls with identical semantic subjects in both roles.
/// Both roles require a host-worker pool. The coordinator separately binds forced mixing policy evidence.
pub fn cases() -> io::Result<Vec<PairCase>> {
    let adaptive = spec::Placement::Adaptive {
        radius: 2,
        threshold: 5.0,
        softness: 10.0,
    };
    let definitions = [
        (
            "tiny-p2-bayer2-srgb",
            9,
            7,
            2,
            spec::BayerSize::Two,
            MatchPolicy::SrgbEuclidean,
            spec::Placement::Everywhere {},
        ),
        (
            "small-p4-bayer4-srgb",
            33,
            25,
            4,
            spec::BayerSize::Four,
            MatchPolicy::SrgbEuclidean,
            spec::Placement::Everywhere {},
        ),
        (
            "medium-p8-bayer4-srgb-adaptive",
            65,
            49,
            8,
            spec::BayerSize::Four,
            MatchPolicy::SrgbEuclidean,
            adaptive,
        ),
        (
            "process-p4-bayer4-oklch-adaptive",
            65,
            49,
            4,
            spec::BayerSize::Four,
            MatchPolicy::OklchCircularHue,
            adaptive,
        ),
    ];
    let mut cases = Vec::new();
    for (name, width, height, count, size, matching, placement) in definitions {
        let source = Dimensions { width, height };
        let rgba = preparation_plan::source_rgba(source);
        let quantize = QuantizeSettings {
            palette: preparation_plan::palette(count),
            alpha: AlphaPolicy::Premultiplied {},
            matching,
        };
        let process = name.starts_with("process-");
        let output = if process {
            Dimensions {
                width: 33,
                height: 25,
            }
        } else {
            source
        };
        let operation = if process {
            PublicOperation::Process {
                settings: ProcessSettings {
                    palette: quantize.palette,
                    recipe: spec::RecipeV1 {
                        version: 1,
                        output: spec::Output {
                            width: output.width,
                            height: output.height,
                            resize: spec::ResizePolicy::Nearest {
                                anchor: spec::Anchor::Center,
                            },
                        },
                        alpha: spec::AlphaPolicy::Premultiplied {},
                        matching: spec::MatchPolicy::OklchCircularHue,
                        dither: spec::DitherPolicy::Yliluoma { size, placement },
                    },
                },
            }
        } else {
            PublicOperation::Yliluoma {
                settings: YliluomaSettings {
                    quantize,
                    size,
                    placement,
                },
            }
        };
        for application_cache in [ApplicationCache::Cold, ApplicationCache::Warm] {
            let warm = application_cache == ApplicationCache::Warm;
            cases.push(PairCase {
                name: format!(
                    "yliluoma-{name}-{}",
                    if warm { "final-hit" } else { "cold" }
                ),
                source,
                rgba: rgba.clone(),
                identity: operation.identity(source, &rgba, output)?,
                reference_subject: operation.reference_subject().into(),
                accepted_subject: operation.subject(BrowserBackend::Package).into(),
                candidate_subject: operation.subject(BrowserBackend::Package).into(),
                native: None,
                browser: Some(BrowserCase {
                    retained_output_limit_bytes: None,
                    row_policy: None,
                    execution: Some(BrowserExecution::HostWorker),
                    operation: operation.clone(),
                    accepted: BrowserBackend::Package,
                    candidate: BrowserBackend::Package,
                    preparation: if warm {
                        BrowserPreparation::PrimedSample
                    } else {
                        BrowserPreparation::FreshInstance
                    },
                    cache: CacheCapability::Roles {
                        accepted: PreparationCapability::ImageStages,
                        candidate: PreparationCapability::ImageStages,
                        sample_prime: warm.then_some(SamplePrime::SameCall),
                    },
                    progress: None,
                    threads: Some(ThreadRoles {
                        accepted: Threads::Required,
                        candidate: Threads::Required,
                    }),
                    measure_nonexact: false,
                }),
                // Existing complete-call defaults. Root freezes final budgets before measurement.
                measurement: Measurement {
                    mode: SampleMode::SingleCall,
                    scope: CallScope::CompleteCall,
                    application_cache,
                    samples: 20,
                    measurement_ms: 10_000,
                    warmup_ms: 50,
                    target_sample_ms: 1,
                },
            });
        }
    }
    Ok(cases)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [destination] = args.as_slice() else {
        return Err(io::Error::other("usage: yliluoma_row_band_cases NEW_JSON"));
    };
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?
        .write_all(&serde_json::to_vec_pretty(&cases()?).map_err(io::Error::other)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ditherette_wasm::bench_subjects::{field_calls, preparation::CompleteCall};

    #[test]
    fn bounded_public_subjects_match_frozen_output_and_cache_controls() {
        // Root's shared transport change enables CompleteCall in the host-worker validator.
        // This fragment checks its operation/identity/oracle contract without running that transport.
        for pair in cases().unwrap().chunks_exact(2) {
            let case = &pair[0];
            let warm = &pair[1];
            assert_eq!(case.identity, warm.identity);
            assert_eq!(case.rgba, warm.rgba);
            let browser = case.browser.as_ref().unwrap();
            let oracle: ditherette_bench_oracle::OracleRequest =
                serde_json::from_value(serde_json::json!({
                    "source": case.source, "rgba": case.rgba, "output": case.identity.output,
                    "operation": browser.operation, "identity": case.identity,
                }))
                .unwrap();
            let expected = oracle.execute().unwrap().output;
            assert_eq!(oracle.prime_output("same-call").unwrap(), expected);
            let request = browser
                .operation
                .processing_request(case.source, &case.rgba)
                .unwrap()
                .unwrap();
            let mut processor = field_calls::processor().unwrap();
            let call = CompleteCall::new(&request).unwrap();
            for _ in 0..2 {
                assert_eq!(
                    call.output(&mut processor, &case.rgba)
                        .unwrap()
                        .verification(),
                    expected,
                    "{}",
                    case.name
                );
            }
            assert_eq!(
                warm.browser.as_ref().unwrap().cache.sample_prime(),
                Some(SamplePrime::SameCall)
            );
        }
    }
}
