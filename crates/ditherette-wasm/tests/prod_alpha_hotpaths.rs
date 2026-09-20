use ditherette_wasm::{
    image::{
        contracts::PaletteEntry, ImageBuf, ImageDimensions, ImageView, PaletteIndex8, Rgba8,
        RowStride,
    },
    prod::{
        contract::request::{AlphaPolicy, MatchPolicy},
        quantize::PreparedQuantizer,
        tiling::RowBand,
    },
    spec,
};

fn all_alpha_bytes(colors: &[[u8; 3]]) -> Vec<u8> {
    (0..=255u8)
        .flat_map(|alpha| colors.iter().flat_map(move |&[r, g, b]| [r, g, b, alpha]))
        .collect()
}

fn assert_hotpaths_match_spec(
    packed: &[u8],
    palette: &[PaletteEntry],
    alpha: AlphaPolicy,
    matching: MatchPolicy,
    scratch: &mut [u64],
) {
    let dimensions = ImageDimensions::new(32, (packed.len() / 4 / 32) as u32).unwrap();
    let expected = spec::quantize::quantize(spec::contract::request::QuantizeRequest {
        version: 1,
        source: spec::contract::request::Source {
            width: dimensions.width(),
            height: dimensions.height(),
            data: packed,
        },
        palette,
        alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap()).unwrap(),
        matching: serde_json::from_value(serde_json::to_value(matching).unwrap()).unwrap(),
    })
    .unwrap();

    // Padding catches traversals that consume the full backing slice as packed pixels.
    let row_bytes = dimensions.width_usize() * 4;
    let stride = row_bytes + 5;
    let mut strided = vec![203; stride * dimensions.height() as usize];
    for (source, target) in packed
        .chunks_exact(row_bytes)
        .zip(strided.chunks_exact_mut(stride))
    {
        target[..row_bytes].copy_from_slice(source);
    }
    let source =
        ImageView::<Rgba8>::new(&strided, dimensions, RowStride::new(stride).unwrap()).unwrap();
    let prepared = PreparedQuantizer::try_new(palette, alpha, matching, u64::MAX).unwrap();
    let mut output = vec![211; packed.len() / 4];
    prepared.quantize_into(source, &mut output);
    assert_eq!(
        output,
        expected.indices.data(),
        "direct: {alpha:?}, {matching:?}"
    );

    output.fill(211);
    prepared
        .quantize_cached_with_progress(source, &mut output, scratch, |_| Ok(()))
        .unwrap();
    assert_eq!(
        output,
        expected.indices.data(),
        "cached: {alpha:?}, {matching:?}"
    );

    output.fill(211);
    // Reverse order forces each band to use absolute source rows, including a short final band.
    for start in (0..dimensions.height()).step_by(7).rev() {
        let end = (start + 7).min(dimensions.height());
        prepared.quantize_rows_into(
            source,
            RowBand::new(start, end).unwrap(),
            &mut output[start as usize * 32..end as usize * 32],
        );
    }
    let actual = prepared
        .into_indexed(ImageBuf::<PaletteIndex8>::from_vec_packed(output, dimensions).unwrap());
    assert_eq!(actual, expected, "row bands: {alpha:?}, {matching:?}");
}

#[test]
fn every_alpha_byte_retains_fractional_thresholds_and_palette_fallbacks() {
    let source = all_alpha_bytes(&[
        [0, 0, 0],
        [255, 255, 255],
        [1, 127, 254],
        [254, 128, 1],
        [17, 73, 211],
        [211, 17, 73],
        [73, 211, 17],
        [128, 128, 128],
    ]);
    let color = |rgb| PaletteEntry::Color { rgb };
    let palettes = [
        vec![PaletteEntry::Transparent {}, PaletteEntry::Transparent {}],
        vec![
            color([255; 3]),
            PaletteEntry::Transparent {},
            color([1, 0, 0]),
            color([0, 1, 0]),
            color([17, 73, 211]),
            PaletteEntry::Transparent {},
        ],
        // Equal darkest sums must retain the first original index, which is not zero.
        vec![
            color([255; 3]),
            color([1, 0, 0]),
            color([0, 1, 0]),
            color([17, 73, 211]),
        ],
    ];
    let mut scratch = [u64::MAX; 16];
    for matching in [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::SrgbCompuphase,
    ] {
        for palette in &palettes {
            for alpha in [
                AlphaPolicy::Preserve { threshold: 0.0 },
                AlphaPolicy::Preserve { threshold: 127.5 },
                AlphaPolicy::Preserve { threshold: 128.0 },
                AlphaPolicy::Preserve { threshold: 254.999 },
                AlphaPolicy::Preserve { threshold: 255.0 },
                AlphaPolicy::Premultiplied {},
                AlphaPolicy::Matte { rgb: [1, 127, 254] },
            ] {
                assert_hotpaths_match_spec(&source, palette, alpha, matching, &mut scratch);
            }
        }
    }
}

#[test]
fn compositing_channel_rounding_matches_spec_at_every_alpha_byte() {
    let source = all_alpha_bytes(&[
        [0, 255, 1],
        [1, 254, 2],
        [2, 253, 0],
        [63, 192, 127],
        [64, 191, 128],
        [127, 128, 129],
        [128, 127, 254],
        [129, 126, 255],
        [191, 64, 63],
        [192, 63, 64],
        [253, 2, 192],
        [254, 1, 191],
        [255, 0, 253],
        [17, 73, 211],
        [73, 211, 17],
        [211, 17, 73],
    ]);
    let mut scratch = [u64::MAX; 16];
    for channel in 0..3 {
        // With an sRGB axis palette, the index equals the composited channel byte.
        // Other channels contribute the same distance to every candidate.
        let palette: Vec<_> = (0..=255u8)
            .map(|value| {
                let mut rgb = [0; 3];
                rgb[channel] = value;
                PaletteEntry::Color { rgb }
            })
            .collect();
        for alpha in [
            AlphaPolicy::Premultiplied {},
            AlphaPolicy::Matte { rgb: [0; 3] },
            AlphaPolicy::Matte { rgb: [255; 3] },
            AlphaPolicy::Matte { rgb: [1, 127, 254] },
            AlphaPolicy::Matte { rgb: [254, 128, 1] },
        ] {
            assert_hotpaths_match_spec(
                &source,
                &palette,
                alpha,
                MatchPolicy::SrgbEuclidean,
                &mut scratch,
            );
        }
    }
}
