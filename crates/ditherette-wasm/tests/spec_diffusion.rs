use ditherette_wasm::spec::dither::error_diffusion::{
    ErrorDiffusionKernel, ATKINSON_TAPS, FLOYD_STEINBERG_TAPS, SIERRA_LITE_TAPS, SIERRA_TAPS,
};

#[test]
fn tap_positions_weights_and_intentional_atkinson_loss_are_exact() {
    // Numerators and denominators make normalization independent of the production constants.
    let cases: &[(ErrorDiffusionKernel, &[(i32, u32, u32)], u32, f32)] = &[
        (
            ErrorDiffusionKernel::FloydSteinberg,
            &[(1, 0, 7), (-1, 1, 3), (0, 1, 5), (1, 1, 1)],
            16,
            1.0,
        ),
        (
            ErrorDiffusionKernel::Sierra,
            &[
                (1, 0, 5),
                (2, 0, 3),
                (-2, 1, 2),
                (-1, 1, 4),
                (0, 1, 5),
                (1, 1, 4),
                (2, 1, 2),
                (-1, 2, 2),
                (0, 2, 3),
                (1, 2, 2),
            ],
            32,
            1.0,
        ),
        (
            ErrorDiffusionKernel::SierraLite,
            &[(1, 0, 2), (-1, 1, 1), (0, 1, 1)],
            4,
            1.0,
        ),
        (
            ErrorDiffusionKernel::Atkinson,
            &[
                (1, 0, 1),
                (2, 0, 1),
                (-1, 1, 1),
                (0, 1, 1),
                (1, 1, 1),
                (0, 2, 1),
            ],
            8,
            0.75,
        ),
    ];
    for (kernel, positions, denominator, sum) in cases {
        let taps = kernel.taps();
        assert_eq!(taps.len(), positions.len());
        for (tap, &(dx, dy, numerator)) in taps.iter().zip(*positions) {
            assert_eq!((tap.dx, tap.dy), (dx, dy));
            assert_eq!(tap.weight, numerator as f32 / *denominator as f32);
            assert!(dy > 0 || dx > 0, "every tap follows the raster scan");
        }
        assert_eq!(taps.iter().map(|tap| tap.weight).sum::<f32>(), *sum);
    }
    assert_eq!(
        ErrorDiffusionKernel::FloydSteinberg.taps(),
        FLOYD_STEINBERG_TAPS
    );
    assert_eq!(ErrorDiffusionKernel::Sierra.taps(), SIERRA_TAPS);
    assert_eq!(ErrorDiffusionKernel::SierraLite.taps(), SIERRA_LITE_TAPS);
    assert_eq!(ErrorDiffusionKernel::Atkinson.taps(), ATKINSON_TAPS);
}
