use super::*;

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).unwrap()
}

fn field(space: WorkingSpace) -> PerturbPolicy {
    PerturbPolicy {
        space,
        strength: 0.7,
        field: if space == WorkingSpace::Srgb {
            Field::Random { seed: 7 }
        } else {
            Field::BlueNoise {}
        },
        placement: if space == WorkingSpace::Srgb {
            Placement::Everywhere {}
        } else {
            Placement::Adaptive {
                radius: 2,
                threshold: 0.05,
                softness: 0.025,
            }
        },
    }
}

fn palette(count: usize) -> Vec<PaletteEntry> {
    let mut palette = vec![PaletteEntry::Color { rgb: [37, 59, 83] }; count - 1];
    palette.push(PaletteEntry::Transparent {});
    palette
}

#[test]
fn measured_field_classes_use_only_demonstrated_worker_counts() {
    for pool in [1, 2, 3, 4, 8] {
        let workers = WorkerBudget::new(pool);
        let srgb = measured_field(dimensions(769, 513), field(WorkingSpace::Srgb), workers);
        let oklab = measured_field(dimensions(65, 49), field(WorkingSpace::Oklab), workers);
        assert_eq!(
            srgb.map(|p| (p.height, p.active_workers)),
            match pool {
                1 => None,
                2 | 3 => Some((32, 2)),
                _ => Some((128, 4)),
            }
        );
        assert_eq!(
            oklab.map(|p| (p.height, p.active_workers)),
            (pool >= 2).then_some((32, 2))
        );
        for size in [dimensions(768, 513), dimensions(769, 512)] {
            assert_eq!(
                measured_field(size, field(WorkingSpace::Srgb), workers),
                None
            );
        }
        for size in [dimensions(64, 49), dimensions(65, 48)] {
            assert_eq!(
                measured_field(size, field(WorkingSpace::Oklab), workers),
                None
            );
        }
    }
    let workers = WorkerBudget::new(4);
    let mut policy = field(WorkingSpace::Oklab);
    policy.strength = 0.2;
    policy.placement = Placement::Adaptive {
        radius: 2,
        threshold: 8.0,
        softness: 4.0,
    };
    assert!(measured_field(dimensions(65, 49), policy, workers).is_some());
    policy.strength = 0.0;
    assert_eq!(measured_field(dimensions(65, 49), policy, workers), None);
    policy.strength = 0.7;
    policy.placement = Placement::Adaptive {
        radius: 3,
        threshold: 0.05,
        softness: 0.025,
    };
    assert_eq!(measured_field(dimensions(65, 49), policy, workers), None);
    policy = field(WorkingSpace::Srgb);
    policy.field = Field::Bayer {
        size: crate::prod::contract::request::BayerSize::Four,
    };
    assert_eq!(measured_field(dimensions(769, 513), policy, workers), None);
}

#[test]
fn indexed_classes_keep_unmeasured_matching_and_palette_work_scalar() {
    let workers = WorkerBudget::new(4);
    for (space, matching, count, size) in [
        (
            WorkingSpace::Srgb,
            MatchPolicy::SrgbEuclidean,
            16,
            dimensions(769, 513),
        ),
        (
            WorkingSpace::Oklab,
            MatchPolicy::OklabEuclidean,
            64,
            dimensions(65, 49),
        ),
    ] {
        let palette = palette(count);
        let request = QuantizeRequest {
            source_width: size.width(),
            source_height: size.height(),
            palette: &palette,
            alpha: AlphaPolicy::Preserve { threshold: 123.0 },
            matching,
        };
        let separable = DitherPolicy::Separable {
            perturb: field(space),
        };
        assert!(measured_indexed(size, request, separable, workers).is_some());
        assert_eq!(
            measured_indexed(size, request, DitherPolicy::None {}, workers).is_some(),
            space == WorkingSpace::Srgb
        );
        assert_eq!(
            measured_indexed(
                size,
                QuantizeRequest {
                    palette: &palette[..count - 1],
                    ..request
                },
                separable,
                workers
            ),
            None
        );
        assert_eq!(
            measured_indexed(
                size,
                QuantizeRequest {
                    alpha: AlphaPolicy::Premultiplied {},
                    ..request
                },
                separable,
                workers
            ),
            None
        );
        assert_eq!(
            measured_indexed(
                size,
                QuantizeRequest {
                    matching: MatchPolicy::SrgbRec709,
                    ..request
                },
                separable,
                workers
            ),
            None
        );
        assert_eq!(
            measured_indexed(
                size,
                request,
                DitherPolicy::Yliluoma {
                    size: crate::prod::contract::request::BayerSize::Two,
                    placement: Placement::Everywhere {}
                },
                workers
            ),
            None
        );
    }
}
