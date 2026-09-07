use ditherette_wasm::{
    image::{
        contracts::{PaletteEntry, WarningCode},
        ImageBuf, ImageDimensions, PaletteIndex8,
    },
    spec::{
        contract::{
            error::ErrorCode,
            request::{AlphaPolicy, MatchPolicy, QuantizeRequest, Request, Source},
        },
        palette::{PalettePixel, PreparedPalette, VisibleColor},
    },
};

const PRESERVE: AlphaPolicy = AlphaPolicy::Preserve { threshold: 0.0 };
const PIXEL: [u8; 4] = [200, 100, 50, 128];

fn request(palette: &[PaletteEntry], alpha: AlphaPolicy) -> QuantizeRequest<'_> {
    QuantizeRequest {
        version: 1,
        source: Source {
            width: 1,
            height: 1,
            data: &PIXEL,
        },
        palette,
        alpha,
        matching: MatchPolicy::SrgbEuclidean,
    }
}

fn prepare(palette: &[PaletteEntry], alpha: AlphaPolicy) -> PreparedPalette {
    Request::Quantize(request(palette, alpha))
        .validate()
        .unwrap();
    PreparedPalette::new(palette, alpha)
}

#[test]
fn preserves_order_duplicates_and_first_transparent_index() {
    let palette = [
        PaletteEntry::Color { rgb: [30, 20, 10] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [30, 20, 10] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color {
            rgb: [255, 255, 255],
        },
    ];
    let prepared = prepare(&palette, PRESERVE);
    assert_eq!(prepared.palette.transparent_index, Some(1));
    assert_eq!(
        prepared.palette.rgba,
        [30, 20, 10, 255, 0, 0, 0, 0, 30, 20, 10, 255, 0, 0, 0, 0, 255, 255, 255, 255]
    );
    assert_eq!(
        prepared.visible,
        [
            VisibleColor {
                index: 0,
                rgb: [30, 20, 10]
            },
            VisibleColor {
                index: 2,
                rgb: [30, 20, 10]
            },
            VisibleColor {
                index: 4,
                rgb: [255, 255, 255]
            },
        ]
    );
    assert!(prepared.warnings.is_empty());
    assert_eq!(
        prepared.prepare_pixel([99, 88, 77, 0]),
        PalettePixel::Index(1)
    );
}

#[test]
fn retains_index_255_without_a_truncation_warning() {
    let mut entries = vec![PaletteEntry::Color { rgb: [1, 2, 3] }; 255];
    entries.push(PaletteEntry::Transparent {});
    let prepared = prepare(&entries, PRESERVE);
    assert_eq!(prepared.palette.rgba.len(), 1024);
    assert_eq!(prepared.palette.transparent_index, Some(255));
    assert_eq!(
        prepared.prepare_pixel([0, 0, 0, 0]),
        PalettePixel::Index(255)
    );
    assert!(prepared.warnings.is_empty());
}

#[test]
fn truncation_happens_before_transparency_and_fallback_selection() {
    let mut entries = vec![PaletteEntry::Color { rgb: [10, 20, 30] }; 256];
    entries.push(PaletteEntry::Transparent {});
    let prepared = prepare(&entries, PRESERVE);
    assert_eq!(prepared.palette.rgba.len(), 1024);
    assert_eq!(prepared.visible.len(), 256);
    assert_eq!(prepared.palette.transparent_index, None);
    assert_eq!(
        prepared.prepare_pixel([255, 255, 255, 0]),
        PalettePixel::Index(0)
    );
    assert_eq!(
        prepared
            .warnings
            .iter()
            .map(|warning| warning.code)
            .collect::<Vec<_>>(),
        [
            WarningCode::PaletteTruncated,
            WarningCode::TransparentFallback
        ]
    );
    assert_eq!(
        prepared.warnings[0].message,
        "Palette was truncated to 256 entries for indexed PNG export."
    );
    assert_eq!(
        prepared.warnings[1].message,
        "Transparent is disabled; alpha-thresholded pixels use the darkest enabled visible color."
    );
}

#[test]
fn transparent_only_short_circuits_every_alpha_mode_and_preserves_warning_order() {
    let entries = vec![PaletteEntry::Transparent {}; 257];
    for alpha in [
        PRESERVE,
        AlphaPolicy::Premultiplied,
        AlphaPolicy::Matte {
            rgb: [255, 255, 255],
        },
    ] {
        let prepared = prepare(&entries, alpha);
        assert_eq!(prepared.palette.transparent_index, Some(0));
        assert!(prepared.visible.is_empty());
        for pixel in [[255, 255, 255, 255], [100, 50, 25, 128], [123, 45, 67, 0]] {
            assert_eq!(prepared.prepare_pixel(pixel), PalettePixel::Index(0));
        }
        assert_eq!(
            prepared
                .warnings
                .iter()
                .map(|warning| warning.code)
                .collect::<Vec<_>>(),
            [WarningCode::PaletteTruncated, WarningCode::TransparentOnly]
        );
        assert_eq!(
            prepared.warnings[1].message,
            "Only Transparent is enabled; every output pixel is transparent."
        );
    }
}

#[test]
fn darkest_fallback_uses_rgb_sum_and_keeps_first_exact_tie() {
    let entries = [
        PaletteEntry::Color { rgb: [255, 0, 0] },
        PaletteEntry::Color { rgb: [0, 254, 0] },
        PaletteEntry::Color { rgb: [0, 0, 254] },
    ];
    let prepared = prepare(&entries, PRESERVE);
    assert_eq!(
        prepared.prepare_pixel([255, 255, 255, 0]),
        PalettePixel::Index(1)
    );
    // The preparation warning exists even when all actual source pixels are opaque.
    assert_eq!(prepared.warnings[0].code, WarningCode::TransparentFallback);
    assert_eq!(
        prepared.prepare_pixel([12, 34, 56, 255]),
        PalettePixel::Color([12, 34, 56])
    );
    for alpha in [
        AlphaPolicy::Premultiplied,
        AlphaPolicy::Matte {
            rgb: [255, 255, 255],
        },
    ] {
        assert!(prepare(&entries, alpha).warnings.is_empty());
    }
}

#[test]
fn preserve_threshold_is_inclusive_and_retains_javascript_number_precision() {
    let entries = [
        PaletteEntry::Color { rgb: [0, 0, 0] },
        PaletteEntry::Transparent {},
    ];
    let below: AlphaPolicy =
        serde_json::from_str(r#"{"mode":"preserve","threshold":127.9999999}"#).unwrap();
    let below = prepare(&entries, below);
    assert_eq!(
        below.prepare_pixel([200, 100, 50, 128]),
        PalettePixel::Color([200, 100, 50])
    );
    assert_eq!(
        below.prepare_pixel([200, 100, 50, 127]),
        PalettePixel::Index(1)
    );
    let equal = prepare(&entries, AlphaPolicy::Preserve { threshold: 128.0 });
    assert_eq!(
        equal.prepare_pixel([200, 100, 50, 128]),
        PalettePixel::Index(1)
    );
    assert_eq!(
        equal.prepare_pixel([200, 100, 50, 129]),
        PalettePixel::Color([200, 100, 50])
    );
    assert_eq!(
        prepare(&entries, AlphaPolicy::Preserve { threshold: 255.0 })
            .prepare_pixel([200, 100, 50, 255]),
        PalettePixel::Index(1)
    );
}

#[test]
fn premultiplication_rounds_to_bytes_before_color_matching() {
    let entries = [
        PaletteEntry::Color { rgb: [0, 0, 0] },
        PaletteEntry::Transparent {},
    ];
    let prepared = prepare(&entries, AlphaPolicy::Premultiplied);
    assert_eq!(
        prepared.prepare_pixel([200, 100, 50, 128]),
        PalettePixel::Color([100, 50, 25])
    );
    assert_eq!(
        prepared.prepare_pixel([255, 1, 254, 128]),
        PalettePixel::Color([128, 1, 127])
    );
    assert_eq!(
        prepared.prepare_pixel([12, 34, 56, 255]),
        PalettePixel::Color([12, 34, 56])
    );
    assert_eq!(
        prepared.prepare_pixel([200, 100, 50, 0]),
        PalettePixel::Color([0, 0, 0])
    );
}

#[test]
fn matte_uses_the_concrete_rgb_and_straight_alpha_byte_rounding() {
    let entries = [
        PaletteEntry::Color { rgb: [0, 0, 0] },
        PaletteEntry::Transparent {},
    ];
    let prepared = prepare(&entries, AlphaPolicy::Matte { rgb: [0, 0, 255] });
    assert_eq!(
        prepared.prepare_pixel([255, 0, 0, 128]),
        PalettePixel::Color([128, 0, 127])
    );
    assert_eq!(
        prepared.prepare_pixel([200, 100, 50, 0]),
        PalettePixel::Color([0, 0, 255])
    );
    assert_eq!(
        prepared.prepare_pixel([200, 100, 50, 255]),
        PalettePixel::Color([200, 100, 50])
    );
    assert!(prepared.warnings.is_empty());
}

#[test]
fn request_boundary_rejects_empty_palettes_and_malformed_images_without_mutation() {
    let entries = [PaletteEntry::Transparent {}];
    assert_eq!(
        serde_json::to_string(&entries[0]).unwrap(),
        r#"{"kind":"transparent"}"#
    );
    assert_eq!(
        serde_json::from_str::<PaletteEntry>(r#"{"kind":"transparent"}"#).unwrap(),
        entries[0]
    );
    let error = Request::Quantize(request(&[], PRESERVE))
        .validate()
        .unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::InvalidPalette, "palette")
    );
    let source = [1, 2, 3];
    let mut invalid = request(&entries, PRESERVE);
    invalid.source.data = &source;
    let error = Request::Quantize(invalid).validate().unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::InvalidImage, "source.data")
    );
    assert_eq!(source, [1, 2, 3]);
    for json in [
        r#"[{"kind":"color","rgb":[-1,0,0]}]"#,
        r#"[{"kind":"color","rgb":[256,0,0]}]"#,
        r#"[{"kind":"color","rgb":[1.5,0,0]}]"#,
        r#"[{"kind":"color","rgb":[0,0]}]"#,
        r#"[{"kind":"transparent","rgb":[0,0,0]}]"#,
        r#"[{"kind":"other"}]"#,
    ] {
        assert!(
            serde_json::from_str::<Vec<PaletteEntry>>(json).is_err(),
            "{json}"
        );
    }
}

#[test]
fn palette_preparation_and_output_own_their_metadata() {
    let mut entries = vec![
        PaletteEntry::Color { rgb: [20, 40, 60] },
        PaletteEntry::Transparent {},
    ];
    let prepared = prepare(&entries, PRESERVE);
    entries[0] = PaletteEntry::Color {
        rgb: [255, 255, 255],
    };
    let result = prepared.into_indexed(
        ImageBuf::<PaletteIndex8>::from_vec_packed(vec![1, 0], ImageDimensions::new(2, 1).unwrap())
            .unwrap(),
    );
    assert_eq!(result.indices.data(), [1, 0]);
    assert_eq!(result.palette.rgba, [20, 40, 60, 255, 0, 0, 0, 0]);
    assert_eq!(result.palette.transparent_index, Some(1));
    assert!(result.warnings.is_empty());
    assert_eq!(PIXEL, [200, 100, 50, 128]);
}
