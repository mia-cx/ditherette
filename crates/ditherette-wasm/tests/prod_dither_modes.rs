//! Production parity with the `dither_modes` reference: every kernel and every rectangular tile.

use ditherette_wasm::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageDimensions, PaletteIndex8,
    },
    prod::{
        contract::{
            failure::Failure,
            request::{
                AlphaPolicy, Diffusion, DiffusionFeedback, DitherPolicy, DitherQuantizeRequest,
                Field, MatchPolicy, OrderedTile, PerturbPolicy, Placement, QuantizeRequest, Source,
                WorkingSpace,
            },
        },
        dither::error_diffusion as production,
        pipeline::{
            perturb::PerturbRequest,
            processor::{Boundary as RgbaBoundary, InputBoundary, Processor},
            quantize::{IndexedMetadataRef, QuantizeBoundary, QuantizeRequest as ProcessorRequest},
        },
    },
    spec::{self, dither_modes},
};
use serde::{de::DeserializeOwned, Serialize};

const KERNELS: [Diffusion; 12] = [
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
const TILES: [OrderedTile; 4] = [
    OrderedTile::ThreeByOne,
    OrderedTile::FourByOne,
    OrderedTile::FourByTwo,
    OrderedTile::FiveByThree,
];
const MATCHING: [MatchPolicy; 4] = [
    MatchPolicy::SrgbEuclidean,
    MatchPolicy::OklabEuclidean,
    MatchPolicy::CielabCiede2000,
    MatchPolicy::OklchHueArc,
];
const PALETTE: [PaletteEntry; 5] = [
    PaletteEntry::Color { rgb: [0; 3] },
    PaletteEntry::Color { rgb: [181, 31, 91] },
    PaletteEntry::Color { rgb: [255; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
    PaletteEntry::Transparent {},
];

/// Serializes one family's value into the other's identically tagged type.
fn same_tag<T: Serialize, U: DeserializeOwned>(value: T) -> U {
    serde_json::from_value(serde_json::to_value(value).unwrap()).unwrap()
}

struct Data<'a>(&'a [u8]);

impl InputBoundary for Data<'_> {
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.0.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        destination.copy_from_slice(self.0);
        Ok(())
    }
}

impl RgbaBoundary for Data<'_> {
    type Output = Vec<u8>;
    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        Ok(bytes.to_vec())
    }
}

impl QuantizeBoundary for Data<'_> {
    type Output = IndexedImage;
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        palette: IndexedMetadataRef<'_>,
    ) -> Result<IndexedImage, Failure> {
        Ok(IndexedImage {
            indices: ImageBuf::<PaletteIndex8>::from_vec_packed(indices.to_vec(), dimensions)
                .unwrap(),
            palette: palette.palette.clone(),
            warnings: palette.warnings.to_vec(),
        })
    }
}

fn image(width: u32, height: u32) -> Vec<u8> {
    (0..width * height)
        .flat_map(|i| {
            [
                (i * 73 + 17) as u8,
                (i * 31 + 99) as u8,
                (i * 117 + 41) as u8,
                [0, 127, 128, 255][i as usize % 4],
            ]
        })
        .collect()
}

fn reference_quantize<'a>(
    data: &'a [u8],
    width: u32,
    height: u32,
    alpha: AlphaPolicy,
    matching: MatchPolicy,
) -> spec::contract::request::QuantizeRequest<'a> {
    spec::contract::request::QuantizeRequest {
        version: 1,
        source: spec::contract::request::Source {
            width,
            height,
            data,
        },
        palette: &PALETTE,
        alpha: same_tag(alpha),
        matching: same_tag(matching),
    }
}

#[test]
fn every_kernel_matches_the_reference_on_all_production_paths() {
    let mut processor = Processor::new(1 << 22, 0).unwrap();
    let mut cases = 0;
    for (width, height) in [(1, 3), (4, 1), (5, 4), (7, 3)] {
        let data = image(width, height);
        for kernel in KERNELS {
            for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
                for matching in MATCHING {
                    for serpentine in [false, true] {
                        for (alpha, placement) in [
                            (
                                AlphaPolicy::Preserve {
                                    threshold: 127.9999999,
                                },
                                Placement::Everywhere {},
                            ),
                            (
                                AlphaPolicy::Matte { rgb: [33, 71, 109] },
                                Placement::Adaptive {
                                    radius: 1,
                                    threshold: 5.0,
                                    softness: 10.0,
                                },
                            ),
                        ] {
                            let dither = DitherPolicy::Diffusion {
                                kernel,
                                feedback,
                                strength: 0.75,
                                serpentine,
                                placement,
                            };
                            let expected = dither_modes::diffusion::diffuse(
                                reference_quantize(&data, width, height, alpha, matching),
                                dither_modes::diffusion::DiffusionPolicy {
                                    kernel: same_tag(kernel),
                                    strength: 0.75,
                                    placement: same_tag(placement),
                                    serpentine,
                                    feedback: same_tag(feedback),
                                },
                            )
                            .unwrap();
                            let request = DitherQuantizeRequest {
                                quantize: QuantizeRequest {
                                    version: 1,
                                    source: Source {
                                        width,
                                        height,
                                        data: &data,
                                    },
                                    palette: &PALETTE,
                                    alpha,
                                    matching,
                                },
                                dither,
                            };
                            let label = format!("{kernel:?} {width}x{height} {dither:?}");
                            assert_eq!(production::diffuse(request).unwrap(), expected, "{label}");
                            assert_eq!(
                                production::prepared::diffuse(request, u64::MAX).unwrap(),
                                expected,
                                "{label}"
                            );
                            let processed = processor
                                .dither_and_quantize(
                                    ProcessorRequest {
                                        source_width: width,
                                        source_height: height,
                                        palette: &PALETTE,
                                        alpha,
                                        matching,
                                    },
                                    dither,
                                    &mut Data(&data),
                                )
                                .unwrap();
                            assert_eq!(processed, expected, "{label}");
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 4 * 12 * 2 * 4 * 2 * 2);
}

#[test]
fn every_tile_matches_the_reference_perturb_and_quantize() {
    let mut processor = Processor::new(1 << 22, 0).unwrap();
    let (width, height) = (11, 7);
    let data = image(width, height);
    for tile in TILES {
        for space in [
            WorkingSpace::Srgb,
            WorkingSpace::Oklab,
            WorkingSpace::Cielch,
        ] {
            for placement in [
                Placement::Everywhere {},
                Placement::Adaptive {
                    radius: 2,
                    threshold: 4.0,
                    softness: 7.0,
                },
            ] {
                let perturb = PerturbPolicy {
                    field: Field::Ordered { tile },
                    space,
                    strength: 0.8,
                    placement,
                };
                let reference = dither_modes::ordered::TilePerturbPolicy {
                    tile: same_tag(tile),
                    space: same_tag(space),
                    strength: 0.8,
                    placement: same_tag(placement),
                };
                let source = spec::contract::request::Source {
                    width,
                    height,
                    data: &data,
                };
                let expected = dither_modes::ordered::perturb(source, reference).unwrap();
                let actual = processor
                    .perturb(
                        PerturbRequest {
                            source_width: width,
                            source_height: height,
                            perturb,
                        },
                        &mut Data(&data),
                    )
                    .unwrap();
                assert_eq!(actual, expected.data(), "{tile:?} {space:?}");
                for matching in MATCHING {
                    let alpha = AlphaPolicy::Premultiplied {};
                    let expected = dither_modes::ordered::dither_and_quantize(
                        reference_quantize(&data, width, height, alpha, matching),
                        reference,
                    )
                    .unwrap();
                    let actual = processor
                        .dither_and_quantize(
                            ProcessorRequest {
                                source_width: width,
                                source_height: height,
                                palette: &PALETTE,
                                alpha,
                                matching,
                            },
                            DitherPolicy::Separable { perturb },
                            &mut Data(&data),
                        )
                        .unwrap();
                    assert_eq!(actual, expected, "{tile:?} {space:?} {matching:?}");
                }
            }
        }
    }
}

#[test]
fn new_tags_serialize_like_the_reference() {
    assert_eq!(
        serde_json::to_value(KERNELS).unwrap(),
        serde_json::to_value(KERNELS.map(same_tag::<_, dither_modes::diffusion::Kernel>)).unwrap()
    );
    assert_eq!(
        serde_json::to_string(&Field::Ordered {
            tile: OrderedTile::FiveByThree
        })
        .unwrap(),
        r#"{"algorithm":"ordered","tile":"5x3"}"#
    );
}
