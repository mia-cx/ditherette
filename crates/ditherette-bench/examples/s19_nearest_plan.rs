//! Prepare the bounded S19 experiment without running any measurements.

use ditherette_bench::{
    paired::{
        coordinator::validate_experiment, ApplicationCache, CallScope, Experiment, Measurement,
        PairCase, SampleMode,
    },
    verification::{input_digest, settings_digest},
};
use ditherette_bench_api::verification::{
    CaseIdentity, Dimensions, Operation, ReferenceState, SemanticIdentity,
};
use std::{env, fs::OpenOptions, io, io::Write};

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [path, host_load_notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: s19_nearest_plan NEW_JSON HOST_LOAD_NOTES",
        ));
    };
    let mut cases = Vec::new();
    for (name, sw, sh, ow, oh) in [
        ("identity", 512, 384, 512, 384),
        ("reduction", 512, 384, 256, 192),
        ("enlargement", 128, 96, 512, 384),
        ("unequal-axes", 512, 96, 128, 384),
        ("tiny", 3, 2, 7, 5),
    ] {
        let source = Dimensions {
            width: sw,
            height: sh,
        };
        let output = Dimensions {
            width: ow,
            height: oh,
        };
        let rgba: Vec<_> = (0..sh)
            .flat_map(|y| {
                (0..sw).flat_map(move |x| [x as u8, y as u8, (x ^ y) as u8, (x + y) as u8])
            })
            .collect();
        let semantics = SemanticIdentity {
            operation: Operation::Resize,
            recipe: "nearest-center-default".into(),
            version: 1,
            space: None,
        };
        let identity = CaseIdentity {
            input: input_digest(source, &rgba),
            settings: settings_digest(&(semantics.clone(), output, "center-default"))
                .map_err(io::Error::other)?,
            semantics,
            output,
        };
        for (mode, suffix) in [
            (SampleMode::SingleCall, "latency"),
            (SampleMode::Throughput, "throughput"),
        ] {
            cases.push(PairCase {
                name: format!("nearest-{name}-{suffix}"),
                identity: identity.clone(),
                source,
                rgba: rgba.clone(),
                reference_subject: "spec:resize:nearest:scalar".into(),
                accepted_subject: "prod:resize:nearest:scalar".into(),
                candidate_subject: "candidate:resize:nearest:incremental".into(),
                measurement: Measurement {
                    mode,
                    scope: CallScope::NativeKernel,
                    application_cache: ApplicationCache::NotApplicable,
                    samples: 100,
                    measurement_ms: 1000,
                    warmup_ms: 250,
                    target_sample_ms: 5,
                },
            });
        }
    }
    let experiment = Experiment {
        label: "S19 exact nearest incremental candidate versus literal production baseline".into(),
        reference_state: ReferenceState::Frozen,
        pairs: 4,
        host_load_notes: host_load_notes.clone(),
        cases,
    };
    validate_experiment(&experiment)?;
    let bytes = serde_json::to_vec_pretty(&experiment).map_err(io::Error::other)?;
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(&bytes)
}
