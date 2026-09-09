//! S41 fixed release plans. Generation validates identities but runs no processing or timing.
#[allow(dead_code, unused_imports)]
#[path = "stage_integration_plan.rs"]
mod anchors;
#[path = "support/row_fields.rs"]
mod fields;
#[allow(dead_code)]
#[path = "preparation_integration_plan.rs"]
mod fixtures;
#[allow(dead_code)]
#[path = "yliluoma_row_band_cases.rs"]
mod mixing;

use ditherette_bench::paired::{
    browser::*, coordinator::validate_experiment, preparation::SamplePrime,
    process::ProcessSettings, quantize::*, *,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use ditherette_wasm::spec::contract::request as spec;
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn dims(width: u32, height: u32) -> Dimensions {
    Dimensions { width, height }
}

fn call(
    name: &str,
    source: Dimensions,
    output: Dimensions,
    operation: PublicOperation,
) -> io::Result<PairCase> {
    let rgba = fixtures::source_rgba(source);
    let measure_nonexact = matches!(
        operation,
        PublicOperation::ResizeArea {}
            | PublicOperation::ResizeBilinear { .. }
            | PublicOperation::ResizeLanczos3 { .. }
    );
    Ok(PairCase {
        name: name.into(),
        identity: operation.identity(source, &rgba, output)?,
        source,
        rgba,
        reference_subject: operation.reference_subject().into(),
        accepted_subject: operation.subject(BrowserBackend::Package).into(),
        candidate_subject: operation.subject(BrowserBackend::Package).into(),
        native: None,
        browser: Some(BrowserCase {
            retained_output_limit_bytes: None,
            operation,
            accepted: BrowserBackend::Package,
            candidate: BrowserBackend::Package,
            preparation: BrowserPreparation::FreshInstance,
            cache: CacheCapability::Roles {
                accepted: PreparationCapability::ImageStages,
                candidate: PreparationCapability::ImageStages,
                sample_prime: None,
            },
            execution: Some(BrowserExecution::HostWorker),
            row_policy: None,
            progress: None,
            threads: None,
            measure_nonexact,
        }),
        measurement: Measurement {
            mode: SampleMode::SingleCall,
            scope: CallScope::CompleteCall,
            application_cache: ApplicationCache::Cold,
            samples: 20,
            warmup_ms: 50,
            measurement_ms: 10_000,
            target_sample_ms: 1,
        },
    })
}

fn warm(mut case: PairCase) -> PairCase {
    case.name.push_str("-final-hit");
    case.measurement.application_cache = ApplicationCache::Warm;
    let browser = case.browser.as_mut().unwrap();
    browser.preparation = BrowserPreparation::PrimedSample;
    browser.cache = CacheCapability::Roles {
        accepted: PreparationCapability::ImageStages,
        candidate: PreparationCapability::ImageStages,
        sample_prime: Some(SamplePrime::SameCall),
    };
    case
}

fn direct() -> QuantizeSettings {
    let mut palette = fixtures::palette(15);
    palette.push(PaletteEntry::Transparent {});
    QuantizeSettings {
        palette,
        alpha: AlphaPolicy::Preserve { threshold: 127.5 },
        matching: MatchPolicy::SrgbEuclidean,
    }
}

fn process(
    output: Dimensions,
    kernel: Option<spec::Diffusion>,
    matching: spec::MatchPolicy,
) -> PublicOperation {
    PublicOperation::Process {
        settings: ProcessSettings {
            palette: direct().palette,
            recipe: spec::RecipeV1 {
                version: 1,
                output: spec::Output {
                    width: output.width,
                    height: output.height,
                    resize: spec::ResizePolicy::Nearest {
                        anchor: spec::Anchor::Center,
                    },
                },
                alpha: spec::AlphaPolicy::Preserve { threshold: 127.5 },
                matching,
                dither: kernel.map_or(spec::DitherPolicy::None {}, |kernel| {
                    spec::DitherPolicy::Diffusion {
                        kernel,
                        feedback: spec::DiffusionFeedback::SrgbBytes,
                        strength: 0.7,
                        serpentine: true,
                        placement: spec::Placement::Everywhere {},
                    }
                }),
            },
        },
    }
}

fn release() -> io::Result<Vec<PairCase>> {
    let mut cases = Vec::new();
    for (name, source, output, operation) in [
        (
            "preview-nearest",
            dims(512, 384),
            dims(256, 192),
            PublicOperation::ResizeNearest {
                anchor: Anchor::Center,
            },
        ),
        (
            "common-direct-srgb16",
            dims(2048, 1536),
            dims(2048, 1536),
            PublicOperation::Quantize { settings: direct() },
        ),
        (
            "large-nearest",
            dims(4096, 2048),
            dims(2048, 1024),
            PublicOperation::ResizeNearest {
                anchor: Anchor::Center,
            },
        ),
        (
            "area-selected",
            dims(1537, 1025),
            dims(769, 513),
            PublicOperation::ResizeArea {},
        ),
        (
            "bilinear-selected",
            dims(1537, 1025),
            dims(769, 513),
            PublicOperation::ResizeBilinear {
                anchor: Anchor::Center,
            },
        ),
        (
            "lanczos3-selected",
            dims(2048, 1536),
            dims(512, 384),
            PublicOperation::ResizeLanczos3 {
                anchor: Anchor::Center,
                support: Support::ScaleAware,
            },
        ),
        (
            "common-process-floyd-steinberg",
            dims(2048, 1536),
            dims(2048, 1536),
            process(
                dims(2048, 1536),
                Some(spec::Diffusion::FloydSteinberg),
                spec::MatchPolicy::SrgbEuclidean,
            ),
        ),
        (
            "preview-process-sierra-oklab",
            dims(512, 384),
            dims(256, 192),
            process(
                dims(256, 192),
                Some(spec::Diffusion::Sierra),
                spec::MatchPolicy::OklabEuclidean,
            ),
        ),
        (
            "extra-trilinear",
            dims(257, 193),
            dims(97, 73),
            PublicOperation::ResizeTrilinear {
                anchor: Anchor::Center,
            },
        ),
    ] {
        cases.push(call(name, source, output, operation)?);
    }
    for (name, operation) in fields::recipes() {
        let source = match name.as_str() {
            "perturb-srgb16-random" => dims(769, 513),
            "separable-oklab64-blue-adaptive2" => dims(65, 49),
            _ => continue,
        };
        cases.push(call(&name, source, source, fields::public(&operation))?);
    }
    for mut case in mixing::cases()?.into_iter().filter(|case| {
        case.measurement.application_cache == ApplicationCache::Cold
            && (case.name.contains("medium-") || case.name.contains("process-"))
    }) {
        case.browser.as_mut().unwrap().threads = None;
        cases.push(case);
    }
    cases.push(warm(cases[0].clone()));
    let medium = cases
        .iter()
        .find(|case| case.name.contains("medium-"))
        .unwrap()
        .clone();
    cases.push(warm(medium));
    Ok(cases)
}

fn typescript() -> io::Result<Vec<PairCase>> {
    let source = dims(512, 384);
    let mut cases = vec![
        call(
            "typescript-nearest-preview",
            source,
            dims(256, 192),
            PublicOperation::ResizeNearest {
                anchor: Anchor::Center,
            },
        )?,
        call(
            "typescript-direct-palette-exact",
            source,
            source,
            PublicOperation::Quantize { settings: direct() },
        )?,
        call(
            "typescript-process-palette-exact",
            source,
            dims(256, 192),
            process(dims(256, 192), None, spec::MatchPolicy::SrgbEuclidean),
        )?,
    ];
    for case in &mut cases {
        let browser = case.browser.as_mut().unwrap();
        if !matches!(browser.operation, PublicOperation::ResizeNearest { .. }) {
            // S39's proven subset, not a claim about arbitrary nearest-color floating-point ties.
            let palette = direct().palette;
            for (index, pixel) in case.rgba.chunks_exact_mut(4).enumerate() {
                let PaletteEntry::Color { rgb } = palette[index % 15] else {
                    unreachable!()
                };
                pixel[..3].copy_from_slice(&rgb);
                pixel[3] = if index % 17 == 0 { 0 } else { 255 };
            }
        }
        browser.accepted = BrowserBackend::TypeScript;
        browser.execution = None;
        browser.cache = CacheCapability::None;
        case.measurement.application_cache = ApplicationCache::NotApplicable;
        case.accepted_subject = browser.operation.subject(BrowserBackend::TypeScript).into();
        case.identity =
            browser
                .operation
                .identity(case.source, &case.rgba, case.identity.output)?;
    }
    Ok(cases)
}

pub fn experiment(lane: &str, notes: String) -> io::Result<Experiment> {
    let mut cases = match lane {
        "anchors" | "candidate" | "anchors-native" | "candidate-native" => {
            let public = !lane.ends_with("native");
            let mut plan = anchors::experiment(public, notes.clone())?;
            if lane.starts_with("candidate") {
                plan.cases.retain(|case| case.measurement.application_cache == ApplicationCache::Cold &&
                    (case.name.starts_with("resize-") || case.name.starts_with("process-")));
                for case in &mut plan.cases {
                    let policy = CacheCapability::Roles { accepted: PreparationCapability::ImageStages,
                        candidate: PreparationCapability::ImageStages, sample_prime: None };
                    if let Some(browser) = &mut case.browser { browser.cache = policy; }
                    else if let Some(native::NativeOperation::Processor { cache, .. }) = &mut case.native { *cache = policy; }
                }
            }
            if lane == "candidate" {
                for case in &mut plan.cases { if let Some(browser) = &mut case.browser { browser.execution = Some(BrowserExecution::HostWorker); } }
            }
            plan.cases
        }
        "release" => release()?,
        "automatic" => release()?.into_iter().filter_map(|mut case| {
            let warm_control = case.measurement.application_cache == ApplicationCache::Warm;
            if !(warm_control || case.name.contains("selected") || case.name.starts_with("common-direct") ||
                case.name.starts_with("perturb-") || case.name.starts_with("separable-") ||
                case.name.contains("medium-") || case.name.contains("yliluoma-process-")) { return None; }
            case.browser.as_mut().unwrap().threads = Some(ThreadRoles { accepted: Threads::Disabled, candidate: Threads::Required });
            Some(case)
        }).collect(),
        "typescript" => typescript()?,
        "capped" => vec![call("capped-indexed-8192x8192", dims(1,1), dims(8192,8192), process(dims(8192,8192), None, spec::MatchPolicy::SrgbEuclidean))?],
        "initialization" | "initialization-threads" => {
            let mut cases = Vec::new();
            for (name, preparation) in [("bytes", BrowserPreparation::InitializationBytes), ("compiled", BrowserPreparation::InitializationCompiled)] {
                let mut case = call(name, dims(2,2), dims(1,1), PublicOperation::ResizeNearest { anchor: Anchor::Center })?;
                case.measurement.scope = CallScope::Initialization;
                case.measurement.application_cache = ApplicationCache::NotApplicable;
                let browser = case.browser.as_mut().unwrap();
                browser.cache = CacheCapability::None;
                browser.preparation = preparation;
                if lane.ends_with("threads") { browser.threads = Some(ThreadRoles { accepted: Threads::Required, candidate: Threads::Required }); }
                cases.push(case);
            }
            cases
        }
        _ => return Err(io::Error::other("lane must be candidate, anchors, release, automatic, typescript, capped, initialization, or initialization-threads")),
    };
    if lane == "capped" {
        cases[0]
            .browser
            .as_mut()
            .unwrap()
            .retained_output_limit_bytes = Some(384 * 1024 * 1024);
    }
    let plan = Experiment {
        label: format!("s41-{lane}"),
        reference_state: ReferenceState::Frozen,
        pairs: 2,
        host_load_notes: notes,
        cases,
    };
    for case in &plan.cases {
        validate_case(case)?;
    }
    validate_experiment(&plan)?;
    Ok(plan)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [lane, selection, destination, notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: release_integration_plan LANE inventory|CASE_INDEX NEW_JSON HOST_LOAD_NOTES",
        ));
    };
    let mut plan = experiment(lane, notes.clone())?;
    let engines = if lane.ends_with("native") {
        1
    } else if matches!(lane.as_str(), "automatic" | "initialization-threads") {
        2
    } else {
        3
    };
    if selection != "inventory" {
        let index: usize = selection.parse().map_err(io::Error::other)?;
        if index >= plan.cases.len() {
            return Err(io::Error::other("case index outside fixed lane"));
        }
        // One case per file bounds prepared JSON copies. The fixed inventory owns the full matrix.
        plan.cases = vec![plan.cases.swap_remove(index)];
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    if selection == "inventory" {
        let inventory: Vec<_> = plan.cases.iter().enumerate().map(|(index, case)| serde_json::json!({
            "index": index, "name": case.name, "source": case.source, "output": case.identity.output,
            "indexed": case.browser.as_ref().map(|browser| matches!(browser.operation, PublicOperation::Quantize { .. } |
                PublicOperation::Process { .. } | PublicOperation::Separable { .. } | PublicOperation::Yliluoma { .. } |
                PublicOperation::Diffusion { .. })), "workers": plan.pairs * 2 * engines
        })).collect();
        serde_json::to_writer(&mut output, &inventory).map_err(io::Error::other)?;
        return output.write_all(b"\n");
    }
    // Streaming avoids creating another source-sized string. The existing trial protocol remains unchanged.
    serde_json::to_writer(&mut output, &plan).map_err(io::Error::other)?;
    output.write_all(b"\n")?;
    println!(
        "{} cases; {} workers; no processing executed",
        plan.cases.len(),
        plan.cases.len() * plan.pairs * 2 * engines
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_counts_and_ordinary_policy_are_explicit() {
        for (lane, count) in [
            ("candidate", 2),
            ("candidate-native", 2),
            ("anchors", 8),
            ("anchors-native", 8),
            ("release", 15),
            ("automatic", 10),
            ("typescript", 3),
            ("capped", 1),
            ("initialization", 2),
            ("initialization-threads", 2),
        ] {
            let plan = experiment(lane, "untimed fixture validation".into()).unwrap();
            assert_eq!(plan.cases.len(), count, "{lane}");
            for case in &plan.cases {
                assert_eq!(case.measurement.samples, 20);
                assert_eq!(case.measurement.measurement_ms, 10_000);
                if let Some(browser) = &case.browser {
                    let encoded = serde_json::to_value(browser).unwrap();
                    assert_eq!(
                        encoded.get("retained_output_limit_bytes").is_some(),
                        lane == "capped"
                    );
                }
                assert!(case
                    .browser
                    .as_ref()
                    .is_none_or(|browser| browser.row_policy.is_none()));
            }
        }
    }

    #[test]
    fn cold_anchors_preserve_exact_historical_input_and_settings() {
        let historical = anchors::experiment(true, "historical".into()).unwrap();
        for case in experiment("candidate", "untimed".into()).unwrap().cases {
            let original = historical
                .cases
                .iter()
                .find(|old| old.name == case.name)
                .unwrap();
            assert_eq!(case.identity, original.identity);
            assert_eq!(case.rgba, original.rgba);
            assert_eq!(case.source, dims(129, 97));
            assert_eq!(case.identity.output, dims(65, 49));
        }
    }

    #[test]
    fn historical_anchors_preserve_page_execution_and_wire_cases() {
        let historical = anchors::experiment(true, "untimed".into()).unwrap();
        let current = experiment("anchors", "untimed".into()).unwrap();
        assert_eq!(
            serde_json::to_value(&current.cases).unwrap(),
            serde_json::to_value(&historical.cases).unwrap()
        );
        for case in current.cases {
            let browser = case.browser.unwrap();
            assert!(browser.execution.is_none());
            assert!(serde_json::to_value(browser).unwrap().get("execution").is_none());
        }
    }

    #[test]
    fn capped_uses_real_pixel_limit_without_a_max_area_json_source() {
        let plan = experiment("capped", "untimed".into()).unwrap();
        let case = &plan.cases[0];
        assert_eq!(case.rgba.len(), 4);
        assert_eq!(
            u64::from(case.identity.output.width) * u64::from(case.identity.output.height),
            67_108_864
        );
    }
}
