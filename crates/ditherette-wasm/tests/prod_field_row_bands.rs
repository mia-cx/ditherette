use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8, RowStride},
    prod::{
        self,
        contract::request::{Placement, WorkingSpace},
        tiling::WorkerBudget,
    },
    spec,
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn disjoint_field_outputs_keep_global_draws_and_full_source_adaptive_neighbors() {
    let dimensions = ImageDimensions::new(5, 7).unwrap();
    let mut bytes = vec![203; 25 * 7];
    for (y, row) in bytes.chunks_exact_mut(25).enumerate() {
        for x in 0..5 {
            let n = y * 5 + x;
            row[x * 4..x * 4 + 4].copy_from_slice(&[
                (n * 73) as u8,
                (n * 31 + 127) as u8,
                (n * 17 + 255) as u8,
                [0, 1, 127, 128, 254, 255][n % 6],
            ]);
        }
    }
    let original = bytes.clone();
    let source = ImageView::<Rgba8>::new(&bytes, dimensions, RowStride::new(25).unwrap()).unwrap();
    use WorkingSpace::*;
    for space in [Srgb, LinearRgb, Oklab, Oklch, Cielab, Cielch, Ycbcr] {
        for placement in [
            Placement::Everywhere {},
            Placement::Adaptive {
                radius: 2,
                threshold: 0.01,
                softness: 0.3,
            },
            Placement::Adaptive {
                radius: 8,
                threshold: 0.1,
                softness: 0.2,
            },
        ] {
            for strength in [0.0, 1.25] {
                for field in 0..6 {
                    let noise = |x, y, index| match field {
                        0..=3 => prod::dither::ordered::bayer_noise_at(
                            x,
                            y,
                            [
                                prod::dither::ordered::BayerSize::Two,
                                prod::dither::ordered::BayerSize::Four,
                                prod::dither::ordered::BayerSize::Eight,
                                prod::dither::ordered::BayerSize::Sixteen,
                            ][field],
                        ),
                        4 => prod::dither::random_noise::random_noise_at(0xdead_beef, index),
                        _ => prod::dither::blue_noise::blue_noise_at(x, y),
                    };
                    let mut expected = vec![0; 140];
                    spec::dither::perturb::perturb_by_field_rows_into(
                        source,
                        ImageViewMut::packed(&mut expected, dimensions).unwrap(),
                        serde_json::from_value(serde_json::to_value(space).unwrap()).unwrap(),
                        strength,
                        serde_json::from_value(serde_json::to_value(placement).unwrap()).unwrap(),
                        spec::tiling::RowBand::new(0, 7).unwrap(),
                        noise,
                    );
                    for (workers, band_height) in [(1, 1), (2, 2), (4, 3), (4, 11)] {
                        let counts: Vec<_> = (0..35).map(|_| AtomicUsize::new(0)).collect();
                        let mut guarded = vec![211; 142];
                        let output = &mut guarded[1..141];
                        let mut work = prod::dither::perturb::try_band_buffers(
                            dimensions,
                            band_height,
                            WorkerBudget::new(workers),
                            workers,
                            u64::MAX,
                        )
                        .unwrap();
                        let caller = std::thread::current().id();
                        let mut completed = 0;
                        prod::dither::perturb::perturb_by_field_bands_into(
                            source,
                            output,
                            space,
                            strength,
                            placement,
                            &mut work,
                            |x, y, index| {
                                assert_eq!(index, u64::from(y) * 5 + u64::from(x));
                                counts[index as usize].fetch_add(1, Ordering::Relaxed);
                                noise(x, y, index)
                            },
                            &mut |rows| {
                                assert_eq!(std::thread::current().id(), caller);
                                assert!(rows > completed);
                                completed = rows;
                                Ok(())
                            },
                        )
                        .unwrap();
                        assert_eq!(completed, 7);
                        assert_eq!(&guarded[1..141], expected);
                        assert_eq!([guarded[0], guarded[141]], [211, 211]);
                        assert!(counts
                            .iter()
                            .all(|count| count.load(Ordering::Relaxed) == 1));
                    }
                }
            }
        }
    }
    assert_eq!(bytes, original);
}
