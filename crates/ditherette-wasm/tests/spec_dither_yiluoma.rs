use ditherette_wasm::spec::{
    contract::request::MatchPolicy,
    dither::yiluoma::{best_matched_mix, best_ordered_mix, PaletteMix},
    quantize::matcher::{PaletteColor, PaletteMatcher},
};

fn matcher(colors: &[(u8, [f32; 3])], matching: MatchPolicy) -> PaletteMatcher {
    PaletteMatcher {
        colors: colors
            .iter()
            .map(|&(index, coordinates)| PaletteColor { index, coordinates })
            .collect(),
        matching,
    }
}

#[test]
fn pair_and_ratio_ties_keep_the_first_enumerated_candidate() {
    let palette = [[0.0; 3], [1.0; 3], [1.0; 3]];
    assert_eq!(
        best_ordered_mix([0.5; 3], &palette, 4),
        PaletteMix {
            low_index: 0,
            high_index: 1,
            high_ratio: 0.5
        }
    );
    // 3/8 is equally far from ratios 1/4 and 1/2. Earlier ratio 1/4 wins.
    assert_eq!(
        best_ordered_mix([0.375; 3], &palette, 4),
        PaletteMix {
            low_index: 0,
            high_index: 1,
            high_ratio: 0.25
        }
    );
    // The initialized first entry wins a tie with the first nonzero mixture.
    assert_eq!(
        best_ordered_mix([0.125; 3], &palette, 4),
        PaletteMix {
            low_index: 0,
            high_index: 0,
            high_ratio: 0.0
        }
    );
}

#[test]
fn matched_search_returns_original_indices_and_preserves_an_earlier_exact_pair() {
    let matcher = matcher(
        &[(1, [0.0; 3]), (3, [1.0; 3]), (5, [0.5; 3])],
        MatchPolicy::SrgbEuclidean,
    );
    assert_eq!(matcher.nearest([0.5; 3]).index, 5);
    assert_eq!(
        best_matched_mix([0.5; 3], &matcher, 4),
        PaletteMix {
            low_index: 1,
            high_index: 3,
            high_ratio: 0.5
        }
    );
}

#[test]
fn hue_interpolation_takes_the_componentwise_midpoint_not_the_shortest_arc() {
    for matching in [
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
    ] {
        let matcher = matcher(
            &[
                (2, [0.5, 0.2, 0.1]),
                (7, [0.5, 0.2, std::f32::consts::TAU - 0.1]),
            ],
            matching,
        );
        // The inherited midpoint hue is PI, opposite the shortest-arc midpoint near zero.
        assert_eq!(
            best_matched_mix([0.5, 0.2, std::f32::consts::PI], &matcher, 4),
            PaletteMix {
                low_index: 2,
                high_index: 7,
                high_ratio: 0.5
            },
            "{matching:?}"
        );
    }
}

#[test]
fn neutral_hue_is_ignored_only_when_the_selected_metric_ignores_it() {
    for matching in [
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
    ] {
        let matcher = matcher(
            &[
                (2, [0.5, 0.0, 0.1]),
                (7, [0.5, 0.0, std::f32::consts::TAU - 0.1]),
            ],
            matching,
        );
        assert_eq!(
            best_matched_mix([0.5, 0.0, std::f32::consts::PI], &matcher, 4),
            PaletteMix {
                low_index: 2,
                high_index: 2,
                high_ratio: 0.0
            }
        );
    }
    let matcher = matcher(
        &[
            (2, [0.5, 0.0, 0.1]),
            (7, [0.5, 0.0, std::f32::consts::TAU - 0.1]),
        ],
        MatchPolicy::OklchEuclidean,
    );
    assert_eq!(
        best_matched_mix([0.5, 0.0, std::f32::consts::PI], &matcher, 4).high_ratio,
        0.5
    );
}
