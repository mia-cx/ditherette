//! Export small frozen outputs for the actual public benchmark adapter. Never measures calls.
use ditherette_bench::paired::quantize::*;
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
    identity: ditherette_bench_api::verification::CaseIdentity,
    settings: QuantizeSettings,
    source: Dimensions,
    rgba: Vec<u8>,
    reference: VerificationOutput,
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [path] = args.as_slice() else {
        return Err(io::Error::other("usage: quantize_conformance output.json"));
    };
    let registry = bench_subjects::bench_subjects();
    let BenchSubject::Conformance(subject) = registry
        .iter()
        .find(|s| s.descriptor().id.as_str() == "spec:quantize:request:v1")
        .expect("registered reference")
    else {
        unreachable!()
    };
    let source = Dimensions {
        width: 4,
        height: 2,
    };
    let rgba = vec![
        0, 0, 0, 0, 128, 128, 128, 127, 128, 128, 128, 128, 255, 255, 255, 255, 17, 33, 71, 1, 100,
        80, 60, 254, 1, 2, 3, 255, 255, 0, 127, 255,
    ];
    let palette = vec![
        PaletteEntry::Color { rgb: [0, 0, 0] },
        PaletteEntry::Color {
            rgb: [255, 255, 255],
        },
        PaletteEntry::Color { rgb: [17, 33, 71] },
        PaletteEntry::Transparent {},
    ];
    let mut settings = Vec::new();
    for matching in [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::YcbcrEuclidean,
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
    ] {
        for alpha in [
            AlphaPolicy::Preserve { threshold: 0.5 },
            AlphaPolicy::Premultiplied {},
            AlphaPolicy::Matte { rgb: [17, 33, 71] },
        ] {
            settings.push(QuantizeSettings {
                palette: palette.clone(),
                alpha,
                matching,
            });
        }
    }
    for palette in [
        vec![PaletteEntry::Transparent {}],
        vec![PaletteEntry::Color { rgb: [1, 2, 3] }; 257],
    ] {
        settings.push(QuantizeSettings {
            palette,
            alpha: AlphaPolicy::Preserve { threshold: 0.5 },
            matching: MatchPolicy::SrgbEuclidean,
        });
    }
    let mut fixtures = Vec::new();
    for settings in settings {
        let request = settings.reference_request(source, &rgba)?;
        let reference = (subject.run)(&request).map_err(io::Error::other)?;
        fixtures.push(Fixture {
            identity: settings.identity(source, &rgba)?,
            settings,
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
