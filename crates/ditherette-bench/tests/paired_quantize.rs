use ditherette_bench::paired::{browser::*, native::*, quantize::*, *};
use ditherette_bench_api::verification::*;

#[path = "fixtures/paired_model.rs"]
mod model;

fn settings() -> QuantizeSettings {
    QuantizeSettings {
        palette: vec![
            PaletteEntry::Color { rgb: [10, 20, 30] },
            PaletteEntry::Transparent {},
        ],
        alpha: AlphaPolicy::Preserve { threshold: 0.5 },
        matching: MatchPolicy::SrgbEuclidean,
    }
}

#[test]
fn full_quantize_settings_and_forward_space_are_bound() {
    let source = Dimensions {
        width: 1,
        height: 1,
    };
    let rgba = [10, 20, 30, 255];
    let original = settings();
    let identity = original.identity(source, &rgba).unwrap();
    for matching in [
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::YcbcrEuclidean,
    ] {
        let mut changed = original.clone();
        changed.matching = matching;
        assert_ne!(
            identity.settings,
            changed.identity(source, &rgba).unwrap().settings
        );
    }
    for alpha in [
        AlphaPolicy::Preserve { threshold: 0.50001 },
        AlphaPolicy::Premultiplied {},
        AlphaPolicy::Matte { rgb: [1, 2, 3] },
    ] {
        let mut changed = original.clone();
        changed.alpha = alpha;
        assert_ne!(
            identity.settings,
            changed.identity(source, &rgba).unwrap().settings
        );
    }
    let mut changed = original.clone();
    changed.palette.reverse();
    assert_ne!(
        identity.settings,
        changed.identity(source, &rgba).unwrap().settings
    );
    let serialized = serde_json::to_string(&original).unwrap();
    assert_eq!(
        serde_json::from_str::<QuantizeSettings>(&serialized).unwrap(),
        original
    );
    assert!(original.identity(source, &[]).is_err());
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Cielab,
        WorkingSpace::Ycbcr,
    ] {
        let operation = NativeOperation::ColorForward { space };
        assert_eq!(
            operation
                .identity(source, &rgba)
                .unwrap()
                .semantics
                .operation,
            Operation::Color
        );
    }
    assert!(NativeOperation::ColorForward {
        space: WorkingSpace::Oklch
    }
    .identity(source, &rgba)
    .is_err());
}

#[test]
fn native_and_public_quantize_scopes_fail_closed() {
    let (mut prepared, _) = model::fixture();
    let case = &mut prepared.experiment.cases[0];
    let native = NativeOperation::Quantize {
        settings: settings(),
    };
    case.identity = native.identity(case.source, &case.rgba).unwrap();
    case.reference_subject = native.reference_subject().into();
    case.measurement.scope = native.scope();
    case.native = Some(native);
    validate_case(case).unwrap();
    case.measurement.scope = CallScope::NativeKernel;
    assert!(validate_case(case).is_err());
    case.measurement.scope = CallScope::NativeCompleteCall;
    case.measurement.application_cache = ApplicationCache::Warm;
    assert!(validate_case(case).is_err());
    case.measurement.application_cache = ApplicationCache::NotApplicable;
    let operation = PublicOperation::Quantize {
        settings: settings(),
    };
    case.browser = Some(BrowserCase {
        operation: operation.clone(),
        accepted: BrowserBackend::Package,
        candidate: BrowserBackend::Package,
        preparation: BrowserPreparation::PrimedInstance,
        cache: CacheCapability::None,
        measure_nonexact: false,
    });
    assert!(validate_case(case).is_err());
    case.native = None;
    case.measurement.scope = CallScope::CompleteCall;
    case.accepted_subject = operation.subject(BrowserBackend::Package).into();
    case.candidate_subject = case.accepted_subject.clone();
    validate_case(case).unwrap();
    case.browser.as_mut().unwrap().accepted = BrowserBackend::TypeScript;
    assert!(validate_case(case)
        .unwrap_err()
        .to_string()
        .contains("faithful TypeScript"));
}
