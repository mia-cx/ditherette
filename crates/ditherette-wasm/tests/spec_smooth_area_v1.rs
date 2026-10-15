//! Proves regional fine/coarse contrast, source boundaries and exact spec/production coverage.
use ditherette_wasm::{
    image::{ImageDimensions, ImageView, Rgba8, RowStride},
    spec::smooth_area_v1,
};

#[test]
fn area_domain_table_matches_complete_square_definition() {
    for (name, w, h, size, tolerance, kind) in [
        ("tiny clipped square", 1, 1, 32768, 0.0, 0),
        ("flat", 17, 13, 4, 0.0, 0),
        ("full-contrast 100-pixel ramp", 101, 13, 64, 0.12, 1),
        ("full-contrast hard step", 17, 13, 16, 0.12, 2),
        ("two-pixel antialiased step", 17, 13, 16, 0.12, 3),
        ("quantized coloured plateaus", 64, 32, 24, 0.12, 4),
        ("isolated strong detail", 17, 13, 4, 0.12, 5),
        ("strong texture", 17, 13, 4, 0.12, 6),
        ("one-byte strictness", 17, 13, 4, 0.0, 7),
        ("one-byte permissive", 17, 13, 4, 0.01, 7),
        ("green-yellow narrow low contrast", 17, 13, 16, 0.25, 8),
        (
            "green-dark-blue same-width high contrast",
            17,
            13,
            16,
            0.25,
            9,
        ),
        ("oblique smooth ramp", 101, 67, 64, 0.12, 10),
        ("diagonal hard boundary", 17, 13, 16, 0.12, 11),
        ("extreme valid side", 32768, 1, 32768, 0.0, 0),
    ] {
        let dims = ImageDimensions::new(w, h).unwrap();
        let mut bytes = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let v = match kind {
                    1 => (255 * x / (w - 1)) as u8,
                    2 => {
                        if x < w / 2 {
                            0
                        } else {
                            255
                        }
                    }
                    3 => {
                        if x < w / 2 {
                            0
                        } else if x == w / 2 {
                            128
                        } else {
                            255
                        }
                    }
                    4 => 40 + ((x + y) / 8) as u8 * 2,
                    5 => {
                        if x == w / 2 && y == h / 2 {
                            255
                        } else {
                            40
                        }
                    }
                    6 => {
                        if (x + y) % 2 == 0 {
                            0
                        } else {
                            255
                        }
                    }
                    7 => 40 + (x % 2) as u8,
                    10 => (255 * (x + y) / (w + h - 2)) as u8,
                    11 => {
                        if x + y < (w + h) / 2 {
                            0
                        } else {
                            255
                        }
                    }
                    _ => 80,
                };
                let mut rgb = [
                    v,
                    v.saturating_add((y % 2) as u8 * if kind == 4 { 1 } else { 0 }),
                    v,
                ];
                if kind == 8 || kind == 9 {
                    rgb = if x < w / 2 {
                        [0, 255, 0]
                    } else if kind == 8 {
                        [255, 255, 0]
                    } else {
                        [0, 0, 128]
                    };
                }
                bytes.extend_from_slice(&[rgb[0], rgb[1], rgb[2], (x + y) as u8]);
            }
        }
        // Literal square enumeration is an independent oracle for the linear sliding implementation.
        let mut expected = vec![false; (w * h) as usize];
        let mut qualified = vec![false; (w * h) as usize];
        let colors: Vec<_> = bytes
            .chunks_exact(4)
            .map(|p| ditherette_wasm::spec::color::oklab::rgb8_to_oklab([p[0], p[1], p[2]]))
            .collect();
        let radius = size / 2;
        // Literal symmetric box sums are independent of the production running-sum scan.
        let mut horizontal = colors.clone();
        let mut coarse = colors.clone();
        if kind != 0 {
            for y in 0..h {
                for x in 0..w {
                    let r = radius.min(x).min(w - 1 - x);
                    let mut sum = [0.0_f64; 3];
                    for nx in x - r..=x + r {
                        for c in 0..3 {
                            sum[c] += f64::from(colors[(y * w + nx) as usize][c]);
                        }
                    }
                    horizontal[(y * w + x) as usize] =
                        sum.map(|v| (v / f64::from(2 * r + 1)) as f32);
                }
            }
            for y in 0..h {
                for x in 0..w {
                    let r = radius.min(y).min(h - 1 - y);
                    let mut sum = [0.0_f64; 3];
                    for ny in y - r..=y + r {
                        for c in 0..3 {
                            sum[c] += f64::from(horizontal[(ny * w + x) as usize][c]);
                        }
                    }
                    coarse[(y * w + x) as usize] = sum.map(|v| (v / f64::from(2 * r + 1)) as f32);
                }
            }
        }
        let sw = size.min(w);
        let sh = size.min(h);
        for top in 0..=h - sh {
            for left in 0..=w - sw {
                let mut sharpness = 0.0_f32;
                for y in top..top + sh {
                    for x in left..left + sw {
                        let p = (y * w + x) as usize;
                        let [dl, da, db] = std::array::from_fn(|c| colors[p][c] - coarse[p][c]);
                        sharpness =
                            sharpness.max((2.0 * (dl * dl + da * da + db * db).sqrt()).min(1.0));
                        for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                            for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                                let left = colors[(y * w + x) as usize];
                                let right = colors[(ny * w + nx) as usize];
                                let [dl, da, db] = std::array::from_fn(|c| left[c] - right[c]);
                                sharpness =
                                    sharpness.max((dl * dl + da * da + db * db).sqrt().min(1.0));
                            }
                        }
                    }
                }
                if sharpness <= tolerance {
                    qualified[(top * w + left) as usize] = true;
                    for y in top..top + sh {
                        for x in left..left + sw {
                            expected[(y * w + x) as usize] = true;
                        }
                    }
                }
            }
        }
        let centres: Vec<_> = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let left = x.saturating_sub((sw - 1) / 2).min(w - sw);
                let top = y.saturating_sub((sh - 1) / 2).min(h - sh);
                qualified[(top * w + left) as usize]
            })
            .collect();
        if kind == 1 || kind == 8 || kind == 10 {
            assert!(
                expected.iter().all(|v| *v),
                "{name}: smooth or low perceptual contrast must qualify"
            );
        }
        if kind == 2 || kind == 3 || kind == 9 || kind == 11 {
            assert!(
                expected.iter().all(|v| !*v),
                "{name}: concentrated high perceptual contrast must fail"
            );
        }
        let mut padded = Vec::new();
        for row in bytes.chunks_exact(w as usize * 4) {
            padded.extend_from_slice(row);
            padded.extend_from_slice(&[231; 7]);
        }
        padded.truncate(padded.len() - 7);
        let view =
            ImageView::<Rgba8>::new(&padded, dims, RowStride::new(w as usize * 4 + 7).unwrap())
                .unwrap();
        let mut reference = vec![[0.0; 3]; smooth_area_v1::scratch_len(dims)];
        smooth_area_v1::select(view, size, tolerance, &mut reference, || Ok::<_, ()>(())).unwrap();
        let mut production = vec![[0.0; 3]; reference.len()];
        ditherette_wasm::prod::dither::smooth_area_v1::select(
            view,
            size,
            tolerance,
            &mut production,
            || Ok::<_, ()>(()),
        )
        .unwrap();
        for i in 0..expected.len() {
            assert!(
                (0.0..=1.0).contains(&reference[i][2]),
                "{name}/{i}: finite coverage"
            );
            if centres.len() == expected.len() {
                assert_eq!(reference[i][2], 1.0, "{name}: coherent interior");
            }
            if centres.is_empty() {
                assert_eq!(reference[i][2], 0.0, "{name}: no qualifying region");
            }
            assert_eq!(production[i][2], reference[i][2], "{name} parity/{i}");
        }
        if kind == 5 {
            let centre = ((h / 2) * w + w / 2) as usize;
            assert_eq!(reference[centre][2], 0.0, "hard detail remains plain");
            assert_eq!(
                reference[centre - 1][2],
                1.0,
                "smooth side reaches the hard boundary"
            );
        }
        smooth_area_v1::select(view, size, 1.0, &mut production, || Ok::<_, ()>(())).unwrap();
        assert!(
            production[..expected.len()].iter().all(|c| c[2] == 1.0),
            "{name}: maximum tolerance"
        );
        let mut rotated = Vec::new();
        for x in 0..w {
            for y in 0..h {
                rotated.extend_from_slice(
                    &bytes[((y * w + x) * 4) as usize..((y * w + x) * 4 + 4) as usize],
                );
            }
        }
        let rotated = ImageView::packed(&rotated, ImageDimensions::new(h, w).unwrap()).unwrap();
        production.resize(smooth_area_v1::scratch_len(rotated.dimensions()), [0.0; 3]);
        smooth_area_v1::select(
            rotated,
            size,
            tolerance,
            &mut production,
            || Ok::<_, ()>(()),
        )
        .unwrap();
        for x in 0..w {
            for y in 0..h {
                assert!(
                    (production[(x * h + y) as usize][2] - reference[(y * w + x) as usize][2])
                        .abs()
                        <= 1e-6,
                    "{name}: transpose"
                );
            }
        }
    }
}
