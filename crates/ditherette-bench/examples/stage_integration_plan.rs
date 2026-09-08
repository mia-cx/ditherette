//! The declared S32 matrix reuses comparable S31 fixtures without running timers.
#[allow(dead_code)]
#[path = "preparation_integration_plan.rs"]
mod preparation_plan;

use ditherette_bench::paired::{
    browser::{
        self, BrowserBackend, BrowserPreparation, CacheCapability, PreparationCapability,
        PublicOperation,
    },
    coordinator::validate_experiment,
    fields::{BayerSize, Field, PerturbPolicy, Placement, SeparableSettings, WorkingSpace},
    native::NativeOperation,
    preparation::{ProcessorSettings, SamplePrime},
    quantize::{AlphaPolicy, MatchPolicy, QuantizeSettings},
    *,
};
use ditherette_bench_api::verification::Dimensions;
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

pub fn experiment(public: bool, notes: String) -> io::Result<Experiment> {
    let mut plan = preparation_plan::experiment(public, notes)?;
    plan.label = "image-stage-reuse".into();
    // Keep the exact S31 quantize, Lanczos3, and Process workloads. Replace trilinear
    // with a separable call whose existing materialized RGBA stage can be shared.
    for case in &mut plan.cases {
        let replace = case.name.starts_with("resize-trilinear-");
        if replace {
            case.source = Dimensions {
                width: 65,
                height: 49,
            };
            case.rgba = preparation_plan::source_rgba(case.source);
            let settings = SeparableSettings {
                quantize: QuantizeSettings {
                    palette: preparation_plan::palette(16),
                    alpha: AlphaPolicy::Premultiplied {},
                    matching: MatchPolicy::SrgbEuclidean,
                },
                perturb: PerturbPolicy {
                    field: Field::Bayer {
                        size: BayerSize::Four,
                    },
                    space: WorkingSpace::Oklab,
                    strength: 0.7,
                    placement: Placement::Everywhere {},
                },
            };
            let native = NativeOperation::Processor {
                settings: ProcessorSettings::Separable {
                    settings: settings.clone(),
                },
                cache: CacheCapability::None,
            };
            let public_operation = PublicOperation::Separable { settings };
            case.identity = if public {
                public_operation.identity(case.source, &case.rgba, case.source)?
            } else {
                native.identity(case.source, &case.rgba)?
            };
            case.reference_subject = "spec:dither-and-quantize:request:v1".into();
            let subject = if public {
                public_operation.subject(BrowserBackend::Package)
            } else {
                ditherette_wasm::bench_subjects::field_calls::SEPARABLE_SUBJECT
            };
            case.accepted_subject = subject.into();
            case.candidate_subject = subject.into();
            case.native = (!public).then_some(native);
            if let Some(browser) = &mut case.browser {
                browser.operation = public_operation;
            }
            case.name = format!(
                "separable-bayer4-oklab-p16-65x49-{}",
                if case.measurement.application_cache == ApplicationCache::Cold {
                    "cold"
                } else {
                    "warm"
                }
            );
        }
        let prime = if replace {
            SamplePrime::Perturb
        } else if case.name.starts_with("quantize-") {
            SamplePrime::NoDither
        } else if case.name.starts_with("process-") {
            SamplePrime::Resize
        } else {
            SamplePrime::SameCall
        };
        let warm = case.measurement.application_cache == ApplicationCache::Warm;
        let cache = CacheCapability::Roles {
            accepted: PreparationCapability::Preparation,
            candidate: PreparationCapability::ImageStages,
            sample_prime: warm.then_some(prime),
        };
        if let Some(browser) = &mut case.browser {
            browser.cache = cache;
            browser.preparation = if warm {
                BrowserPreparation::PrimedSample
            } else {
                BrowserPreparation::FreshInstance
            };
        } else if let Some(NativeOperation::Processor { cache: actual, .. }) = &mut case.native {
            *actual = cache;
        }
    }
    validate_experiment(&plan)?;
    Ok(plan)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [kind, destination, notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: stage_integration_plan native|public DESTINATION HOST_LOAD_NOTES",
        ));
    };
    if !matches!(kind.as_str(), "native" | "public") {
        return Err(io::Error::other("unknown plan kind"));
    }
    let plan = experiment(kind == "public", notes.clone())?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?
        .write_all(&serde_json::to_vec_pretty(&plan).map_err(io::Error::other)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ditherette_wasm::bench_subjects::{
        self, field_calls, preparation::CompleteCall, reference::ReferenceRequest, BenchSubject,
    };
    use ditherette_wasm::spec::contract::request as spec;

    fn frozen(
        request: &ReferenceRequest<'_>,
    ) -> ditherette_bench_api::verification::VerificationOutput {
        let subject = match request {
            ReferenceRequest::Processing(spec::Request::Resize(_)) => "spec:resize:request:v1",
            ReferenceRequest::Processing(spec::Request::Quantize(_)) => "spec:quantize:request:v1",
            ReferenceRequest::Processing(spec::Request::Perturb(_)) => "spec:perturb:request:v1",
            ReferenceRequest::Processing(spec::Request::DitherAndQuantize(_)) => {
                "spec:dither-and-quantize:request:v1"
            }
            ReferenceRequest::Processing(spec::Request::Process(_)) => "spec:process:request:v1",
            _ => unreachable!("processing fixture"),
        };
        bench_subjects::bench_subjects()
            .into_iter()
            .find_map(|entry| match entry {
                BenchSubject::Conformance(entry) if entry.descriptor.id.as_str() == subject => {
                    Some((entry.run)(request).unwrap())
                }
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn matrix_preserves_explicit_lifecycle_and_historical_metadata() {
        let historical =
            serde_json::json!({"roles":{"accepted":"uncached","candidate":"preparation"}});
        let parsed: CacheCapability = serde_json::from_value(historical.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), historical);
        for public in [false, true] {
            let plan = experiment(public, "untimed validation".into()).unwrap();
            assert_eq!(plan.cases.len() * plan.pairs * 2 * 4, 128);
            for case in plan.cases {
                let cache = case.browser.as_ref().map(|b| b.cache).unwrap_or_else(|| {
                    match case.native.as_ref().unwrap() {
                        NativeOperation::Processor { cache, .. } => *cache,
                        _ => unreachable!(),
                    }
                });
                assert_eq!(
                    cache.sample_prime().is_some(),
                    case.measurement.application_cache == ApplicationCache::Warm
                );
                assert_eq!(case.measurement.samples, 20);
                let mut invalid = case.clone();
                invalid.measurement.mode = SampleMode::Throughput;
                assert!(browser::validate_case(&invalid).is_err());
                if let Some(browser) = &mut invalid.browser {
                    browser.preparation = BrowserPreparation::PrimedInstance;
                    invalid.measurement = case.measurement;
                    if cache.sample_prime().is_some() {
                        assert!(browser::validate_case(&invalid).is_err());
                    }
                }
            }
        }
    }

    #[test]
    fn every_native_prime_and_measured_call_matches_its_frozen_output() {
        for case in experiment(false, "untimed conformance".into())
            .unwrap()
            .cases
        {
            let NativeOperation::Processor { settings, cache } = case.native.as_ref().unwrap()
            else {
                unreachable!()
            };
            let measured = settings.reference_request(case.source, &case.rgba).unwrap();
            let expected = frozen(&measured);
            for _ in 0..2 {
                let mut instance = field_calls::processor().unwrap();
                if let Some(prime) = cache.sample_prime() {
                    let request = settings
                        .prime_request(prime, case.source, &case.rgba)
                        .unwrap();
                    assert_eq!(
                        CompleteCall::new(&request)
                            .unwrap()
                            .output(&mut instance, &case.rgba)
                            .unwrap()
                            .verification(),
                        frozen(&request),
                        "{} prime",
                        case.name
                    );
                }
                assert_eq!(
                    CompleteCall::new(&measured)
                        .unwrap()
                        .output(&mut instance, &case.rgba)
                        .unwrap()
                        .verification(),
                    expected,
                    "{} measured",
                    case.name
                );
            }
        }
    }

    #[test]
    fn isolated_oracle_derives_each_prime_from_the_measured_identity() {
        let native = experiment(false, "untimed native reference".into()).unwrap();
        for (case, native_case) in experiment(true, "untimed oracle".into())
            .unwrap()
            .cases
            .into_iter()
            .zip(native.cases)
        {
            let browser = case.browser.as_ref().unwrap();
            let Some(prime) = browser.cache.sample_prime() else {
                continue;
            };
            let request: ditherette_bench_oracle::OracleRequest = serde_json::from_value(serde_json::json!({"source":case.source,"rgba":case.rgba,"output":case.identity.output,"operation":browser.operation,"identity":case.identity})).unwrap();
            let NativeOperation::Processor { settings, .. } = native_case.native.as_ref().unwrap()
            else {
                unreachable!()
            };
            let expected = frozen(
                &settings
                    .prime_request(prime, native_case.source, &native_case.rgba)
                    .unwrap(),
            );
            let tag = serde_json::to_value(prime).unwrap();
            assert_eq!(
                request.prime_output(tag.as_str().unwrap()).unwrap(),
                expected
            );
            assert!(request.prime_output("unknown").is_err());
        }
    }
}
