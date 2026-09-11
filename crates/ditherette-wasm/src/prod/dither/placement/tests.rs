use super::*;
use crate::{
    image::{ImageDimensions, RowStride},
    spec,
};

const SPACES: [WorkingSpace; 7] = [
    WorkingSpace::Srgb,
    WorkingSpace::LinearRgb,
    WorkingSpace::Oklab,
    WorkingSpace::Oklch,
    WorkingSpace::Cielab,
    WorkingSpace::Cielch,
    WorkingSpace::Ycbcr,
];

#[test]
fn optional_rows_count_actual_capacity_and_preserve_low_budget_and_everywhere_fallbacks() {
    let adaptive = Placement::Adaptive {
        radius: 1,
        threshold: 5.0,
        softness: 10.0,
    };
    let full = AdaptivePlacementWork::try_new(7, adaptive, u64::MAX).unwrap();
    let minimum = full.capacity_bytes();
    assert_eq!(minimum, AdaptivePlacementWork::RECORD_BYTES + 7 * 36);
    assert!(AdaptivePlacementWork::try_new(7, adaptive, minimum - 1).is_none());
    assert_eq!(
        AdaptivePlacementWork::try_new(7, adaptive, minimum)
            .unwrap()
            .capacity_bytes(),
        minimum
    );
    assert!(AdaptivePlacementWork::try_new(7, Placement::Everywhere {}, u64::MAX).is_none());
}

#[test]
fn cached_fields_draw_once_per_pixel_and_cancel_after_the_completed_row() {
    use crate::{
        image::ImageViewMut,
        prod::{
            contract::{
                error::ErrorCode,
                failure::{ErrorPath, Failure},
            },
            dither::perturb::perturb_by_field_with_scratch,
            tiling::RowBand,
        },
    };
    use std::cell::Cell;
    let dimensions = ImageDimensions::new(7, 5).unwrap();
    let bytes: Vec<_> = (0..7 * 5 * 4).map(|n| (n * 73) as u8).collect();
    let source = ImageView::packed(&bytes, dimensions).unwrap();
    let cancelled = Failure::new(ErrorCode::Callback, ErrorPath::OnProgress);
    for space in SPACES {
        let mut expected = None;
        for scratch_len in [0, 7 * 3] {
            let mut scratch = vec![[f32::NAN; 3]; scratch_len];
            for cancel in [true, false] {
                let mut output = vec![213; bytes.len()];
                let draws = Cell::new(0);
                let mut completed = 0;
                let result = perturb_by_field_with_scratch(
                    source,
                    ImageViewMut::packed(&mut output, dimensions).unwrap(),
                    space,
                    0.75,
                    Placement::Adaptive {
                        radius: 2,
                        threshold: 5.0,
                        softness: 10.0,
                    },
                    RowBand::new(0, 5).unwrap(),
                    &mut scratch,
                    |x, y, index| {
                        assert_eq!(index, draws.get());
                        assert_eq!(index, u64::from(y * 7 + x));
                        draws.set(index + 1);
                        crate::prod::dither::random_noise::random_noise_at(71, index)
                    },
                    |row| {
                        completed = row;
                        if cancel && row == 2 {
                            Err(cancelled)
                        } else {
                            Ok(())
                        }
                    },
                );
                if cancel {
                    assert_eq!(result, Err(cancelled));
                    assert_eq!((completed, draws.get()), (2, 14));
                    assert!(output[7 * 2 * 4..].iter().all(|&value| value == 213));
                } else {
                    result.unwrap();
                    assert_eq!((completed, draws.get()), (5, 35));
                    assert_eq!(output, *expected.get_or_insert_with(|| output.clone()));
                }
            }
        }
    }
}

#[test]
fn cached_rows_preserve_frozen_contrast_and_masks_across_spaces_edges_and_scan_orders() {
    for (width, height) in [(1, 1), (1, 7), (7, 1), (3, 5), (8, 9)] {
        let dimensions = ImageDimensions::new(width, height).unwrap();
        let stride = width as usize * 4 + 7;
        let mut bytes = vec![213; stride * height as usize];
        for y in 0..height {
            for x in 0..width {
                let offset = y as usize * stride + x as usize * 4;
                bytes[offset..offset + 4].copy_from_slice(&[
                    (x * 73 + y * 17 + 11) as u8,
                    (x * 31 + y * 99 + 43) as u8,
                    (x * 117 + y * 41 + 19) as u8,
                    [0, 127, 128, 255][(x + y) as usize % 4],
                ]);
            }
        }
        let source = ImageView::new(&bytes, dimensions, RowStride::new(stride).unwrap()).unwrap();
        for space in SPACES {
            let converter = Converter::new(PackedSpace::from_working(space));
            let frozen_space =
                serde_json::from_value(serde_json::to_value(space).unwrap()).unwrap();
            for radius in [1, 2, height + 3, u32::MAX] {
                let mut scratch =
                    vec![[f32::NAN; 3]; width as usize * AdaptivePlacementRows::ROW_COUNT];
                let mut cache =
                    AdaptivePlacementRows::new(source, &converter, space, radius, &mut scratch);
                // A forward pass, a reverse pass, and jumps exercise both retention and replacement.
                for y in (0..height)
                    .chain((0..height).rev())
                    .chain([height - 1, 0, height / 2])
                {
                    let row = cache.prepare_row(y);
                    for x in (0..width).rev() {
                        let expected = spec::dither::placement::contrast_at(
                            source,
                            x,
                            y,
                            frozen_space,
                            radius,
                        );
                        let actual = row.contrast_at(x);
                        assert_eq!(
                            actual.to_bits(),
                            expected.to_bits(),
                            "{space:?}, {width}x{height}, r{radius}, ({x},{y})"
                        );
                        assert_eq!(
                            actual.to_bits(),
                            contrast_with_converter(source, x, y, space, radius, &converter)
                                .to_bits()
                        );
                        for (threshold, softness) in [
                            (0.0, 0.0),
                            (5.0, 10.0),
                            (100.0, 0.0),
                            (actual as f32, 0.0),
                            (actual as f32, 0.0001),
                            (f32::MAX, f32::MAX),
                        ] {
                            let placement = Placement::Adaptive {
                                radius,
                                threshold,
                                softness,
                            };
                            let frozen_placement =
                                serde_json::from_value(serde_json::to_value(placement).unwrap())
                                    .unwrap();
                            assert_eq!(
                                row.mask_at(x, threshold, softness).to_bits(),
                                spec::dither::placement::placement_mask_at(
                                    source,
                                    x,
                                    y,
                                    frozen_space,
                                    frozen_placement
                                )
                                .to_bits()
                            );
                            assert_eq!(
                                row.mask_at(x, threshold, softness).to_bits(),
                                placement_mask_with_converter(
                                    source, x, y, space, placement, &converter
                                )
                                .to_bits()
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn radius_one_retains_needed_slots_and_clamped_rows_share_one_conversion() {
    let dimensions = ImageDimensions::new(3, 5).unwrap();
    let bytes = vec![255; 3 * 5 * 4];
    let source = ImageView::packed(&bytes, dimensions).unwrap();
    let converter = Converter::new(PackedSpace::Srgb);
    let mut scratch = [[0.0; 3]; 9];
    let mut cache =
        AdaptivePlacementRows::new(source, &converter, WorkingSpace::Srgb, 1, &mut scratch);
    let row = cache.prepare_row(0);
    assert!(std::ptr::eq(row.above.as_ptr(), row.center.as_ptr()));
    assert_eq!(cache.tags.iter().flatten().count(), 2);
    for y in 1..5 {
        let tags = cache.tags;
        // Sentinels prove a retained row is not needlessly converted again.
        for (slot, tag) in tags.iter().enumerate() {
            if let Some(tag) = tag {
                cache.coordinates[slot * 3] = [*tag as f32 + 100.0; 3];
            }
        }
        cache.prepare_row(y);
        for (slot, tag) in tags.iter().enumerate() {
            if tag.is_some_and(|row| (y - 1..=(y + 1).min(4)).contains(&row)) {
                assert_eq!(cache.tags[slot], *tag);
                assert_eq!(
                    cache.coordinates[slot * 3],
                    [tag.unwrap() as f32 + 100.0; 3]
                );
            }
        }
    }
}

#[test]
fn rebinding_scratch_discards_prior_coordinates_and_ignores_source_alpha() {
    let dimensions = ImageDimensions::new(3, 2).unwrap();
    let converter = Converter::new(PackedSpace::Oklab);
    let mut scratch = [[f32::NAN; 3]; 9];
    let mut baseline = None;
    for alpha in [0, 127, 255] {
        let bytes: Vec<_> = (0..6)
            .flat_map(|i| [(i * 31) as u8, (i * 73) as u8, (i * 117) as u8, alpha])
            .collect();
        let source = ImageView::packed(&bytes, dimensions).unwrap();
        let mut cache =
            AdaptivePlacementRows::new(source, &converter, WorkingSpace::Oklab, 1, &mut scratch);
        let values: Vec<_> = (0..2)
            .flat_map(|y| {
                let row = cache.prepare_row(y);
                (0..3)
                    .map(|x| row.contrast_at(x).to_bits())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(values, *baseline.get_or_insert_with(|| values.clone()));
        scratch.fill([f32::NAN; 3]);
    }
}
