use super::*;
use crate::{
    image::{ImageView, ImageViewMut},
    prod::{
        contract::{
            failure::Failure,
            request::{Anchor, Output, Source, Support},
        },
        pipeline::processor::{Boundary, InputBoundary, Processor},
    },
};
use pollster::block_on;

struct InputBytes<'a>(&'a [u8]);

/// A whole-image trilinear reduction first owns a 2048x2048 float mip and
/// a 2048x4096 float horizontal intermediate beside three 64 MiB source copies.
#[test]
fn deep_trilinear_admission_covers_intermediate_mip_construction() {
    let source = ImageDimensions::new(4096, 4096).unwrap();
    let output = ImageDimensions::new(1, 1).unwrap();
    let policy = ResizePolicy::Trilinear {
        anchor: Anchor::Center,
    };
    let whole = ResizeMemory::new(source, output, policy, GpuTiling::Disabled).required_bytes;
    assert!(
        whole >= 384 * 1024 * 1024,
        "known live ownership is 384 MiB, plan={whole}"
    );
    assert!(
        whole > 360 * 1024 * 1024,
        "a 360 MiB request must reject before allocation"
    );
    let tiled = ResizeMemory::new(source, output, policy, GpuTiling::Auto).required_bytes;
    assert!(
        tiled < whole,
        "bounded mip tiles should retain their memory advantage"
    );
}
impl InputBoundary for InputBytes<'_> {
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.0.len())
    }
    fn copy_input(&mut self, dst: &mut [u8]) -> Result<(), Failure> {
        dst.copy_from_slice(self.0);
        Ok(())
    }
}
impl Boundary for InputBytes<'_> {
    type Output = Vec<u8>;
    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Self::Output, Failure> {
        Ok(bytes.to_vec())
    }
}

#[test]
fn every_resize_preserves_anchors_coverage_and_global_tile_halos() {
    let Some((_serial, Backend::Gpu(gpu))) = super::super::tests::any_gpu() else {
        return;
    };
    let mut backend = Backend::Gpu(gpu);
    // Proves all filters work through GPU buffers with hidden colour/coverage,
    // anchors, mip rounding and the same samples on both sides of a tile edge.
    let anchors = [
        Anchor::TopLeft,
        Anchor::Top,
        Anchor::TopRight,
        Anchor::Left,
        Anchor::Center,
        Anchor::Right,
        Anchor::BottomLeft,
        Anchor::Bottom,
        Anchor::BottomRight,
    ];
    let mut cases = 0;
    for (sw, sh, ow, oh) in [
        (37, 29, 19, 13),
        (17, 13, 41, 31),
        (1, 17, 9, 5),
        (19, 1, 3, 7),
        (2053, 5, 2051, 3),
    ] {
        let source = ImageDimensions::new(sw, sh).unwrap();
        let output = ImageDimensions::new(ow, oh).unwrap();
        let rgba = (0..sw * sh)
            .flat_map(|i| {
                [
                    i.wrapping_mul(41) as u8,
                    i.wrapping_mul(71) as u8,
                    i.wrapping_mul(13) as u8,
                    [0, 37, 127, 255][i as usize % 4],
                ]
            })
            .collect::<Vec<_>>();
        for anchor in anchors {
            let policies = [
                ResizePolicy::Nearest { anchor },
                ResizePolicy::Area {},
                ResizePolicy::Bilinear { anchor },
                ResizePolicy::Trilinear { anchor },
                ResizePolicy::Bicubic {
                    anchor,
                    support: Support::Fixed,
                },
                ResizePolicy::Bicubic {
                    anchor,
                    support: Support::ScaleAware,
                },
                ResizePolicy::Lanczos2 {
                    anchor,
                    support: Support::Fixed,
                },
                ResizePolicy::Lanczos2 {
                    anchor,
                    support: Support::ScaleAware,
                },
                ResizePolicy::Lanczos3 {
                    anchor,
                    support: Support::Fixed,
                },
                ResizePolicy::Lanczos3 {
                    anchor,
                    support: Support::ScaleAware,
                },
            ];
            for policy in policies {
                let request = ResizeRequest {
                    version: 1,
                    source: Source {
                        width: sw,
                        height: sh,
                        data: &rgba,
                    },
                    output: Output {
                        width: ow,
                        height: oh,
                        resize: policy,
                    },
                };
                for coverage in [false, true] {
                    let mut expected = vec![0; (ow * oh * 4) as usize];
                    if coverage && !matches!(policy, ResizePolicy::Nearest { .. }) {
                        crate::prod::coverage::resize(
                            ImageView::packed(&rgba, source).unwrap(),
                            ImageViewMut::packed(&mut expected, output).unwrap(),
                            policy,
                            512 * 1024 * 1024,
                            &mut |_, _| Ok(()),
                        )
                        .unwrap();
                    } else {
                        let mut cpu = Processor::new(512 * 1024 * 1024, 0).unwrap();
                        expected = cpu
                            .resize(
                                crate::prod::pipeline::processor::ResizeRequest {
                                    source_width: sw,
                                    source_height: sh,
                                    output: request.output,
                                },
                                &mut InputBytes(&rgba),
                            )
                            .unwrap();
                    }
                    let whole = block_on(backend.resize(
                        request,
                        coverage,
                        GpuTiling::Disabled,
                        512 * 1024 * 1024,
                    ))
                    .unwrap();
                    let tiled = block_on(backend.resize(
                        request,
                        coverage,
                        GpuTiling::Rows(3),
                        512 * 1024 * 1024,
                    ))
                    .unwrap();
                    assert_eq!(
                        whole, tiled,
                        "tile seams: {sw}x{sh}->{ow}x{oh}, {policy:?}, coverage {coverage}"
                    );
                    let max = whole
                        .iter()
                        .zip(&expected)
                        .map(|(&a, &b)| a.abs_diff(b))
                        .max()
                        .unwrap();
                    assert!(max<=2,"maximum byte error {max}: {sw}x{sh}->{ow}x{oh}, {policy:?}, coverage {coverage}");
                    if matches!(policy, ResizePolicy::Nearest { .. }) {
                        assert_eq!(whole, expected);
                    }
                    cases += 1;
                }
            }
        }
    }
    eprintln!("GPU resize: {cases} ordinary/coverage cases, all anchors, whole/tiled");
}

#[test]
fn resize_rejects_budget_and_cancellation_before_publishing_then_recovers() {
    // Proves rejected or cancelled work leaves no partial image and permits retry.
    let Some((_serial, Backend::Gpu(gpu))) = super::super::tests::any_gpu() else {
        return;
    };
    let mut backend = Backend::Gpu(gpu);
    let rgba = [
        255, 0, 0, 0, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 64,
    ];
    let request = ResizeRequest {
        version: 1,
        source: Source {
            width: 2,
            height: 2,
            data: &rgba,
        },
        output: Output {
            width: 7,
            height: 9,
            resize: ResizePolicy::Trilinear {
                anchor: Anchor::Center,
            },
        },
    };
    let source = ImageDimensions::new(2, 2).unwrap();
    let output = ImageDimensions::new(7, 9).unwrap();
    let required =
        ResizeMemory::new(source, output, request.output.resize, GpuTiling::Rows(2)).required_bytes;
    assert!(matches!(
        block_on(backend.resize(request, true, GpuTiling::Rows(2), required - 1)),
        Err(Fallback::MemoryLimit)
    ));
    let mut checks = 0;
    let result =
        block_on(
            backend.resize_checked(request, true, GpuTiling::Rows(2), required, &mut || {
                checks += 1;
                if checks >= 4 {
                    Err(Fallback::Cancelled)
                } else {
                    Ok(())
                }
            }),
        );
    assert!(matches!(result, Err(Fallback::Cancelled)));
    assert_eq!(
        block_on(backend.resize(request, true, GpuTiling::Rows(2), required))
            .unwrap()
            .len(),
        7 * 9 * 4
    );
    assert!(matches!(backend, Backend::Gpu(_)));
}
