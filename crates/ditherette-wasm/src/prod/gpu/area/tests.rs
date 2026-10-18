use pollster::block_on;

use super::*;
use crate::image::contracts::{IndexedImage, PaletteEntry};
use crate::prod::contract::request::*;
use crate::{image::ImageView, prod::dither::smooth_area_v1};

struct CpuInput<'a>(&'a [u8]);

/// Proves row bands preserve global field/mixture phase, odd packed words and
/// Adaptive coverage. The thin long image also forces cancellation checkpoints
/// between real tiled submissions rather than accepting an ignored policy.
#[test]
fn indexed_row_tiles_preserve_pixels_and_bound_cancellation() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        return;
    };
    let palette = [
        PaletteEntry::Color { rgb: [0, 0, 0] },
        PaletteEntry::Color {
            rgb: [255, 255, 255],
        },
        PaletteEntry::Color {
            rgb: [90, 120, 100],
        },
        PaletteEntry::Transparent {},
    ];
    for (width, height) in [(37, 19), (1, 67), (67, 1), (3, 521)] {
        let source: Vec<_> = (0..width * height)
            .flat_map(|i| {
                [
                    (i * 71) as u8,
                    (i * 31) as u8,
                    (i * 23) as u8,
                    (i * 97) as u8,
                ]
            })
            .collect();
        let quantize = QuantizeRequest {
            version: 1,
            source: Source {
                width,
                height,
                data: &source,
            },
            palette: &palette,
            alpha: AlphaPolicy::Preserve { threshold: 127.9 },
            matching: MatchPolicy::OklchHueArc,
        };
        for placement in [
            Placement::Everywhere {},
            Placement::SmoothAreaV1 {
                sample_size: 20,
                contrast_threshold: 0.12,
            },
        ] {
            let mut dithers = vec![
                DitherPolicy::None {},
                DitherPolicy::Yliluoma {
                    size: BayerSize::Four,
                    placement,
                },
            ];
            for field in [
                Field::Bayer {
                    size: BayerSize::Sixteen,
                },
                Field::Random { seed: 0xfffffff1 },
                Field::BlueNoise {},
                Field::Ordered {
                    tile: OrderedTile::FiveByThree,
                },
            ] {
                dithers.push(DitherPolicy::Separable {
                    perturb: PerturbPolicy {
                        field,
                        space: WorkingSpace::Oklab,
                        strength: 0.3,
                        placement,
                    },
                });
            }
            for dither in dithers {
                let request = DitherQuantizeRequest { quantize, dither };
                let mut whole_checks = 0;
                let (expected, _) = block_on(backend.dither_tiled_checked(
                    request,
                    u64::MAX,
                    None,
                    false,
                    super::super::GpuTiling::Disabled,
                    &mut || {
                        whole_checks += 1;
                        Ok(())
                    },
                ))
                .unwrap();
                for tiling in [
                    super::super::GpuTiling::Auto,
                    super::super::GpuTiling::Rows(1),
                    super::super::GpuTiling::Rows(3),
                ] {
                    let mut checks = 0;
                    let (actual, _) = block_on(backend.dither_tiled_checked(
                        request,
                        u64::MAX,
                        None,
                        false,
                        tiling,
                        &mut || {
                            checks += 1;
                            Ok(())
                        },
                    ))
                    .unwrap();
                    assert_eq!(actual, expected, "{width}x{height}, {tiling:?}, {dither:?}");
                    if width == 3
                        && height == 521
                        && matches!(dither, DitherPolicy::None {})
                        && matches!(tiling, super::super::GpuTiling::Rows(1))
                    {
                        assert!(
                            checks > whole_checks,
                            "row bands must create real bounded submissions"
                        );
                    }
                }
            }
        }
    }
}
impl crate::prod::pipeline::processor::InputBoundary for CpuInput<'_> {
    fn input_len(&mut self) -> Result<usize, crate::prod::contract::failure::Failure> {
        Ok(self.0.len())
    }
    fn copy_input(
        &mut self,
        target: &mut [u8],
    ) -> Result<(), crate::prod::contract::failure::Failure> {
        target.copy_from_slice(self.0);
        Ok(())
    }
}
impl crate::prod::pipeline::quantize::QuantizeBoundary for CpuInput<'_> {
    type Output = IndexedImage;
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        metadata: crate::prod::pipeline::quantize::IndexedMetadataRef<'_>,
    ) -> Result<Self::Output, crate::prod::contract::failure::Failure> {
        Ok(IndexedImage {
            indices: crate::image::ImageBuf::from_vec_packed(indices.to_vec(), dimensions).unwrap(),
            palette: metadata.palette.clone(),
            warnings: metadata.warnings.to_vec(),
        })
    }
}
fn cpu(request: DitherQuantizeRequest<'_>) -> IndexedImage {
    crate::prod::pipeline::processor::Processor::new(128 * 1024 * 1024, 0)
        .unwrap()
        .dither_and_quantize(
            crate::prod::pipeline::quantize::QuantizeRequest {
                source_width: request.quantize.source.width,
                source_height: request.quantize.source.height,
                palette: request.quantize.palette,
                alpha: request.quantize.alpha,
                matching: request.quantize.matching,
            },
            request.dither,
            &mut CpuInput(request.quantize.source.data),
        )
        .unwrap()
}

fn upload(job: &AreaJob<'_>, cells: &[[f32; 3]]) {
    let bytes: Vec<_> = cells
        .iter()
        .flatten()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    job.gpu.queue.write_buffer(&job.cells, 0, &bytes);
}

#[test]
fn retained_components_cross_tiles_without_losing_thin_connections() {
    let Some((_serial, super::super::Backend::Gpu(gpu))) = super::super::tests::any_gpu() else {
        return;
    };
    // Proves complete eight-connected retention, including diagonal-only joins,
    // small islands and a path much longer than a workgroup edge.
    for (width, height) in [(79, 67), (1, 193), (193, 1)] {
        let dimensions = ImageDimensions::new(width, height).unwrap();
        let pixels = width as usize * height as usize;
        let mut cells = vec![[0.0, 1.0, 0.0]; pixels];
        for y in 0..height {
            for x in 0..width {
                if x == y % width || y % 17 == 0 || (x + y) % 11 == 0 {
                    cells[(y * width + x) as usize][0] = 1.0;
                }
            }
        }
        let mut expected = cells.clone();
        smooth_area_v1::retain_regions(dimensions, 8, 64.0, &mut expected, &mut || Ok::<_, ()>(()))
            .unwrap();
        let mut job = AreaJob::new(&gpu, dimensions, &vec![0; pixels * 4]).unwrap();
        upload(&job, &cells);
        let mut parameters = Parameters::new(dimensions, 32, 0.12);
        parameters.minimum = 8;
        job.retain(parameters);
        let actual = block_on(job.execute()).unwrap();
        for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
            assert_eq!(a[2], b[2], "component pixel {i}, {width}x{height}");
        }
    }
}

#[test]
fn skew_tiles_keep_all_four_reconstruction_orders() {
    let Some((_serial, super::super::Backend::Gpu(gpu))) = super::super::tests::any_gpu() else {
        return;
    };
    // Proves the tile DAG preserves the frozen predecessor order, especially
    // upper-right dependencies that make ordinary rectangular tiles cyclic.
    for (width, height) in [(67, 53), (1, 79), (79, 1), (17, 17)] {
        let dimensions = ImageDimensions::new(width, height).unwrap();
        let pixels = width as usize * height as usize;
        let source = vec![128; pixels * 4];
        for mode in [0, 1] {
            let mut cells = vec![[0.0, 1.0, 128.0]; pixels];
            for (i, cell) in cells.iter_mut().enumerate() {
                if i % 31 == 0 {
                    cell[2] = 0.0;
                } else if mode == 1 && i % 47 == 0 {
                    cell[2] = -1.0;
                }
            }
            let mut expected = cells.clone();
            let mut job = AreaJob::new(&gpu, dimensions, &source).unwrap();
            upload(&job, &cells);
            let mut parameters = Parameters::new(dimensions, 64, 0.12);
            parameters.mode = mode;
            job.pixel_pass(ANALYSIS, "colors", parameters);
            let directions = if mode == 0 {
                vec![(false, false), (true, true)]
            } else {
                vec![(false, false), (true, true), (false, true), (true, false)]
            };
            if mode == 0 {
                smooth_area_v1::geometric_distance(dimensions, &mut expected, &mut || {
                    Ok::<_, ()>(())
                })
                .unwrap();
            }
            let converter = crate::prod::color::packed::Converter::new(
                crate::prod::color::packed::PackedSpace::Oklab,
            );
            let mut rows = vec![[0.0; 3]; 3 * width as usize];
            for (reverse_y, reverse_x) in directions {
                if mode == 1 {
                    smooth_area_v1::spread(
                        ImageView::packed(&source, dimensions).unwrap(),
                        0.12,
                        64.0,
                        (reverse_y, reverse_x),
                        &mut expected,
                        &mut rows,
                        &mut |rgb| converter.coordinates(rgb),
                        &mut || Ok::<_, ()>(()),
                    )
                    .unwrap();
                }
                job.sweep(parameters, reverse_x, reverse_y);
            }
            let actual = block_on(job.execute()).unwrap();
            for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
                assert_eq!(
                    a[2].to_bits(),
                    b[2].to_bits(),
                    "sweep pixel {i}, {width}x{height}, mode {mode}"
                );
            }
        }
    }
}

#[test]
fn complete_adaptive_analysis_preserves_ramps_texture_and_lineart() {
    let Some((_serial, super::super::Backend::Gpu(gpu))) = super::super::tests::any_gpu() else {
        return;
    };
    eprintln!("Adaptive adapter: {:?}", gpu.info());
    // Proves the complete GPU selector keeps smooth regions and rejects sharp
    // lineart/texture, without a CPU selector or intermediate readback.
    let dimensions = ImageDimensions::new(137, 113).unwrap();
    let mut source = Vec::new();
    for y in 0..113u32 {
        for x in 0..137u32 {
            let shade = 64 + x / 2 + y / 8;
            let c = if x == 80 || x == 81 {
                [4, 8, 12]
            } else if y > 80 && x < 60 {
                if (x + y) % 2 == 0 {
                    [180, 90, 50]
                } else {
                    [40, 130, 70]
                }
            } else {
                [shade as u8, (shade + 30) as u8, (shade + 10) as u8]
            };
            source.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    for sample_size in [4, 20, 64, 100] {
        let mut expected = vec![[0.0; 3]; smooth_area_v1::scratch_len(dimensions)];
        smooth_area_v1::select(
            ImageView::packed(&source, dimensions).unwrap(),
            sample_size,
            0.12,
            &mut expected,
            || Ok::<_, ()>(()),
        )
        .unwrap();
        let mut job = AreaJob::new(&gpu, dimensions, &source).unwrap();
        job.analysis(Parameters::new(dimensions, sample_size, 0.12));
        let actual = block_on(job.execute()).unwrap();
        let mut maximum = 0.0_f32;
        let mut disagreements = 0;
        for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
            maximum = maximum.max((a[2] - b[2]).abs());
            disagreements += usize::from(
                smooth_area_v1::covers(a[2], i as u32 % 137, i as u32 / 137)
                    != smooth_area_v1::covers(b[2], i as u32 % 137, i as u32 / 137),
            );
        }
        eprintln!("sample {sample_size}: density error {maximum}, coverage disagreements {disagreements}/{}", actual.len());
        assert!(maximum < 0.001, "density error {maximum}");
        assert!(disagreements <= 8, "coverage disagreement {disagreements}");
    }
}

#[test]
fn packed_coverage_admits_before_allocation_and_owns_partial_words() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        return;
    };
    // Proves the public GPU boundary rejects an insufficient budget, remains
    // usable, and returns exactly the reference coverage with zero padding.
    let dimensions = ImageDimensions::new(37, 19).unwrap();
    let source: Vec<u8> = (0..37 * 19)
        .flat_map(|i| [80 + (i % 37) as u8, 110, 140, 255])
        .collect();
    let required = AreaMemory::new(dimensions).required_bytes;
    assert_eq!(
        block_on(backend.adaptive_coverage(dimensions, &source, 20, 0.12, required - 1)),
        Err(Fallback::MemoryLimit)
    );
    let output =
        block_on(backend.adaptive_coverage(dimensions, &source, 20, 0.12, required)).unwrap();
    let mut reference = vec![[0.0; 3]; smooth_area_v1::scratch_len(dimensions)];
    smooth_area_v1::select(
        ImageView::packed(&source, dimensions).unwrap(),
        20,
        0.12,
        &mut reference,
        || Ok::<_, ()>(()),
    )
    .unwrap();
    for i in 0..output.len() * 8 {
        let actual = output[i / 8] & (1 << (i % 8)) != 0;
        let expected =
            i < 37 * 19 && smooth_area_v1::covers(reference[i][2], i as u32 % 37, i as u32 / 37);
        assert_eq!(actual, expected, "coverage bit {i}");
    }
}

#[test]
fn indexed_gpu_fields_keep_palette_alpha_ties_and_all_matching_policies() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        return;
    };
    // Proves callers receive owned palette indices for all policies, with the
    // ordinary byte boundary, alpha rules, global field phase and first ties.
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color { rgb: [180, 30, 90] },
        PaletteEntry::Color { rgb: [20, 170, 80] },
        PaletteEntry::Color { rgb: [30, 70, 210] },
        PaletteEntry::Color { rgb: [180, 30, 90] },
        PaletteEntry::Transparent {},
    ];
    let data: Vec<_> = (0..23 * 17u32)
        .flat_map(|i| {
            [
                (i * 73 + 17) as u8,
                (i * 31 + 99) as u8,
                (i * 117 + 41) as u8,
                [0, 127, 128, 255][i as usize % 4],
            ]
        })
        .collect();
    let matching = [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
        MatchPolicy::YcbcrEuclidean,
    ];
    let fields = [
        Field::Bayer {
            size: BayerSize::Sixteen,
        },
        Field::BlueNoise {},
        Field::Random { seed: 0xfffffff1 },
        Field::Ordered {
            tile: OrderedTile::FiveByThree,
        },
    ];
    let mut comparisons = 0;
    let mut differences = 0;
    for matching in matching {
        for field in fields {
            for strength in [0.3, 96.0 / 63.75, 4.0] {
                for alpha in [
                    AlphaPolicy::Preserve { threshold: 127.9 },
                    AlphaPolicy::Premultiplied {},
                    AlphaPolicy::Matte { rgb: [7, 200, 90] },
                ] {
                    let request = DitherQuantizeRequest {
                        quantize: QuantizeRequest {
                            version: 1,
                            source: Source {
                                width: 23,
                                height: 17,
                                data: &data,
                            },
                            palette: &palette,
                            alpha,
                            matching,
                        },
                        dither: DitherPolicy::Separable {
                            perturb: PerturbPolicy {
                                field,
                                space: matching.space(),
                                strength,
                                placement: Placement::Everywhere {},
                            },
                        },
                    };
                    let expected = cpu(request);
                    let actual =
                        block_on(backend.dither_and_quantize(request, 128 * 1024 * 1024)).unwrap();
                    assert_eq!(actual.palette, expected.palette);
                    assert_eq!(actual.warnings, expected.warnings);
                    let changed = actual
                        .indices
                        .data()
                        .iter()
                        .zip(expected.indices.data())
                        .filter(|(a, b)| a != b)
                        .count();
                    assert!(
                        changed <= 4,
                        "{matching:?}, {field:?}, {alpha:?}: {changed}/391 differing indices"
                    );
                    assert!(actual
                        .indices
                        .data()
                        .iter()
                        .all(|&i| (i as usize) < palette.len() && i != 5));
                    differences += changed;
                    comparisons += 1;
                }
            }
        }
    }
    eprintln!(
        "indexed parity: {differences} differing indices / {} pixels across {comparisons} cases",
        comparisons * 391
    );
}

#[test]
fn gpu_masks_preserve_every_diffusion_kernel_and_scan_order() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        return;
    };
    // Proves moving analysis to GPU does not restart or approximate diffusion.
    let kernels = [
        Diffusion::FloydSteinberg,
        Diffusion::Sierra,
        Diffusion::SierraLite,
        Diffusion::Atkinson,
        Diffusion::JarvisJudiceNinke,
        Diffusion::Stucki,
        Diffusion::Burkes,
        Diffusion::TwoRowSierra,
        Diffusion::Fan,
        Diffusion::ShiauFan,
        Diffusion::ShiauFan2,
        Diffusion::Simple2d,
    ];
    let source: Vec<_> = (0..53 * 37u32)
        .flat_map(|i| {
            let x = i % 53;
            let y = i / 53;
            let shade = if x == 28 { 5 } else { (64 + x + y / 3) as u8 };
            [shade, shade.saturating_add(25), 110, 255]
        })
        .collect();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color {
            rgb: [90, 120, 100],
        },
        PaletteEntry::Color {
            rgb: [160, 90, 120],
        },
    ];
    for kernel in kernels {
        for feedback in [DiffusionFeedback::Matching, DiffusionFeedback::SrgbBytes] {
            for serpentine in [false, true] {
                let request = DitherQuantizeRequest {
                    quantize: QuantizeRequest {
                        version: 1,
                        source: Source {
                            width: 53,
                            height: 37,
                            data: &source,
                        },
                        palette: &palette,
                        matching: MatchPolicy::OklabEuclidean,
                        alpha: AlphaPolicy::Preserve { threshold: 127.0 },
                    },
                    dither: DitherPolicy::Diffusion {
                        kernel,
                        feedback,
                        serpentine,
                        strength: 1.0,
                        placement: Placement::SmoothAreaV1 {
                            sample_size: 20,
                            contrast_threshold: 0.12,
                        },
                    },
                };
                assert_eq!(
                    block_on(backend.dither_and_quantize(request, 128 * 1024 * 1024)).unwrap(),
                    cpu(request),
                    "{kernel:?}, {feedback:?}, serpentine {serpentine}"
                );
            }
        }
    }
}

#[test]
fn continuous_gpu_diffusion_preserves_kernels_feedback_and_serpentine() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        return;
    };
    // Proves GPU recurrence carries through scan turns, transparent targets and
    // output-only masks, using every matching policy and authoritative CPU taps.
    let source: Vec<_> = (0..17 * 13u32)
        .flat_map(|i| {
            let x = i % 17;
            let y = i / 17;
            let c = if x == 9 {
                5
            } else {
                (64 + x * 5 + y / 3) as u8
            };
            [
                c,
                c.saturating_add(25),
                110,
                [0, 127, 128, 255][i as usize % 4],
            ]
        })
        .collect();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color {
            rgb: [90, 120, 100],
        },
        PaletteEntry::Color {
            rgb: [160, 90, 120],
        },
        PaletteEntry::Color {
            rgb: [90, 120, 100],
        },
    ];
    let kernels = [
        Diffusion::FloydSteinberg,
        Diffusion::Sierra,
        Diffusion::SierraLite,
        Diffusion::Atkinson,
        Diffusion::JarvisJudiceNinke,
        Diffusion::Stucki,
        Diffusion::Burkes,
        Diffusion::TwoRowSierra,
        Diffusion::Fan,
        Diffusion::ShiauFan,
        Diffusion::ShiauFan2,
        Diffusion::Simple2d,
    ];
    let policies = [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
        MatchPolicy::YcbcrEuclidean,
    ];
    let mut comparisons = 0;
    let mut differences = 0;
    for matching in policies {
        for kernel in kernels {
            if !matches!(
                matching,
                MatchPolicy::SrgbEuclidean | MatchPolicy::OklabEuclidean
            ) && kernel != Diffusion::Sierra
            {
                continue;
            }
            for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
                for serpentine in [false, true] {
                    for placement in [
                        Placement::Everywhere {},
                        Placement::SmoothAreaV1 {
                            sample_size: 4,
                            contrast_threshold: 0.12,
                        },
                    ] {
                        let request = DitherQuantizeRequest {
                            quantize: QuantizeRequest {
                                version: 1,
                                source: Source {
                                    width: 17,
                                    height: 13,
                                    data: &source,
                                },
                                palette: &palette,
                                alpha: AlphaPolicy::Preserve { threshold: 127.0 },
                                matching,
                            },
                            dither: DitherPolicy::Diffusion {
                                kernel,
                                feedback,
                                serpentine,
                                strength: 1.0,
                                placement,
                            },
                        };
                        let expected = cpu(request);
                        let actual =
                            block_on(backend.diffuse_gpu(request, 128 * 1024 * 1024)).unwrap();
                        assert_eq!(actual.palette, expected.palette);
                        assert_eq!(actual.warnings, expected.warnings);
                        let changed = actual
                            .indices
                            .data()
                            .iter()
                            .zip(expected.indices.data())
                            .filter(|(a, b)| a != b)
                            .count();
                        assert!(changed<=22,"{matching:?}/{kernel:?}/{feedback:?}/{serpentine}/{placement:?}: {changed}/221");
                        assert!(actual
                            .indices
                            .data()
                            .iter()
                            .all(|&i| i != 5 && (i as usize) < palette.len()));
                        comparisons += 1;
                        differences += changed;
                    }
                }
            }
        }
    }
    eprintln!(
        "GPU diffusion: {differences} differing indices / {} pixels across {comparisons} cases",
        comparisons * 221
    );
}

#[test]
fn gpu_diffusion_wavefront_crosses_tile_edges_in_raster_order() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        return;
    };
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color {
            rgb: [90, 120, 100],
        },
        PaletteEntry::Color {
            rgb: [160, 90, 120],
        },
        PaletteEntry::Color {
            rgb: [90, 120, 100],
        },
    ];
    let kernels = [
        Diffusion::FloydSteinberg,
        Diffusion::Sierra,
        Diffusion::SierraLite,
        Diffusion::Atkinson,
        Diffusion::JarvisJudiceNinke,
        Diffusion::Stucki,
        Diffusion::Burkes,
        Diffusion::TwoRowSierra,
        Diffusion::Fan,
        Diffusion::ShiauFan,
        Diffusion::ShiauFan2,
        Diffusion::Simple2d,
    ];
    for (width, height) in [(79, 67), (1, 129), (129, 1), (2051, 5)] {
        let source: Vec<_> = (0..width * height)
            .flat_map(|i| {
                let shade = 90 + ((i % width) * 37 / width) as u8;
                [shade, shade, shade, 255]
            })
            .collect();
        for kernel in kernels {
            for serpentine in [false, true] {
                let request = DitherQuantizeRequest {
                    quantize: QuantizeRequest {
                        version: 1,
                        source: Source {
                            width,
                            height,
                            data: &source,
                        },
                        palette: &palette,
                        alpha: AlphaPolicy::Preserve { threshold: 127.0 },
                        matching: MatchPolicy::SrgbEuclidean,
                    },
                    dither: DitherPolicy::Diffusion {
                        kernel,
                        feedback: DiffusionFeedback::SrgbBytes,
                        serpentine,
                        strength: 1.0,
                        placement: Placement::Everywhere {},
                    },
                };
                let expected = block_on(backend.diffuse_gpu_scheduled(
                    request,
                    128 * 1024 * 1024,
                    None,
                    false,
                    &mut || Ok(()),
                    true,
                ))
                .unwrap()
                .0;
                let actual = block_on(backend.diffuse_gpu(request, 128 * 1024 * 1024)).unwrap();
                let changed = actual
                    .indices
                    .data()
                    .iter()
                    .zip(expected.indices.data())
                    .filter(|(a, b)| a != b)
                    .count();
                assert!(
                    changed == 0,
                    "{width}x{height}/{kernel:?}/{serpentine}: {changed}; first={:?}",
                    actual
                        .indices
                        .data()
                        .iter()
                        .zip(expected.indices.data())
                        .position(|(a, b)| a != b)
                );
            }
        }
    }
}

#[test]
fn yliluoma_matches_literal_search_across_metrics_alpha_and_placement() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        return;
    };
    // Proves actual GPU mixture search preserves original palette indices,
    // duplicate ties, global Bayer phase, alpha preparation and Adaptive output.
    let source: Vec<_> = (0..17 * 13u32)
        .flat_map(|i| {
            [
                (i * 71) as u8,
                (i * 37 + 8) as u8,
                (i * 113 + 12) as u8,
                [0, 127, 128, 255][i as usize % 4],
            ]
        })
        .collect();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color {
            rgb: [90, 120, 100],
        },
        PaletteEntry::Color {
            rgb: [160, 90, 120],
        },
        PaletteEntry::Color {
            rgb: [90, 120, 100],
        },
    ];
    let policies = [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
        MatchPolicy::YcbcrEuclidean,
    ];
    let mut total = 0;
    let mut differences = 0;
    for matching in policies {
        for size in [
            BayerSize::Two,
            BayerSize::Four,
            BayerSize::Eight,
            BayerSize::Sixteen,
        ] {
            for placement in [
                Placement::Everywhere {},
                Placement::SmoothAreaV1 {
                    sample_size: 4,
                    contrast_threshold: 1.0,
                },
            ] {
                for alpha in [
                    AlphaPolicy::Preserve { threshold: 127.0 },
                    AlphaPolicy::Premultiplied {},
                    AlphaPolicy::Matte { rgb: [7, 200, 90] },
                ] {
                    let request = DitherQuantizeRequest {
                        quantize: QuantizeRequest {
                            version: 1,
                            source: Source {
                                width: 17,
                                height: 13,
                                data: &source,
                            },
                            palette: &palette,
                            alpha,
                            matching,
                        },
                        dither: DitherPolicy::Yliluoma { size, placement },
                    };
                    let expected = cpu(request);
                    let actual =
                        block_on(backend.dither_and_quantize(request, 128 * 1024 * 1024)).unwrap();
                    assert_eq!(actual.palette, expected.palette);
                    assert_eq!(actual.warnings, expected.warnings);
                    let changed = actual
                        .indices
                        .data()
                        .iter()
                        .zip(expected.indices.data())
                        .filter(|(a, b)| a != b)
                        .count();
                    assert!(
                        changed <= 3,
                        "{matching:?}/{size:?}/{placement:?}/{alpha:?}: {changed}/221"
                    );
                    assert!(actual
                        .indices
                        .data()
                        .iter()
                        .all(|&i| i != 5 && (i as usize) < palette.len()));
                    differences += changed;
                    total += 1;
                }
            }
        }
    }
    eprintln!(
        "Yliluoma: {differences} differing indices / {} pixels across {total} cases",
        total * 221
    );
    let request = DitherQuantizeRequest {
        quantize: QuantizeRequest {
            version: 1,
            source: Source {
                width: 17,
                height: 13,
                data: &source,
            },
            palette: &palette,
            alpha: AlphaPolicy::Preserve { threshold: 127.0 },
            matching: MatchPolicy::SrgbEuclidean,
        },
        dither: DitherPolicy::Yliluoma {
            size: BayerSize::Sixteen,
            placement: Placement::Everywhere {},
        },
    };
    let required = MixMemory::new(
        ImageDimensions::new(17, 13).unwrap(),
        palette.len() as u32,
        16,
    )
    .required_bytes;
    assert_eq!(
        block_on(backend.dither_and_quantize(request, required - 1)),
        Err(Fallback::MemoryLimit)
    );
    assert_eq!(
        block_on(backend.dither_and_quantize(request, required)).unwrap(),
        cpu(request)
    );
    let mut checks = 0;
    assert_eq!(
        block_on(
            backend.dither_and_quantize_checked(request, required, &mut || {
                checks += 1;
                if checks == 4 {
                    Err(Fallback::Cancelled)
                } else {
                    Ok(())
                }
            })
        ),
        Err(Fallback::Cancelled)
    );
    assert_eq!(
        block_on(backend.dither_and_quantize(request, required)).unwrap(),
        cpu(request)
    );
    let transparent = [PaletteEntry::Transparent {}];
    let request = DitherQuantizeRequest {
        quantize: QuantizeRequest {
            palette: &transparent,
            ..request.quantize
        },
        ..request
    };
    assert_eq!(
        block_on(backend.dither_and_quantize(request, required)).unwrap(),
        cpu(request)
    );
}

#[test]
fn gpu_diffusion_masks_only_output_and_recovers_after_control_failures() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        return;
    };
    // Proves coverage cannot change the continuous recurrence, including across
    // wavefront tiles and serial chunk boundaries; admission/callback failures
    // publish no partial image and leave the next device job usable.
    let (width, height) = (2051, 5);
    let dimensions = ImageDimensions::new(width, height).unwrap();
    let source: Vec<_> = (0..width * height)
        .flat_map(|i| [(i * 17) as u8, (i * 73) as u8, (i * 29 + 40) as u8, 255])
        .collect();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color {
            rgb: [90, 120, 100],
        },
        PaletteEntry::Color {
            rgb: [160, 90, 120],
        },
    ];
    let base = QuantizeRequest {
        version: 1,
        source: Source {
            width,
            height,
            data: &source,
        },
        palette: &palette,
        alpha: AlphaPolicy::Preserve { threshold: 127.0 },
        matching: MatchPolicy::OklabEuclidean,
    };
    let plain = block_on(backend.dither_and_quantize(
        DitherQuantizeRequest {
            quantize: base,
            dither: DitherPolicy::None {},
        },
        128 * 1024 * 1024,
    ))
    .unwrap();
    let bits: Vec<_> = (0..AreaMemory::new(dimensions).coverage_bytes / 4)
        .flat_map(|_| 0x96cf_30a5u32.to_le_bytes())
        .collect();
    for serpentine in [false, true] {
        let request = DitherQuantizeRequest {
            quantize: base,
            dither: DitherPolicy::Diffusion {
                kernel: Diffusion::Sierra,
                feedback: DiffusionFeedback::Matching,
                serpentine,
                strength: 1.0,
                placement: Placement::Everywhere {},
            },
        };
        let required = DiffusionMemory::new(dimensions).required_bytes;
        assert_eq!(
            block_on(backend.diffuse_gpu(request, required - 1)),
            Err(Fallback::MemoryLimit)
        );
        let mut checks = 0;
        let failed =
            block_on(
                backend.diffuse_gpu_checked(request, required, None, false, &mut || {
                    checks += 1;
                    if checks == 3 {
                        Err(Fallback::Callback)
                    } else {
                        Ok(())
                    }
                }),
            );
        assert_eq!(failed, Err(Fallback::Callback));
        let ordinary = block_on(backend.diffuse_gpu(request, required)).unwrap();
        let masked = DitherQuantizeRequest {
            dither: DitherPolicy::Diffusion {
                kernel: Diffusion::Sierra,
                feedback: DiffusionFeedback::Matching,
                serpentine,
                strength: 1.0,
                placement: Placement::SmoothAreaV1 {
                    sample_size: 64,
                    contrast_threshold: 0.12,
                },
            },
            ..request
        };
        let (actual, pending) = block_on(backend.diffuse_gpu_checked(
            masked,
            required,
            Some(bits.clone()),
            true,
            &mut || Ok(()),
        ))
        .unwrap();
        assert!(pending.is_none());
        for (i, &actual) in actual.indices.data().iter().enumerate() {
            let expected = if bits[i / 8] & (1 << (i % 8)) != 0 {
                ordinary.indices.data()[i]
            } else {
                plain.indices.data()[i]
            };
            assert_eq!(actual, expected, "serpentine={serpentine},pixel={i}");
        }
    }
}

#[test]
fn cancelled_batches_leave_no_result_and_keep_the_device_usable() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        return;
    };
    // Proves a cancellation between bounded dispatch batches releases work and
    // leaves the single device usable, rather than publishing partial bytes.
    let dimensions = ImageDimensions::new(193, 127).unwrap();
    let source = vec![128; 193 * 127 * 4];
    let mut checks = 0;
    let result = block_on(backend.adaptive_coverage_checked(
        dimensions,
        &source,
        20,
        0.12,
        128 * 1024 * 1024,
        &mut || {
            checks += 1;
            if checks == 4 {
                Err(Fallback::Cancelled)
            } else {
                Ok(())
            }
        },
    ));
    assert_eq!(result, Err(Fallback::Cancelled));
    assert!(matches!(backend, super::super::Backend::Gpu(_)));
    assert!(
        block_on(backend.adaptive_coverage(dimensions, &source, 20, 0.12, 128 * 1024 * 1024))
            .is_ok()
    );
}

#[test]
fn retained_gpu_coverage_bypasses_analysis_across_fields_and_diffusion() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        return;
    };
    let source: Vec<_> = (0..53 * 37u32)
        .flat_map(|i| [64 + (i % 53) as u8, 100 + (i / 53) as u8, 120, 255])
        .collect();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color {
            rgb: [90, 120, 100],
        },
    ];
    let quantize = QuantizeRequest {
        version: 1,
        source: Source {
            width: 53,
            height: 37,
            data: &source,
        },
        palette: &palette,
        alpha: AlphaPolicy::Preserve { threshold: 127.0 },
        matching: MatchPolicy::OklabEuclidean,
    };
    let area = Placement::SmoothAreaV1 {
        sample_size: 20,
        contrast_threshold: 0.12,
    };
    let field = DitherPolicy::Separable {
        perturb: PerturbPolicy {
            field: Field::Bayer {
                size: BayerSize::Sixteen,
            },
            space: WorkingSpace::Oklab,
            strength: 0.3,
            placement: area,
        },
    };
    let mut cold_checks = 0;
    let (cold, coverage) = block_on(backend.dither_prepared_checked(
        DitherQuantizeRequest {
            quantize,
            dither: field,
        },
        128 * 1024 * 1024,
        None,
        true,
        &mut || {
            cold_checks += 1;
            Ok(())
        },
    ))
    .unwrap();
    assert_eq!(
        cold,
        cpu(DitherQuantizeRequest {
            quantize,
            dither: field
        })
    );
    let coverage = coverage.expect("successful analysis exports pending coverage");
    for dither in [
        DitherPolicy::Separable {
            perturb: PerturbPolicy {
                field: Field::BlueNoise {},
                space: WorkingSpace::Cielab,
                strength: 1.0,
                placement: area,
            },
        },
        DitherPolicy::Diffusion {
            kernel: Diffusion::Sierra,
            feedback: DiffusionFeedback::Matching,
            serpentine: true,
            strength: 1.0,
            placement: area,
        },
    ] {
        #[derive(Default)]
        struct Control {
            checks: usize,
            rows: usize,
        }
        impl JobControl for Control {
            fn check(&mut self) -> Result<(), Fallback> {
                self.checks += 1;
                Ok(())
            }
            fn row(&mut self, _row: u32) -> Result<(), Fallback> {
                self.rows += 1;
                Ok(())
            }
        }
        let mut control = Control::default();
        let request = DitherQuantizeRequest { quantize, dither };
        let (warm, pending) = block_on(backend.dither_prepared_checked(
            request,
            128 * 1024 * 1024,
            Some(coverage.clone()),
            true,
            &mut control,
        ))
        .unwrap();
        assert_eq!(warm, cpu(request));
        assert!(
            control.checks < cold_checks,
            "warm {}, cold {cold_checks}",
            control.checks
        );
        assert_eq!(
            control.rows,
            if matches!(dither, DitherPolicy::Diffusion { .. }) {
                37
            } else {
                0
            }
        );
        assert!(
            pending.is_none(),
            "reuse does not republish a duplicate mask"
        );
    }
}
