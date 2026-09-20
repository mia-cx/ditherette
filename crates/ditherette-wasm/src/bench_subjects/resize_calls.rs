//! Cold complete native resize calls, including source snapshot, hashing, preparation and output copy.
//! Browser subjects must also include the actual installed package boundary in a host worker.

use super::*;
use crate::prod::{
    contract::{
        failure::Failure,
        request::{Anchor, Output, ResizePolicy, Support, MAX_MEMORY_LIMIT_BYTES},
    },
    pipeline::{
        execution::{ExecutionPolicy, RowBandPolicy},
        processor::{Boundary, Processor, ResizeRequest},
    },
    tiling::{WorkerBudget, MAX_WORKER_BUDGET},
};

pub(super) fn subjects() -> Vec<BenchSubject> {
    let entries: [(&str, &str, ResizeU8RgbaFn); 9] = [
        ("nearest", "nearest:scalar", |i, o, p| {
            run(i, o, p, |anchor| ResizePolicy::Nearest { anchor })
        }),
        ("area", "area:scalar", |i, o, p| {
            run(i, o, p, |_| ResizePolicy::Area {})
        }),
        ("bilinear", "bilinear:scalar", |i, o, p| {
            run(i, o, p, |anchor| ResizePolicy::Bilinear { anchor })
        }),
        ("bicubic-fixed", "bicubic:catmull-rom", |i, o, p| {
            run(i, o, p, |anchor| ResizePolicy::Bicubic {
                anchor,
                support: Support::Fixed,
            })
        }),
        (
            "bicubic-scale-aware",
            "bicubic:catmull-rom-scale-aware",
            |i, o, p| {
                run(i, o, p, |anchor| ResizePolicy::Bicubic {
                    anchor,
                    support: Support::ScaleAware,
                })
            },
        ),
        ("lanczos2-fixed", "lanczos2:fixed", |i, o, p| {
            run(i, o, p, |anchor| ResizePolicy::Lanczos2 {
                anchor,
                support: Support::Fixed,
            })
        }),
        ("lanczos2-scale-aware", "lanczos2:scale-aware", |i, o, p| {
            run(i, o, p, |anchor| ResizePolicy::Lanczos2 {
                anchor,
                support: Support::ScaleAware,
            })
        }),
        ("lanczos3-fixed", "lanczos3:fixed", |i, o, p| {
            run(i, o, p, |anchor| ResizePolicy::Lanczos3 {
                anchor,
                support: Support::Fixed,
            })
        }),
        ("lanczos3-scale-aware", "lanczos3:scale-aware", |i, o, p| {
            run(i, o, p, |anchor| ResizePolicy::Lanczos3 {
                anchor,
                support: Support::ScaleAware,
            })
        }),
    ];
    entries
        .into_iter()
        .map(|(id, oracle, call)| {
            let mut subject = resize_subject_with_oracle(
                &format!("candidate:resize:{id}:complete-call"),
                &format!("cold complete {id} call with optional budgeted row bands"),
                "crates/ditherette-wasm/src/bench_subjects/resize_calls.rs",
                call,
                Some(&format!("prod:resize:{oracle}")),
            );
            let BenchSubject::Resize(ref mut resize) = subject else {
                unreachable!()
            };
            resize.descriptor.capabilities.supports_tiling_params = true;
            subject
        })
        .collect()
}

fn run(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
    filter: impl FnOnce(Anchor) -> ResizePolicy,
) -> Result<(), BenchSubjectError> {
    use ditherette_bench_api::ResizeAnchorParam as A;
    let anchor = match params.anchor {
        A::TopLeft => Anchor::TopLeft,
        A::Top => Anchor::Top,
        A::TopRight => Anchor::TopRight,
        A::Left => Anchor::Left,
        A::Center => Anchor::Center,
        A::Right => Anchor::Right,
        A::BottomLeft => Anchor::BottomLeft,
        A::Bottom => Anchor::Bottom,
        A::BottomRight => Anchor::BottomRight,
    };
    let resize = filter(anchor);
    let band = params
        .tile_policy
        .as_ref()
        .map(|tile| {
            let height = tile.tile_height.ok_or_else(|| {
                BenchSubjectError::new("complete-call row bands require tile height")
            })?;
            let active_workers = tile.worker_count.ok_or_else(|| {
                BenchSubjectError::new("complete-call row bands require worker count")
            })?;
            if height == 0 || active_workers == 0 || active_workers > MAX_WORKER_BUDGET {
                return Err(BenchSubjectError::new(
                    "complete-call row bands require positive height and 1..8 workers",
                ));
            }
            Ok(RowBandPolicy {
                height,
                workers: WorkerBudget::new(active_workers),
                active_workers,
            })
        })
        .transpose()?;
    with_views(input, output, |source, output| -> Result<(), Failure> {
        let source_dimensions = source.dimensions();
        let output_dimensions = output.dimensions();
        let request = ResizeRequest {
            source_width: source_dimensions.width(),
            source_height: source_dimensions.height(),
            output: Output {
                width: output_dimensions.width(),
                height: output_dimensions.height(),
                resize,
            },
        };
        let mut processor = Processor::new(MAX_MEMORY_LIMIT_BYTES, 0)?;
        processor.set_execution_policy(ExecutionPolicy {
            resize: band,
            ..Default::default()
        })?;
        processor.resize(request, &mut Io { source, output })
    })?
    .map_err(|failure| BenchSubjectError::new(format!("{failure:?}")))
}

struct Io<'a, 'b> {
    source: ImageView<'a, Rgba8>,
    output: ImageViewMut<'b, Rgba8>,
}

impl Boundary for Io<'_, '_> {
    type Output = ();
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.source.dimensions().storage_len::<Rgba8>().unwrap())
    }
    fn copy_input(&mut self, output: &mut [u8]) -> Result<(), Failure> {
        let row = self.source.dimensions().width() as usize * 4;
        for (y, destination) in output.chunks_exact_mut(row).enumerate() {
            destination.copy_from_slice(self.source.row(y as u32).unwrap());
        }
        Ok(())
    }
    fn snapshot_input(&mut self, output: &mut [u8], compare: bool) -> Result<bool, Failure> {
        let row = self.source.dimensions().width() as usize * 4;
        if compare
            && output
                .chunks_exact(row)
                .enumerate()
                .all(|(y, bytes)| bytes == self.source.row(y as u32).unwrap())
        {
            return Ok(true);
        }
        self.copy_input(output)?;
        Ok(false)
    }
    fn complete(&mut self, bytes: &[u8], dimensions: ImageDimensions) -> Result<(), Failure> {
        for (y, source) in bytes
            .chunks_exact(dimensions.width() as usize * 4)
            .enumerate()
        {
            self.output
                .row_mut(y as u32)
                .unwrap()
                .copy_from_slice(source);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_call_subjects_exercise_scalar_and_budgeted_paths() {
        let source: Vec<_> = (0..71 * 53 * 4).map(|n| (n * 73) as u8).collect();
        for subject in subjects() {
            let BenchSubject::Resize(subject) = subject else {
                unreachable!()
            };
            let mut expected = vec![0; 19 * 17 * 4];
            let mut candidate = expected.clone();
            let input = ResizeInputU8Rgba {
                data: &source,
                width: 71,
                height: 53,
                row_stride_elements: 71 * 4,
            };
            let call = subject.resize_u8_rgba;
            call(
                input,
                ResizeOutputU8Rgba {
                    data: &mut expected,
                    width: 19,
                    height: 17,
                    row_stride_elements: 19 * 4,
                },
                &ResizeParams::default(),
            )
            .unwrap();
            call(
                input,
                ResizeOutputU8Rgba {
                    data: &mut candidate,
                    width: 19,
                    height: 17,
                    row_stride_elements: 19 * 4,
                },
                &ResizeParams {
                    tile_policy: Some(ditherette_bench_api::TilePolicyParam {
                        tile_height: Some(3),
                        worker_count: Some(4),
                        chunking_strategy: None,
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(candidate, expected, "{}", subject.descriptor.id);
        }
    }
}
