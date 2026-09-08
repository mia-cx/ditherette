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

#[test]
fn disjoint_bands_share_preparation_and_preserve_all_matching_alpha_and_metadata() {
    let dimensions = ImageDimensions::new(7, 9).unwrap();
    let packed: Vec<u8> = (0..63u32)
        .flat_map(|n| {
            [
                (n * 73) as u8,
                (n * 31 + 127) as u8,
                (n * 17 + 255) as u8,
                [0, 1, 127, 128, 254, 255][n as usize % 6],
            ]
        })
        .collect();
    let mut strided = vec![203; 33 * 9];
    for (source, target) in packed.chunks_exact(28).zip(strided.chunks_exact_mut(33)) {
        target[..28].copy_from_slice(source);
    }
    let source =
        ImageView::<Rgba8>::new(&strided, dimensions, RowStride::new(33).unwrap()).unwrap();
    let palette = [
        PaletteEntry::Color { rgb: [0, 0, 0] },
        PaletteEntry::Color {
            rgb: [255, 255, 255],
        },
        PaletteEntry::Color { rgb: [17, 97, 173] },
        PaletteEntry::Color { rgb: [17, 97, 173] },
        PaletteEntry::Transparent {},
    ];
    use MatchPolicy::*;
    for matching in [
        SrgbEuclidean,
        LinearRgbEuclidean,
        OklabEuclidean,
        CielabEuclidean,
        YcbcrEuclidean,
        SrgbCompuphase,
        SrgbRec601,
        SrgbRec709,
        OklchEuclidean,
        OklchCircularHue,
        OklchHueArc,
        CielabCiede2000,
        CielchEuclidean,
        CielchCircularHue,
        CielchHueArc,
    ] {
        for alpha in [
            AlphaPolicy::Preserve { threshold: 127.5 },
            AlphaPolicy::Premultiplied {},
            AlphaPolicy::Matte { rgb: [23, 71, 139] },
        ] {
            let oracle = spec::quantize::quantize(spec::contract::request::QuantizeRequest {
                version: 1,
                source: spec::contract::request::Source {
                    width: 7,
                    height: 9,
                    data: &packed,
                },
                palette: &palette,
                alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap()).unwrap(),
                matching: serde_json::from_value(serde_json::to_value(matching).unwrap()).unwrap(),
            })
            .unwrap();
            let capacity =
                PreparedQuantizer::required_capacity_bytes(&palette, alpha, matching).unwrap();
            assert!(PreparedQuantizer::try_new(&palette, alpha, matching, capacity - 1).is_err());
            let prepared = PreparedQuantizer::try_new(&palette, alpha, matching, capacity).unwrap();
            assert_eq!(prepared.capacity_bytes(), capacity);
            let mut last = Vec::new();
            for (workers, band_height) in [(1, 1), (2, 2), (4, 3), (4, 11)] {
                let mut guarded = vec![211; 65];
                let output = &mut guarded[1..64];
                let worker_rows = dimensions.height_usize().div_ceil(workers);
                std::thread::scope(|scope| {
                    for (worker, output) in output.chunks_mut(7 * worker_rows).enumerate() {
                        let prepared = &prepared;
                        scope.spawn(move || {
                            for (band, output) in output.chunks_mut(7 * band_height).enumerate() {
                                let start = worker * worker_rows + band * band_height;
                                let rows =
                                    RowBand::new(start as u32, (start + output.len() / 7) as u32)
                                        .unwrap();
                                prepared.quantize_rows_into(source, rows, output);
                            }
                        });
                    }
                });
                assert_eq!(&guarded[1..64], oracle.indices.data());
                assert_eq!([guarded[0], guarded[64]], [211, 211]);
                last = guarded[1..64].to_vec();
            }
            let actual = prepared.into_indexed(
                ImageBuf::<PaletteIndex8>::from_vec_packed(last, dimensions).unwrap(),
            );
            assert_eq!(actual.palette, oracle.palette);
            assert_eq!(actual.warnings, oracle.warnings);
        }
    }
    assert!(strided.chunks_exact(33).all(|row| row[28..] == [203; 5]));
}
