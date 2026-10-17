use pollster::block_on;

use super::*;
use crate::image::contracts::{IndexedImage, PaletteEntry};
use crate::prod::contract::request::*;
use crate::{image::ImageView, prod::dither::smooth_area_v1};

struct CpuInput<'a>(&'a [u8]);
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
                            strength: 0.3,
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
    eprintln!(
        "indexed parity: {differences} differing indices / {} pixels across {comparisons} cases",
        comparisons * 391
    );
}

#[test]
#[ignore = "requires a host-provided 2600x4168 RGBA fixture"]
fn full_resolution_adaptive_gpu_probe() {
    let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
        panic!("GPU required");
    };
    let input =
        std::fs::read(std::env::var("DITHERETTE_GPU_RGBA").expect("RGBA fixture path")).unwrap();
    let dimensions = ImageDimensions::new(2600, 4168).unwrap();
    assert_eq!(input.len(), 2600 * 4168 * 4);
    for sample in [20, 64, 100] {
        let start = std::time::Instant::now();
        let output = block_on(backend.adaptive_coverage(
            dimensions,
            &input,
            sample,
            0.12,
            1536 * 1024 * 1024,
        ))
        .unwrap();
        let elapsed = start.elapsed();
        let mut reference = vec![[0.0; 3]; smooth_area_v1::scratch_len(dimensions)];
        smooth_area_v1::select(
            ImageView::packed(&input, dimensions).unwrap(),
            sample,
            0.12,
            &mut reference,
            || Ok::<_, ()>(()),
        )
        .unwrap();
        let differences = (0..2600 * 4168)
            .filter(|&i| {
                (output[i / 8] & (1 << (i % 8)) != 0)
                    != smooth_area_v1::covers(reference[i][2], i as u32 % 2600, i as u32 / 2600)
            })
            .count();
        eprintln!("full Adaptive sample {sample}: {differences}/10836800 coverage disagreements, GPU call {elapsed:?} (diagnostic shared-host observation)");
        assert!(
            differences < 10836800 / 200,
            "coverage differs at {differences} pixels"
        );
    }
}
