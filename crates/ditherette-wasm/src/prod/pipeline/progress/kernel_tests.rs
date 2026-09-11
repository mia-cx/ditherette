use super::*;
use crate::{
    image::{contracts::PaletteEntry, ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::request::{
            AlphaPolicy, Diffusion, DiffusionFeedback, DitherPolicy, MatchPolicy, Placement,
            WorkingSpace,
        },
        dither::{
            error_diffusion::prepared::{execute_with_progress, DiffusionPolicy},
            ordered::BayerSize,
            perturb::perturb_by_field_with_progress,
            yiluoma::dither_yiluoma_with_progress,
        },
        quantize::PreparedQuantizer,
        tiling::RowBand,
    },
};

fn check_rows(
    row_bytes: usize,
    mut run: impl FnMut(&mut [u8], &mut dyn FnMut(u32) -> Result<(), Failure>) -> Result<(), Failure>,
) {
    let mut expected = vec![203; row_bytes * 4];
    run(&mut expected, &mut |_| Ok(())).unwrap();
    let mut actual = vec![203; expected.len()];
    let mut rows = Vec::new();
    run(&mut actual, &mut |row| {
        rows.push(row);
        Ok(())
    })
    .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(rows, [1, 2, 3, 4]);
    actual.fill(203);
    let failure = Failure::new(ErrorCode::Callback, ErrorPath::OnProgress);
    assert_eq!(run(&mut actual, &mut |_| Err(failure)), Err(failure));
    assert_eq!(&actual[..row_bytes], &expected[..row_bytes]);
    assert!(actual[row_bytes..].iter().all(|&byte| byte == 203));
}

#[test]
fn indexed_hooks_report_finished_rows_and_abort_before_the_next() {
    let dimensions = ImageDimensions::new(3, 4).unwrap();
    let bytes: Vec<u8> = (0..48).map(|n| (n * 73 + 17) as u8).collect();
    let source = ImageView::<Rgba8>::packed(&bytes, dimensions).unwrap();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let prepared = PreparedQuantizer::try_new(
        &palette,
        AlphaPolicy::Preserve { threshold: 128.0 },
        MatchPolicy::SrgbEuclidean,
        1 << 20,
    )
    .unwrap();
    check_rows(3, |output, progress| {
        prepared.quantize_with_progress(source, output, progress)
    });
    check_rows(3, |output, progress| {
        dither_yiluoma_with_progress(
            source,
            &prepared,
            output,
            BayerSize::Four,
            Placement::Everywhere {},
            progress,
        )
    });
    for kernel in [
        Diffusion::FloydSteinberg,
        Diffusion::Sierra,
        Diffusion::SierraLite,
        Diffusion::Atkinson,
    ] {
        for serpentine in [false, true] {
            let policy = DiffusionPolicy::new(DitherPolicy::Diffusion {
                kernel,
                feedback: DiffusionFeedback::SrgbBytes,
                strength: 1.0,
                serpentine,
                placement: Placement::Everywhere {},
            })
            .unwrap();
            for cache_size in [0, 128] {
                let mut cache = vec![0; cache_size];
                check_rows(3, |output, progress| {
                    execute_with_progress(
                        &prepared,
                        &mut [[0.0; 3]; 9],
                        &mut cache,
                        source,
                        output,
                        policy,
                        progress,
                    )
                });
            }
        }
    }
}

#[test]
fn perturb_hooks_keep_full_source_neighbors_and_global_field_indices() {
    let dimensions = ImageDimensions::new(3, 4).unwrap();
    let bytes: Vec<u8> = (0..48).map(|n| (n * 73 + 17) as u8).collect();
    let source = ImageView::<Rgba8>::packed(&bytes, dimensions).unwrap();
    check_rows(12, |output, progress| {
        perturb_by_field_with_progress(
            source,
            ImageViewMut::packed(output, dimensions).unwrap(),
            WorkingSpace::Srgb,
            0.75,
            Placement::Adaptive {
                radius: 1,
                threshold: 0.1,
                softness: 0.2,
            },
            RowBand::new(0, 4).unwrap(),
            |x, y, index| {
                assert_eq!(index, u64::from(y * 3 + x));
                crate::prod::dither::random_noise::random_noise_at(7, index)
            },
            progress,
        )
    });
}

#[test]
fn prepared_resize_hooks_preserve_paths_bytes_counts_and_recovery() {
    use super::super::resize::PreparedResize;
    use crate::prod::contract::request::{Anchor, ResizePolicy, Support};
    let policies = [
        ResizePolicy::Nearest {
            anchor: Anchor::Center,
        },
        ResizePolicy::Area {},
        ResizePolicy::Bilinear {
            anchor: Anchor::Center,
        },
        ResizePolicy::Bicubic {
            anchor: Anchor::Center,
            support: Support::Fixed,
        },
        ResizePolicy::Lanczos2 {
            anchor: Anchor::Center,
            support: Support::ScaleAware,
        },
        ResizePolicy::Lanczos3 {
            anchor: Anchor::Center,
            support: Support::ScaleAware,
        },
        ResizePolicy::Trilinear {
            anchor: Anchor::Center,
        },
    ];
    for (sw, sh, ow, oh) in [
        (9, 7, 9, 7),
        (9, 7, 9, 4),
        (9, 7, 4, 7),
        (12, 8, 6, 4),
        (6, 4, 12, 8),
        (129, 117, 37, 43),
    ] {
        let source_dimensions = ImageDimensions::new(sw, sh).unwrap();
        let output_dimensions = ImageDimensions::new(ow, oh).unwrap();
        let source: Vec<u8> = (0..sw * sh * 4).map(|n| (n * 73 + 17) as u8).collect();
        let view = ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap();
        for policy in policies {
            let mut prepared =
                PreparedResize::new(source_dimensions, output_dimensions, policy, 1 << 24).unwrap();
            let capacity = prepared.capacity_bytes();
            let mut expected = vec![203; (ow * oh * 4) as usize];
            prepared
                .execute(
                    view,
                    ImageViewMut::packed(&mut expected, output_dimensions).unwrap(),
                )
                .unwrap();
            let mut actual = vec![203; expected.len()];
            let mut events = Vec::new();
            prepared
                .execute_with_progress(
                    view,
                    ImageViewMut::packed(&mut actual, output_dimensions).unwrap(),
                    &mut |completed, total| {
                        assert!(completed <= total);
                        events.push((completed, total));
                        Ok(())
                    },
                )
                .unwrap();
            assert_eq!(actual, expected, "{policy:?} {sw}x{sh}->{ow}x{oh}");
            let &(completed, total) = events.last().unwrap();
            assert_eq!(completed, total);
            assert!(events
                .windows(2)
                .all(|pair| pair[0].0 <= pair[1].0 && pair[0].1 == pair[1].1));
            if sw == 129
                && matches!(
                    policy,
                    ResizePolicy::Lanczos2 { .. } | ResizePolicy::Lanczos3 { .. }
                )
            {
                assert_eq!(total, sh + oh);
            }
            if sw == 129 && matches!(policy, ResizePolicy::Trilinear { .. }) {
                assert!(total > sh + oh * 2);
            }
            let failure = Failure::new(ErrorCode::Callback, ErrorPath::OnProgress);
            assert_eq!(
                prepared.execute_with_progress(
                    view,
                    ImageViewMut::packed(&mut actual, output_dimensions).unwrap(),
                    &mut |done, _| {
                        if done > 0 {
                            Err(failure)
                        } else {
                            Ok(())
                        }
                    }
                ),
                Err(failure)
            );
            prepared
                .execute(
                    view,
                    ImageViewMut::packed(&mut actual, output_dimensions).unwrap(),
                )
                .unwrap();
            assert_eq!(actual, expected);
            assert_eq!(prepared.capacity_bytes(), capacity);
        }
    }
}
