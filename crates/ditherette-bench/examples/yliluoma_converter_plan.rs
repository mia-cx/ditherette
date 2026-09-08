//! Eight declared complete-call cases. This generator never measures operations.
use ditherette_bench::paired::{
    browser::*, coordinator::validate_experiment, native::NativeOperation, quantize::*,
    yliluoma::YliluomaSettings, *,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use ditherette_wasm::{
    bench_subjects::yiluoma::YLILUOMA_SUBJECT,
    spec::contract::request::{BayerSize, Placement},
};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn experiment(public: bool, notes: String) -> io::Result<Experiment> {
    use MatchPolicy::*;
    let adaptive = Placement::Adaptive {
        radius: 1,
        threshold: 5.0,
        softness: 10.0,
    };
    let everywhere = Placement::Everywhere {};
    // Explicit cases bound the exhaustive P² * (matrix² + 1) search cost.
    let recipes = [
        (
            "srgb-p2-b2",
            32,
            24,
            2,
            BayerSize::Two,
            SrgbEuclidean,
            everywhere,
        ),
        (
            "linear-p16-b2",
            8,
            8,
            16,
            BayerSize::Two,
            LinearRgbEuclidean,
            everywhere,
        ),
        (
            "oklab-p8-b4",
            16,
            12,
            8,
            BayerSize::Four,
            OklabEuclidean,
            everywhere,
        ),
        (
            "cielab-p16-b4",
            8,
            8,
            16,
            BayerSize::Four,
            CielabEuclidean,
            everywhere,
        ),
        (
            "ciede2000-p4-b4",
            4,
            4,
            4,
            BayerSize::Four,
            CielabCiede2000,
            everywhere,
        ),
        (
            "oklch-p8-b4-adaptive",
            8,
            8,
            8,
            BayerSize::Four,
            OklchHueArc,
            adaptive,
        ),
        (
            "srgb-p256-b2-control",
            1,
            1,
            256,
            BayerSize::Two,
            SrgbEuclidean,
            everywhere,
        ),
        (
            "srgb-p2-b16-control",
            2,
            2,
            2,
            BayerSize::Sixteen,
            SrgbEuclidean,
            everywhere,
        ),
    ];
    let mut cases = Vec::new();
    for (name, width, height, palette_len, size, matching, placement) in recipes {
        let source = Dimensions { width, height };
        let rgba: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                [
                    (i * 73 + 17) as u8,
                    (i * 31 + 71) as u8,
                    (i * 43 + 113) as u8,
                    255,
                ]
            })
            .collect();
        let settings = YliluomaSettings {
            quantize: QuantizeSettings {
                palette: (0..palette_len)
                    .map(|i| PaletteEntry::Color {
                        rgb: [(i * 73) as u8, (i * 31 + 17) as u8, (i * 43 + 119) as u8],
                    })
                    .collect(),
                alpha: AlphaPolicy::Matte { rgb: [29, 71, 211] },
                matching,
            },
            size,
            placement,
        };
        let native = NativeOperation::Yliluoma {
            settings: settings.clone(),
        };
        let operation = PublicOperation::Yliluoma { settings };
        let subject = if public {
            operation.subject(BrowserBackend::Package)
        } else {
            YLILUOMA_SUBJECT
        };
        cases.push(PairCase {
            name: name.into(),
            identity: native.identity(source, &rgba)?,
            source,
            rgba,
            reference_subject: native.reference_subject().into(),
            accepted_subject: subject.into(),
            candidate_subject: subject.into(),
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
                accepted: BrowserBackend::Package,
                candidate: BrowserBackend::Package,
                preparation: BrowserPreparation::PrimedInstance,
                cache: CacheCapability::None,
                measure_nonexact: false,
            }),
            native: (!public).then_some(native),
        });
    }
    let experiment = Experiment {
        label: format!(
            "S29 {} literal4e0c134 versus prepared-converter candidate",
            if public { "public" } else { "native" }
        ),
        reference_state: ReferenceState::Frozen,
        pairs: 2,
        host_load_notes: notes,
        cases,
    };
    validate_experiment(&experiment)?;
    Ok(experiment)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [kind, output, notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: yliluoma_converter_plan native|public output.json host-load-notes",
        ));
    };
    let public = match kind.as_str() {
        "native" => false,
        "public" => true,
        _ => return Err(io::Error::other("kind must be native or public")),
    };
    let bytes =
        serde_json::to_vec_pretty(&experiment(public, notes.clone())?).map_err(io::Error::other)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?
        .write_all(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eight_explicit_cases_bind_identical_native_public_settings_and_bounded_caps() {
        let native = experiment(false, "fixture".into()).unwrap();
        let public = experiment(true, "fixture".into()).unwrap();
        assert_eq!((native.cases.len(), public.cases.len()), (8, 8));
        assert_eq!(
            (native.cases.len() + public.cases.len() * 3) * native.pairs as usize * 2,
            128
        );
        for (native, public) in native.cases.iter().zip(&public.cases) {
            assert_eq!(native.identity, public.identity);
            assert_eq!(native.rgba, public.rgba);
            let browser = public.browser.as_ref().unwrap();
            assert_eq!(
                browser
                    .operation
                    .identity(public.source, &public.rgba, public.source)
                    .unwrap(),
                native.identity
            );
            assert_eq!(browser.preparation, BrowserPreparation::PrimedInstance);
            assert!(!browser.measure_nonexact);
            for case in [native, public] {
                assert_eq!(case.measurement.samples, 20);
                assert_eq!(case.measurement.measurement_ms, 10_000);
                assert_eq!(case.measurement.mode, SampleMode::SingleCall);
                assert_eq!(
                    case.measurement.application_cache,
                    ApplicationCache::NotApplicable
                );
            }
        }
        assert_eq!(
            native.cases[6].source,
            Dimensions {
                width: 1,
                height: 1
            }
        );
        assert_eq!(
            native.cases[7].source,
            Dimensions {
                width: 2,
                height: 2
            }
        );
    }
}
