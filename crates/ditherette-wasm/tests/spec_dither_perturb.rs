use ditherette_wasm::spec::dither::{
    ordered::{bayer_noise_at, BayerSize},
    random_noise::{random_noise_at, random_u32_at, Mulberry32},
};

#[test]
fn bayer_orientation_matches_the_website_and_all_sizes_cover_centered_ranks() {
    let four = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(
                bayer_noise_at(x, y, BayerSize::Four),
                (four[y as usize][x as usize] as f32 + 0.5) / 16.0 - 0.5
            );
        }
    }
    for size in [
        BayerSize::Two,
        BayerSize::Four,
        BayerSize::Eight,
        BayerSize::Sixteen,
    ] {
        let width = size.width() as u32;
        let mut values = Vec::new();
        for y in 0..width {
            for x in 0..width {
                let value = bayer_noise_at(x, y, size);
                assert_eq!(value, bayer_noise_at(x + width, y + 3 * width, size));
                values.push(value);
            }
        }
        values.sort_by(f32::total_cmp);
        assert_eq!(values.iter().sum::<f32>(), 0.0);
        for (rank, value) in values.into_iter().enumerate() {
            assert_eq!(value, (rank as f32 + 0.5) / (width * width) as f32 - 0.5);
        }
    }
}

#[test]
fn global_random_draws_match_independent_javascript_integer_fixtures() {
    // Independent Math.imul evaluation of the existing website mixer, not Rust-generated snapshots.
    for (seed, expected) in [
        (
            0,
            [
                1144304738, 1416247, 958946056, 2787370982, 1958536065, 2834277798, 3461135945,
            ],
        ),
        (
            1,
            [
                2693262067, 11749833, 2265367787, 3095568220, 1828783984, 663542962, 2100857034,
            ],
        ),
        (
            u32::MAX,
            [
                3850105811, 813802916, 3073704848, 2042566601, 583504547, 3250051920, 3933861565,
            ],
        ),
    ] {
        for (index, expected) in [0, 1, 2, 7, 8, 15, 16].into_iter().zip(expected) {
            assert_eq!(random_u32_at(seed, index), expected);
            assert_eq!(
                random_noise_at(seed, index),
                (f64::from(expected) / 4294967296.0 - 0.5) as f32
            );
        }
        let mut stream = Mulberry32::new(seed);
        for index in 0..100 {
            assert_eq!(random_u32_at(seed, index), stream.next_u32());
        }
    }
}

#[test]
fn global_random_index_wraps_only_at_the_defined_u32_sequence_period() {
    for (seed, last_draw) in [(0, 0), (1, 63), (u32::MAX, 142530043)] {
        assert_eq!(random_u32_at(seed, u64::from(u32::MAX)), last_draw);
        assert_eq!(random_u32_at(seed, 1_u64 << 32), random_u32_at(seed, 0));
    }
    assert_ne!(random_noise_at(0, 16), random_noise_at(1, 16));
}
