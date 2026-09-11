use super::{prepare_matcher, prepare_palette};
use crate::{
    image::contracts::PaletteEntry,
    prod::{
        color::packed::{Converter, PackedSpace},
        contract::request::{AlphaPolicy, MatchPolicy},
    },
};

#[test]
fn baseline_constructor_adapters_use_landed_converter_without_reordering() {
    let matching_modes = [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
        MatchPolicy::YcbcrEuclidean,
    ];
    let colors: Vec<_> = (0..257)
        .map(|i| match i % 17 {
            0 => PaletteEntry::Transparent {},
            1 => PaletteEntry::Color { rgb: [73; 3] },
            _ => PaletteEntry::Color {
                rgb: [(i * 73) as u8, (i * 117) as u8, (i * 31) as u8],
            },
        })
        .collect();
    for entries in [colors, vec![PaletteEntry::Transparent {}; 257]] {
        for alpha in [
            AlphaPolicy::Preserve { threshold: 127.5 },
            AlphaPolicy::Premultiplied {},
            AlphaPolicy::Matte { rgb: [17, 33, 71] },
        ] {
            let palette = prepare_palette(&entries, alpha);
            for matching in matching_modes {
                let matcher = prepare_matcher(&palette, matching);
                let converter = Converter::new(PackedSpace::from_matching(matching).unwrap());
                assert_eq!(matcher.matching, matching);
                assert_eq!(
                    matcher
                        .colors
                        .iter()
                        .map(|color| (color.index, color.coordinates.map(f32::to_bits)))
                        .collect::<Vec<_>>(),
                    palette
                        .visible
                        .iter()
                        .map(|color| (
                            color.index,
                            converter.coordinates(color.rgb).map(f32::to_bits)
                        ))
                        .collect::<Vec<_>>(),
                    "{matching:?} {alpha:?}"
                );
            }
        }
    }
}

#[test]
fn finite_matching_checks_candidates_after_an_exact_match() {
    use crate::prod::quantize::matcher::{PaletteColor, PaletteMatcher};

    let first = PaletteColor {
        index: 7,
        coordinates: [0.0; 3],
    };
    let mut matcher = PaletteMatcher {
        colors: vec![first, PaletteColor { index: 9, ..first }],
        matching: MatchPolicy::SrgbEuclidean,
    };
    assert_eq!(matcher.nearest_finite([0.0; 3]), Some(first));

    matcher.colors[1].coordinates[0] = f32::MAX;
    assert!(super::nearest_finite(&matcher, [0.0; 3]).is_err());
    assert_eq!(matcher.nearest_finite([0.0; 3]), None);
}
