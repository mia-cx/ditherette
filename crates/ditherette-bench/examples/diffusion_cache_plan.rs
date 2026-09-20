//! Declare native diffusion controls with and without optional RGB cache allocation.
//! This generator never measures operations or changes retained executable snapshots.

use ditherette_bench::paired::{
    coordinator::validate_experiment,
    native::NativeOperation,
    scalar::{self, Comparison},
    Experiment,
};
use ditherette_bench_api::verification::Dimensions;
use ditherette_wasm::spec::contract::request::Placement;
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn fixture(source: Dimensions, repeated: bool) -> Vec<u8> {
    // Match the scalar controls at every source coordinate; only the width changes.
    const COLORS: [[u8; 4]; 4] = [
        [37, 93, 149, 255],
        [100, 100, 100, 255],
        [203, 151, 89, 255],
        [61, 187, 113, 255],
    ];
    (0..source.height)
        .flat_map(|y| {
            (0..source.width).flat_map(move |x| {
                if repeated {
                    return COLORS[((x / 32 + y / 16) % COLORS.len() as u32) as usize];
                }
                [
                    (x * 17 + y * 7 + 11) as u8,
                    (y * 31 + x * 3 + 23) as u8,
                    ((x ^ y) + 47) as u8,
                    (x * 43 + y * 19 + 255) as u8,
                ]
            })
        })
        .collect()
}

fn experiment(notes: String, adaptive: bool) -> io::Result<Experiment> {
    let mut plan = scalar::experiment(Comparison::ProdProd, notes)?;
    let fields: Vec<_> = plan
        .cases
        .iter()
        .filter(|case| {
            matches!(
                case.name.as_str(),
                "perturb-bayer8-oklab" | "perturb-random-oklch" | "perturb-blue-cielch"
            )
        })
        .cloned()
        .collect();
    plan.cases
        .retain(|case| case.name.starts_with("diffusion-"));
    let source = Dimensions {
        width: 512,
        height: 96,
    };
    let controls = plan.cases.clone();
    for repeated in [false, true] {
        let rgba = fixture(source, repeated);
        let suffix = if repeated {
            "512x96-opaque-repeated"
        } else {
            "512x96-original-alpha"
        };
        for control in &controls {
            let mut case = control.clone();
            case.name = format!("{}-{suffix}", case.name);
            case.source = source;
            case.rgba = rgba.clone();
            case.identity = case
                .native
                .as_ref()
                .expect("scalar diffusion controls have native operations")
                .identity(source, &case.rgba)?;
            plan.cases.push(case);
        }
    }
    plan.label = "Native diffusion RGB cache controls: original, wide, and opaque repeated".into();
    if adaptive {
        let rgba = fixture(source, false);
        for (index, control) in controls.iter().enumerate() {
            let mut case = control.clone();
            let radius = 1 + (index / 2 % 2) as u32;
            case.name = format!("{}-512x96-adaptive-radius{radius}", case.name);
            case.source = source;
            case.rgba = rgba.clone();
            let operation = case.native.as_mut().expect("native diffusion control");
            let NativeOperation::Diffusion { settings } = operation else {
                unreachable!("scalar diffusion case");
            };
            settings.placement = Placement::Adaptive {
                radius,
                threshold: 10.0,
                softness: 5.0,
            };
            case.identity = operation.identity(source, &case.rgba)?;
            plan.cases.push(case);
        }
        plan.cases.extend(fields);
        plan.label =
            "Native adaptive coordinate rows with retained diffusion cache controls".into();
    }
    plan.pairs = 2;
    for case in &mut plan.cases {
        case.measurement.samples = 80;
        case.measurement.warmup_ms = 250;
        case.measurement.measurement_ms = 1000;
    }
    validate_experiment(&plan)?;
    Ok(plan)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [output, notes, options @ ..] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: diffusion_cache_plan NEW_JSON HOST_LOAD_NOTES [--adaptive]",
        ));
    };
    let adaptive = match options {
        [] => false,
        [option] if option == "--adaptive" => true,
        _ => return Err(io::Error::other("only --adaptive is supported")),
    };
    let bytes = serde_json::to_vec_pretty(&experiment(notes.clone(), adaptive)?)
        .map_err(io::Error::other)?;
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?
        .write_all(&bytes)
}
