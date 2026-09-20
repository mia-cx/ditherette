//! Fixed S35-S37 complete-call plans. This executable starts no browser or measurement.
#[allow(dead_code)]
#[path = "yliluoma_row_band_cases.rs"]
mod mixing;
#[allow(dead_code)]
#[path = "preparation_integration_plan.rs"]
mod preparation_plan;
#[path = "support/row_fields.rs"]
mod row_fields;

use ditherette_bench::paired::{
    browser::{
        Anchor, BrowserBackend, BrowserCase, BrowserExecution, BrowserPreparation, CacheCapability,
        PreparationCapability, PublicOperation, RowBandParameters, RowPolicyRoles, RowStage,
        Support, ThreadRoles, Threads,
    },
    coordinator::validate_experiment,
    preparation::SamplePrime,
    ApplicationCache, CallScope, Experiment, Measurement, PairCase, SampleMode,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use std::{env, fs::OpenOptions, io, io::Write};

const PAIRS: usize = 2;
const ENGINES: usize = 2;
const ROLES: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Choice {
    Two,
    Four,
    Warm,
}

impl Choice {
    fn name(self) -> &'static str {
        match self {
            Self::Two => "two",
            Self::Four => "four",
            Self::Warm => "final-hit-overhead",
        }
    }

    fn parameters(self, stage: RowStage) -> RowBandParameters {
        let two = self == Self::Two;
        RowBandParameters {
            height: match (stage, two) {
                (RowStage::Mixing, true) => 4,
                (RowStage::Mixing, false) => 16,
                (_, true) => 32,
                (_, false) => 128,
            },
            active_workers: if two { 2 } else { 4 },
        }
    }
}

fn dimensions(width: u32, height: u32) -> Dimensions {
    Dimensions { width, height }
}

fn case(
    name: String,
    source: Dimensions,
    output: Dimensions,
    operation: PublicOperation,
) -> io::Result<PairCase> {
    let rgba = preparation_plan::source_rgba(source);
    // Landed resize drift remains an incorrect frozen gate. Diagnostics retain it;
    // any row-policy selection still requires exact equality with the scalar role.
    let measure_nonexact = matches!(
        operation,
        PublicOperation::ResizeArea {}
            | PublicOperation::ResizeBilinear { .. }
            | PublicOperation::ResizeLanczos3 { .. }
    );
    Ok(PairCase {
        name,
        identity: operation.identity(source, &rgba, output)?,
        source,
        rgba,
        reference_subject: operation.reference_subject().into(),
        accepted_subject: operation.subject(BrowserBackend::Package).into(),
        candidate_subject: operation.subject(BrowserBackend::Package).into(),
        native: None,
        browser: Some(BrowserCase {
            row_policy: None,
            execution: Some(BrowserExecution::HostWorker),
            operation,
            accepted: BrowserBackend::Package,
            candidate: BrowserBackend::Package,
            preparation: BrowserPreparation::FreshInstance,
            cache: CacheCapability::Roles {
                accepted: PreparationCapability::ImageStages,
                candidate: PreparationCapability::ImageStages,
                sample_prime: None,
            },
            progress: None,
            threads: Some(ThreadRoles {
                accepted: Threads::Required,
                candidate: Threads::Required,
            }),
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

fn resize_cases(choice: Choice) -> io::Result<Vec<PairCase>> {
    let anchor = Anchor::Center;
    let mut cases = Vec::new();
    for (name, operation, source, output) in [
        (
            "nearest",
            PublicOperation::ResizeNearest { anchor },
            dimensions(2048, 1536),
            dimensions(1024, 768),
        ),
        (
            "area",
            PublicOperation::ResizeArea {},
            dimensions(1537, 1025),
            dimensions(769, 513),
        ),
        (
            "bilinear",
            PublicOperation::ResizeBilinear { anchor },
            dimensions(1537, 1025),
            dimensions(769, 513),
        ),
        (
            "lanczos3-scale-aware",
            PublicOperation::ResizeLanczos3 {
                anchor,
                support: Support::ScaleAware,
            },
            dimensions(2048, 1536),
            dimensions(512, 384),
        ),
    ] {
        if choice == Choice::Two {
            cases.push(case(
                format!("{name}-small"),
                dimensions(129, 97),
                dimensions(65, 49),
                operation.clone(),
            )?);
        }
        cases.push(case(format!("{name}-large"), source, output, operation)?);
    }
    Ok(cases)
}

fn field_cases(choice: Choice) -> io::Result<Vec<PairCase>> {
    let mut cases = Vec::new();
    for (name, native) in row_fields::recipes() {
        let perceptual = name.ends_with("oklab64-blue-adaptive2");
        // The separable perceptual call covers the second policy's dispatch overhead.
        // Repeating its two component calls would exceed the fixed first-sweep budget.
        if choice == Choice::Four && perceptual && !name.starts_with("separable-") {
            continue;
        }
        let operation = row_fields::public(&native);
        if choice == Choice::Two {
            cases.push(case(
                format!("{name}-small"),
                dimensions(33, 25),
                dimensions(33, 25),
                operation.clone(),
            )?);
        }
        let large = if perceptual {
            dimensions(65, 49)
        } else {
            dimensions(769, 513)
        };
        cases.push(case(format!("{name}-large"), large, large, operation)?);
    }
    Ok(cases)
}

fn experiment(slice: &str, choice: Choice, notes: String) -> io::Result<Experiment> {
    let (stage, mut cases) = match slice {
        "s35" => (RowStage::Resize, resize_cases(choice)?),
        "s36" => (RowStage::Indexed, field_cases(choice)?),
        "s37" => (
            RowStage::Mixing,
            mixing::cases()?
                .into_iter()
                .filter(|case| case.measurement.application_cache == ApplicationCache::Cold)
                .collect(),
        ),
        _ => return Err(io::Error::other("slice must be s35, s36, or s37")),
    };
    for case in &mut cases {
        case.name = format!("{slice}-{}-{}", case.name, choice.name());
        let browser = case.browser.as_mut().unwrap();
        browser.row_policy = Some(RowPolicyRoles {
            stage,
            accepted: RowBandParameters {
                height: 0,
                active_workers: 1,
            },
            candidate: choice.parameters(stage),
        });
        if choice == Choice::Warm {
            case.measurement.application_cache = ApplicationCache::Warm;
            browser.preparation = BrowserPreparation::PrimedSample;
            browser.cache = CacheCapability::Roles {
                accepted: PreparationCapability::ImageStages,
                candidate: PreparationCapability::ImageStages,
                sample_prime: Some(SamplePrime::SameCall),
            };
        }
        ditherette_bench::paired::browser::validate_case(case)?;
    }
    let plan = Experiment {
        label: format!("{slice}-row-bands-{}", choice.name()),
        reference_state: ReferenceState::Frozen,
        pairs: PAIRS,
        host_load_notes: notes,
        cases,
    };
    validate_experiment(&plan)?;
    Ok(plan)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [slice, policy, destination, notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: row_band_integration_plan s35|s36|s37 two|four|warm NEW_JSON HOST_LOAD_NOTES",
        ));
    };
    let choice = match policy.as_str() {
        "two" => Choice::Two,
        "four" => Choice::Four,
        "warm" => Choice::Warm,
        _ => return Err(io::Error::other("policy must be two, four, or warm")),
    };
    let plan = experiment(slice, choice, notes.clone())?;
    let workers = plan.cases.len() * plan.pairs * ROLES * ENGINES;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?
        .write_all(&serde_json::to_vec_pretty(&plan).map_err(io::Error::other)?)?;
    eprintln!(
        "{} cases; {workers} browser workers across {ENGINES} engines. No measurement started.",
        plan.cases.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn row_band_matrix_is_fixed_at_400_workers_with_explicit_roles_and_final_hit_controls() {
        let mut total = 0;
        for (slice, counts) in [("s35", [8, 4, 4]), ("s36", [12, 4, 6]), ("s37", [4, 4, 4])] {
            let mut cold_identities = BTreeSet::new();
            let mut warm_identities = BTreeSet::new();
            for (choice, count) in [Choice::Two, Choice::Four, Choice::Warm]
                .into_iter()
                .zip(counts)
            {
                let plan = experiment(slice, choice, "untimed matrix fixture".into()).unwrap();
                assert_eq!(plan.cases.len(), count);
                assert_eq!(plan.pairs, 2);
                assert_eq!(plan.reference_state, ReferenceState::Frozen);
                total += count * plan.pairs * ROLES * ENGINES;
                for case in &plan.cases {
                    let browser = case.browser.as_ref().unwrap();
                    assert!(case.native.is_none());
                    assert_eq!(
                        browser.measure_nonexact,
                        matches!(
                            browser.operation,
                            PublicOperation::ResizeArea {}
                                | PublicOperation::ResizeBilinear { .. }
                                | PublicOperation::ResizeLanczos3 { .. }
                        )
                    );
                    assert_eq!(browser.execution, Some(BrowserExecution::HostWorker));
                    assert_eq!(
                        browser.threads,
                        Some(ThreadRoles {
                            accepted: Threads::Required,
                            candidate: Threads::Required
                        })
                    );
                    let policy = browser.row_policy.unwrap();
                    assert_eq!(
                        policy.accepted,
                        RowBandParameters {
                            height: 0,
                            active_workers: 1
                        }
                    );
                    assert_eq!(policy.candidate, choice.parameters(policy.stage));
                    assert_eq!(
                        case.accepted_subject,
                        browser.operation.subject(BrowserBackend::Package)
                    );
                    assert_eq!(case.accepted_subject, case.candidate_subject);
                    assert_eq!(
                        case.reference_subject,
                        browser.operation.reference_subject()
                    );
                    assert_eq!(
                        case.identity,
                        browser
                            .operation
                            .identity(case.source, &case.rgba, case.identity.output)
                            .unwrap()
                    );
                    assert_eq!(case.measurement.mode, SampleMode::SingleCall);
                    assert_eq!(case.measurement.scope, CallScope::CompleteCall);
                    assert_eq!(
                        (
                            case.measurement.samples,
                            case.measurement.warmup_ms,
                            case.measurement.measurement_ms
                        ),
                        (20, 50, 10_000)
                    );
                    let identity = serde_json::to_string(&case.identity).unwrap();
                    if choice == Choice::Warm {
                        assert!(warm_identities.insert(identity.clone()));
                        assert!(cold_identities.contains(&identity));
                        assert!(case.name.ends_with("final-hit-overhead"));
                        assert_eq!(browser.cache.sample_prime(), Some(SamplePrime::SameCall));
                        assert_eq!(browser.preparation, BrowserPreparation::PrimedSample);
                    } else {
                        cold_identities.insert(identity);
                        assert_eq!(case.measurement.application_cache, ApplicationCache::Cold);
                        assert_eq!(browser.cache.sample_prime(), None);
                    }
                }
            }
        }
        assert_eq!(total, 400);
    }

    #[test]
    fn row_band_matrix_preserves_required_shapes_and_caps_perceptual_work() {
        let resize = resize_cases(Choice::Two).unwrap();
        assert_eq!(
            resize
                .iter()
                .map(|case| (
                    case.source.width,
                    case.source.height,
                    case.identity.output.width,
                    case.identity.output.height
                ))
                .collect::<Vec<_>>(),
            [
                (129, 97, 65, 49),
                (2048, 1536, 1024, 768),
                (129, 97, 65, 49),
                (1537, 1025, 769, 513),
                (129, 97, 65, 49),
                (1537, 1025, 769, 513),
                (129, 97, 65, 49),
                (2048, 1536, 512, 384),
            ]
        );
        for case in field_cases(Choice::Two).unwrap() {
            if case.name.contains("oklab64") {
                assert!(case.source.width <= 65 && case.source.height <= 49);
            }
        }
        let mixing = experiment("s37", Choice::Two, "untimed fixture".into()).unwrap();
        assert_eq!(mixing.cases.len(), 4);
        assert!(mixing.cases.iter().any(|case| matches!(
            case.browser.as_ref().unwrap().operation,
            PublicOperation::Process { .. }
        )));
    }
}
