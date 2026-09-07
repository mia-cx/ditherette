use ditherette_wasm::spec::{
    color::srgb::{rgb8_to_srgb, srgb_to_rgb8},
    dither::blue_noise::{blue_noise_at, BLUE_NOISE_32X32, BLUE_NOISE_SIDE},
};

#[cfg(not(target_arch = "wasm32"))]
#[path = "../src/spec/dither/blue_noise/generator.rs"]
mod generator;

#[test]
fn rank_midpoints_have_exact_zero_mean_and_exclude_the_endpoints() {
    let samples: Vec<_> = (0..BLUE_NOISE_SIDE)
        .flat_map(|y| (0..BLUE_NOISE_SIDE).map(move |x| blue_noise_at(x, y)))
        .collect();
    assert_eq!(samples.iter().sum::<f32>(), 0.0);
    assert!(samples.iter().all(|sample| *sample > -0.5 && *sample < 0.5));
    assert_eq!(blue_noise_at(23, 13), -0.5 + 1.0 / 2048.0);
    assert_eq!(blue_noise_at(6, 27), 0.5 - 1.0 / 2048.0);
}

#[test]
fn lookup_uses_global_coordinates_across_rows_bands_and_large_origins() {
    for y in [0, 1, 31, 32, 33, 95, u32::MAX] {
        for x in [0, 1, 31, 32, 33, 65, u32::MAX] {
            assert_eq!(blue_noise_at(x, y), blue_noise_at(x % 32, y % 32));
        }
    }
    let complete: Vec<_> = (0..66)
        .flat_map(|y| (0..35).map(move |x| blue_noise_at(x, y)))
        .collect();
    let bands: Vec<_> = [0..17, 17..33, 33..66]
        .into_iter()
        .flat_map(|rows| rows.flat_map(|y| (0..35).map(move |x| blue_noise_at(x, y))))
        .collect();
    assert_eq!(bands, complete);
}

#[test]
fn fixed_srgb_rgba8_composition_rounds_channels_and_preserves_hidden_rgb_alpha() {
    let source = [
        [128, 128, 128, 0],
        [128, 128, 128, 1],
        [128, 128, 128, 127],
        [128, 128, 128, 255],
    ];
    let output: Vec<_> = source
        .iter()
        .enumerate()
        .map(|(x, pixel)| {
            let color = rgb8_to_srgb([pixel[0], pixel[1], pixel[2]]);
            let noise = blue_noise_at(x as u32, 0);
            let [r, g, b] = srgb_to_rgb8(color.map(|channel| channel + 0.25 * noise));
            [r, g, b, pixel[3]]
        })
        .collect();
    // First ranks303,763,3,874 produce byte offsets -13,+16,-32,+23.
    assert_eq!(
        output,
        [
            [115, 115, 115, 0],
            [144, 144, 144, 1],
            [96, 96, 96, 127],
            [151, 151, 151, 255]
        ]
    );
    assert_eq!(source[0], [128, 128, 128, 0]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn generator_reproduces_the_asset_and_all_preregistered_spectral_gates() {
    let construction = generator::generate().unwrap();
    assert_eq!(construction.ranks, BLUE_NOISE_32X32);
    assert_eq!(construction.relaxation_moves, 45);
    assert!(construction.passes, "{:#?}", construction.spectra);
    let recorded: serde_json::Value =
        serde_json::from_str(include_str!("../src/spec/dither/blue_noise/analysis.json")).unwrap();
    for (actual, saved) in construction
        .spectra
        .iter()
        .zip(recorded["spectra"].as_array().unwrap())
    {
        assert_eq!(
            actual.occupied,
            saved["occupied"].as_u64().unwrap() as usize
        );
        assert!(
            (actual.low_mean_over_white - saved["low_mean_over_white"].as_f64().unwrap()).abs()
                < 1e-12
        );
        assert!((actual.peak_fraction - saved["peak_fraction"].as_f64().unwrap()).abs() < 1e-12);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn periodic_density_counts_each_site_once_and_wraps_at_the_seam() {
    let mut pattern = [false; generator::COUNT];
    pattern[0] = true;
    assert_eq!(generator::density(&pattern, 0, true), 1.0);
    let neighbor_weight = (-1.0_f64 / (2.0 * 1.5 * 1.5)).exp();
    for site in [1, 31, 32, 992] {
        assert_eq!(generator::density(&pattern, site, true), neighbor_weight);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn spectral_gates_reject_the_old_bayer_defect_and_low_frequency_stripes() {
    use ditherette_wasm::spec::dither::ordered::bayer_value;
    // Repeat transposed Bayer8, assigning distinct low bits to produce 1024 ranks.
    let old: Vec<_> = (0..1024)
        .map(|site| {
            let x = site % 32;
            let y = site / 32;
            bayer_value(y % 8, x % 8, 8) * 16 + (y / 8 * 4 + x / 8) as u16
        })
        .collect();
    let old_report = generator::spectrum(&old, 512);
    assert!(!old_report.passes);
    assert!((old_report.peak_fraction - 1.0).abs() < 1e-12);
    let stripes: Vec<u16> = (0..1024).collect();
    let stripe_report = generator::spectrum(&stripes, 512);
    assert!(!stripe_report.passes);
    assert!(stripe_report.low_mean_over_white > 0.2);
}
