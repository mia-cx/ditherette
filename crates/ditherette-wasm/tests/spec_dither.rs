use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, LinearRgb32, PaletteIndex8},
    spec::dither::{
        blue_noise::{dither_blue_noise_into, BLUE_NOISE_8X8},
        error_diffusion::{dither_error_diffusion_into, ErrorDiffusionKernel},
        ordered::{bayer_value, dither_bayer_into, BayerSize},
        random_noise::{dither_random_noise_into, Mulberry32},
        yiluoma::{best_ordered_mix, dither_yiluoma_into},
    },
};

fn view<'a>(data: &'a [f32], dimensions: ImageDimensions) -> ImageView<'a, LinearRgb32> {
    ImageView::<LinearRgb32>::packed(data, dimensions).unwrap()
}

fn output_view<'a>(
    data: &'a mut [u8],
    dimensions: ImageDimensions,
) -> ImageViewMut<'a, PaletteIndex8> {
    ImageViewMut::<PaletteIndex8>::packed(data, dimensions).unwrap()
}

#[test]
fn bayer_values_cover_the_matrix_range() {
    let mut values = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            values.push(bayer_value(x, y, 4));
        }
    }
    values.sort_unstable();

    assert_eq!(values, (0..16).collect::<Vec<_>>());
}

#[test]
fn bayer_strength_zero_matches_direct_nearest_quantization() {
    let dimensions = ImageDimensions::new(3, 1).unwrap();
    let source = [0.1, 0.1, 0.1, 0.6, 0.6, 0.6, 0.9, 0.9, 0.9];
    let palette = [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
    let mut output = [255; 3];

    dither_bayer_into(
        view(&source, dimensions),
        &palette,
        output_view(&mut output, dimensions),
        BayerSize::Four,
        0.0,
    );

    assert_eq!(output, [0, 1, 1]);
}

#[test]
fn random_noise_is_seeded_and_repeatable() {
    let mut left = Mulberry32::new(123);
    let mut right = Mulberry32::new(123);

    assert_eq!(left.next_u32(), right.next_u32());
    assert_eq!(left.next_u32(), right.next_u32());
}

#[test]
fn random_noise_dither_is_deterministic_for_same_seed() {
    let dimensions = ImageDimensions::new(8, 1).unwrap();
    let source = vec![0.5; dimensions.storage_len::<LinearRgb32>().unwrap()];
    let palette = [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
    let mut left = [0; 8];
    let mut right = [0; 8];

    dither_random_noise_into(
        view(&source, dimensions),
        &palette,
        output_view(&mut left, dimensions),
        42,
        1.0,
    );
    dither_random_noise_into(
        view(&source, dimensions),
        &palette,
        output_view(&mut right, dimensions),
        42,
        1.0,
    );

    assert_eq!(left, right);
}

#[test]
fn blue_noise_tile_contains_each_threshold_once() {
    let mut values = BLUE_NOISE_8X8.to_vec();
    values.sort_unstable();

    assert_eq!(values, (0..64).collect::<Vec<_>>());
}

#[test]
fn blue_noise_strength_zero_matches_direct_nearest_quantization() {
    let dimensions = ImageDimensions::new(2, 1).unwrap();
    let source = [0.2, 0.2, 0.2, 0.8, 0.8, 0.8];
    let palette = [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
    let mut output = [255; 2];

    dither_blue_noise_into(
        view(&source, dimensions),
        &palette,
        output_view(&mut output, dimensions),
        0.0,
    );

    assert_eq!(output, [0, 1]);
}

#[test]
fn error_diffusion_supports_requested_kernels() {
    let dimensions = ImageDimensions::new(4, 1).unwrap();
    let source = [0.2, 0.2, 0.2, 0.4, 0.4, 0.4, 0.6, 0.6, 0.6, 0.8, 0.8, 0.8];
    let palette = [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];

    for kernel in [
        ErrorDiffusionKernel::FloydSteinberg,
        ErrorDiffusionKernel::Sierra,
        ErrorDiffusionKernel::SierraLite,
        ErrorDiffusionKernel::Atkinson,
    ] {
        let mut output = [255; 4];
        dither_error_diffusion_into(
            view(&source, dimensions),
            &palette,
            output_view(&mut output, dimensions),
            kernel,
            1.0,
            false,
        );
        assert!(output.iter().all(|&index| index <= 1));
    }
}

#[test]
fn error_diffusion_strength_zero_matches_direct_nearest_quantization() {
    let dimensions = ImageDimensions::new(3, 1).unwrap();
    let source = [0.2, 0.2, 0.2, 0.49, 0.49, 0.49, 0.8, 0.8, 0.8];
    let palette = [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
    let mut output = [255; 3];

    dither_error_diffusion_into(
        view(&source, dimensions),
        &palette,
        output_view(&mut output, dimensions),
        ErrorDiffusionKernel::FloydSteinberg,
        0.0,
        false,
    );

    assert_eq!(output, [0, 0, 1]);
}

#[test]
fn yiluoma_best_mix_finds_two_color_approximation() {
    let palette = [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
    let mix = best_ordered_mix([0.25, 0.25, 0.25], &palette, 4);

    assert_eq!(mix.low_index, 0);
    assert_eq!(mix.high_index, 1);
    assert_eq!(mix.high_ratio, 0.25);
}

#[test]
fn yiluoma_dither_outputs_palette_indices() {
    let dimensions = ImageDimensions::new(4, 1).unwrap();
    let source = vec![0.25; dimensions.storage_len::<LinearRgb32>().unwrap()];
    let palette = [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
    let mut output = [255; 4];

    dither_yiluoma_into(
        view(&source, dimensions),
        &palette,
        output_view(&mut output, dimensions),
        BayerSize::Two,
    );

    assert!(output.iter().all(|&index| index <= 1));
    assert!(output.contains(&1));
}

#[test]
fn bayer_can_use_custom_nearest_metric() {
    use ditherette_wasm::spec::dither::ordered::dither_bayer_by_nearest_into;

    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let source = [0.9, 0.0, 0.0];
    let palette = [[1.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
    let mut output = [255; 1];

    dither_bayer_by_nearest_into(
        view(&source, dimensions),
        &palette,
        output_view(&mut output, dimensions),
        BayerSize::Two,
        0.0,
        |_color, palette| (1, palette[1]),
    );

    assert_eq!(output, [1]);
}

#[test]
fn error_diffusion_can_use_custom_nearest_metric() {
    use ditherette_wasm::spec::dither::error_diffusion::dither_error_diffusion_by_nearest_into;

    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let source = [0.9, 0.0, 0.0];
    let palette = [[1.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
    let mut output = [255; 1];

    dither_error_diffusion_by_nearest_into(
        view(&source, dimensions),
        &palette,
        output_view(&mut output, dimensions),
        ErrorDiffusionKernel::FloydSteinberg,
        1.0,
        false,
        |_color, palette| (1, palette[1]),
    );

    assert_eq!(output, [1]);
}
