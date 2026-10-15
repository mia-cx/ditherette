//! Proves spatial support survives RGB8 quantization and rejects edges, tails and texture.
use ditherette_wasm::{
    image::{ImageDimensions, ImageView, Rgba8, RowStride},
    spec::smooth_gradient_v1::select,
};

#[test]
fn gradient_domain_table() {
    // Each row buys a different mathematical domain boundary, not helper coverage.
    for (name, width, height, kind, minimum, lower, upper) in [
        ("flat", 128_u32, 64_u32, 0, 24, 0.0, 0.0),
        ("hard diagonal", 128, 64, 1, 24, 0.0, 0.0),
        ("short with long tails", 2048, 32, 2, 24, 0.0, 0.0),
        ("repeated-byte ramp", 2048, 32, 3, 24, 0.95, 1.0),
        ("nonlinear broad ramp", 2048, 32, 4, 256, 0.8, 1.0),
        ("oblique ramp", 128, 96, 5, 24, 0.85, 1.0),
        ("texture", 128, 64, 6, 24, 0.0, 0.0),
        ("larger than image", 128, 64, 3, 32768, 0.0, 0.0),
        ("one-pixel image", 1, 1, 0, 4, 0.0, 0.0),
        ("one-row ramp", 1024, 1, 3, 24, 0.95, 1.0),
        ("equal slopes in a rectangle", 96, 64, 7, 50, 0.95, 1.0),
        ("short gentle ramp", 128, 64, 8, 4, 0.04, 0.09),
        ("short gentle ramp excluded", 128, 64, 8, 24, 0.0, 0.0),
        ("quantized coloured plane", 512, 128, 9, 4, 0.95, 1.0),
    ] {
        let mut bytes = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let v = match kind {
                    1 => {
                        if x + y < 90 {
                            64
                        } else {
                            192
                        }
                    }
                    2 => 64 + x.saturating_sub(1000).min(8) * 16,
                    3 => 64 + (128.0 * x as f64 / (width - 1) as f64).round() as u32,
                    4 => 64 + (128.0 * (x as f64 / (width - 1) as f64).powi(2)).round() as u32,
                    5 => 64 + (0.5 * x as f64 + 0.7 * y as f64).round() as u32,
                    6 => 64 + (x % 8) * 8,
                    7 => 64 + x + y,
                    8 => 64 + x.saturating_sub(48).min(8) * 4,
                    _ => 128,
                } as u8;
                let rgb = if kind == 9 {
                    let x = x as f64;
                    let y = y as f64;
                    [
                        (2.0 * ((50.0 - 0.004 * x - 0.003 * y) / 2.0).round()) as u8,
                        (2.0 * ((10.0 - 0.001 * x - 0.004 * y) / 2.0).round()) as u8,
                        (2.0 * ((72.0 - 0.03 * x - 0.04 * y) / 2.0).round()) as u8,
                    ]
                } else {
                    [v; 3]
                };
                bytes.extend_from_slice(&[rgb[0], rgb[1], rgb[2], (x + y) as u8]);
            }
        }
        let source =
            ImageView::<Rgba8>::packed(&bytes, ImageDimensions::new(width, height).unwrap())
                .unwrap();
        let mut scratch = vec![[0.0; 3]; (width * height) as usize];
        select(source, minimum, &mut scratch, || Ok::<_, ()>(())).unwrap();
        let mut padded = Vec::new();
        for row in bytes.chunks_exact(width as usize * 4) {
            padded.extend_from_slice(row);
            padded.extend_from_slice(&[231; 7]);
        }
        padded.truncate(padded.len() - 7);
        let strided = ImageView::new(
            &padded,
            source.dimensions(),
            RowStride::new(width as usize * 4 + 7).unwrap(),
        )
        .unwrap();
        let mut actual = vec![[99.0; 3]; scratch.len()];
        ditherette_wasm::prod::dither::smooth_gradient_v1::select(
            strided,
            minimum,
            &mut actual,
            || Ok::<_, ()>(()),
        )
        .unwrap();
        assert_eq!(actual, scratch, "{name}: exact strided spec/prod analysis");
        let mut rotated = Vec::with_capacity(bytes.len());
        for x in 0..width {
            for y in 0..height {
                let i = (y * width + x) as usize * 4;
                rotated.extend_from_slice(&bytes[i..i + 4]);
            }
        }
        let transposed =
            ImageView::packed(&rotated, ImageDimensions::new(height, width).unwrap()).unwrap();
        let mut rotated_mask = vec![[0.0; 3]; scratch.len()];
        select(transposed, minimum, &mut rotated_mask, || Ok::<_, ()>(())).unwrap();
        for x in 0..width {
            for y in 0..height {
                assert_eq!(
                    scratch[(y * width + x) as usize][2],
                    rotated_mask[(x * height + y) as usize][2],
                    "{name}: axis orientation {x},{y}"
                );
            }
        }
        let mut larger = vec![[0.0; 3]; scratch.len()];
        select(
            source,
            minimum.saturating_mul(2).min(32768),
            &mut larger,
            || Ok::<_, ()>(()),
        )
        .unwrap();
        assert!(
            larger
                .iter()
                .zip(&scratch)
                .all(|(large, small)| large[2] <= small[2]),
            "{name}: increasing minimum only removes pixels"
        );
        let selected = scratch.iter().filter(|cell| cell[2] == 1.0).count();
        let coverage = selected as f64 / scratch.len() as f64;
        eprintln!(
            "{name} minimum{minimum}: {selected}/{} ({coverage:.4})",
            scratch.len()
        );
        assert!((lower..=upper).contains(&coverage), "{name}: {coverage}");
    }
}
