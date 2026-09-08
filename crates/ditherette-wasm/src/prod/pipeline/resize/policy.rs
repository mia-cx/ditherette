//! Conservative public-call regions measured in Chromium trial02 and Firefox trial03.
//! See `.plans/77-row-bands.md` for samples, exactness, and retained scalar classes.

use crate::{
    image::ImageDimensions,
    prod::{
        contract::request::{Anchor, ResizePolicy, Support},
        pipeline::execution::RowBandPolicy,
        tiling::WorkerBudget,
    },
};

const HALF_SCALE_MIN_OUTPUT: (u32, u32) = (769, 513);
const QUARTER_SCALE_MIN_OUTPUT: (u32, u32) = (512, 384);

/// Select only measured scale classes, anchors, band heights, and worker counts.
pub(crate) fn measured(
    source: ImageDimensions,
    output: ImageDimensions,
    resize: ResizePolicy,
    workers: WorkerBudget,
) -> Option<RowBandPolicy> {
    let eligible = match resize {
        ResizePolicy::Area {}
        | ResizePolicy::Bilinear {
            anchor: Anchor::Center,
        } => {
            output.width() >= HALF_SCALE_MIN_OUTPUT.0
                && output.height() >= HALF_SCALE_MIN_OUTPUT.1
                && source.width() as u64 + 1 == output.width() as u64 * 2
                && source.height() as u64 + 1 == output.height() as u64 * 2
        }
        ResizePolicy::Lanczos3 {
            anchor: Anchor::Center,
            support: Support::ScaleAware,
        } => {
            output.width() >= QUARTER_SCALE_MIN_OUTPUT.0
                && output.height() >= QUARTER_SCALE_MIN_OUTPUT.1
                && source.width() as u64 == output.width() as u64 * 4
                && source.height() as u64 == output.height() as u64 * 4
        }
        _ => false,
    };
    if !eligible || workers.pool_size() < 2 {
        return None;
    }
    let (active_workers, height) = if workers.pool_size() >= 4 {
        (4, 128)
    } else {
        (2, 32)
    };
    Some(RowBandPolicy {
        height,
        workers,
        active_workers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dims(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }

    #[test]
    fn measured_regions_preserve_worker_counts_and_reject_unmeasured_shapes() {
        let filters = [
            (ResizePolicy::Area {}, (769, 513), 2),
            (
                ResizePolicy::Bilinear {
                    anchor: Anchor::Center,
                },
                (769, 513),
                2,
            ),
            (
                ResizePolicy::Lanczos3 {
                    anchor: Anchor::Center,
                    support: Support::ScaleAware,
                },
                (512, 384),
                4,
            ),
        ];
        for (filter, (width, height), scale) in filters {
            let edge = u32::from(scale == 2);
            let source = dims(width * scale - edge, height * scale - edge);
            let output = dims(width, height);
            for pool in 1..=8 {
                let selected = measured(source, output, filter, WorkerBudget::new(pool));
                let expected = match pool {
                    1 => None,
                    2 | 3 => Some((2, 32)),
                    _ => Some((4, 128)),
                };
                assert_eq!(
                    selected.map(|band| (band.active_workers, band.height)),
                    expected
                );
            }
            let pool = WorkerBudget::new(4);
            for (w, h) in [(width - 1, height), (width, height - 1)] {
                assert_eq!(
                    measured(
                        dims(w * scale - edge, h * scale - edge),
                        dims(w, h),
                        filter,
                        pool
                    ),
                    None
                );
            }
            assert_eq!(measured(output, source, filter, pool), None);
            assert_eq!(measured(output, output, filter, pool), None);
            assert_eq!(
                measured(
                    dims(source.width() + 1, source.height()),
                    output,
                    filter,
                    pool
                ),
                None
            );
        }
    }

    #[test]
    fn unmeasured_filters_anchors_and_support_stay_scalar() {
        for resize in [
            ResizePolicy::Nearest {
                anchor: Anchor::Center,
            },
            ResizePolicy::Bilinear {
                anchor: Anchor::TopLeft,
            },
            ResizePolicy::Bicubic {
                anchor: Anchor::Center,
                support: Support::ScaleAware,
            },
            ResizePolicy::Lanczos2 {
                anchor: Anchor::Center,
                support: Support::ScaleAware,
            },
            ResizePolicy::Lanczos3 {
                anchor: Anchor::Center,
                support: Support::Fixed,
            },
            ResizePolicy::Lanczos3 {
                anchor: Anchor::BottomRight,
                support: Support::ScaleAware,
            },
            ResizePolicy::Trilinear {
                anchor: Anchor::Center,
            },
        ] {
            assert_eq!(
                measured(
                    dims(2048, 1536),
                    dims(512, 384),
                    resize,
                    WorkerBudget::new(4)
                ),
                None
            );
        }
    }
}
