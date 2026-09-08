//! Export frozen S26 benchmark outputs. No operation is timed.
#[path = "support/fields.rs"]
mod fields;

use ditherette_bench::paired::{browser::PublicOperation, quantize::PaletteEntry};
use ditherette_bench_api::verification::{Dimensions, VerificationOutput};
use ditherette_wasm::bench_subjects::{self, BenchSubject};
use serde::Serialize;
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

#[derive(Serialize)]
struct Fixture {
    name: String,
    operation: PublicOperation,
    source: Dimensions,
    rgba: Vec<u8>,
    reference: VerificationOutput,
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [path] = args.as_slice() else {
        return Err(io::Error::other("usage: field_conformance output.json"));
    };
    let registry = bench_subjects::bench_subjects();
    let source = Dimensions {
        width: 4,
        height: 2,
    };
    let rgba = vec![
        0, 0, 0, 0, 128, 128, 128, 127, 128, 128, 128, 128, 255, 255, 255, 255, 17, 33, 71, 1, 100,
        80, 60, 254, 1, 2, 3, 255, 255, 0, 127, 255,
    ];
    let mut recipes = fields::complete_recipes();
    let PublicOperation::Separable { settings } = &recipes[7].1 else {
        unreachable!()
    };
    let settings = settings.clone();
    for (name, palette) in [
        ("transparent-only", vec![PaletteEntry::Transparent {}]),
        (
            "transparent-fallback",
            vec![PaletteEntry::Color { rgb: [17, 33, 71] }],
        ),
        (
            "palette-truncated",
            vec![PaletteEntry::Color { rgb: [1, 2, 3] }; 257],
        ),
    ] {
        let mut settings = settings.clone();
        settings.quantize.palette = palette;
        recipes.push((
            format!("separable-{name}"),
            PublicOperation::Separable { settings },
        ));
    }
    let mut fixtures = Vec::new();
    for (name, operation) in recipes {
        let request = operation
            .processing_request(source, &rgba)?
            .expect("S26 processing");
        let BenchSubject::Conformance(subject) = registry
            .iter()
            .find(|subject| subject.descriptor().id.as_str() == operation.reference_subject())
            .expect("registered frozen reference")
        else {
            unreachable!()
        };
        let reference = (subject.run)(&request).map_err(io::Error::other)?;
        fixtures.push(Fixture {
            name,
            operation,
            source,
            rgba: rgba.clone(),
            reference,
        });
    }
    let bytes = serde_json::to_vec_pretty(&fixtures).map_err(io::Error::other)?;
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(&bytes)
}
