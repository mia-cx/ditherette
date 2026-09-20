use ditherette_wasm::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageDimensions, PaletteIndex8,
    },
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::*,
        },
        palette::PreparedPalette,
        pipeline::{
            process::ProcessRequest,
            processor::{Allocator, Processor},
            quantize::QuantizeBoundary,
        },
    },
    spec,
};

const SOURCE: [u8; 16] = [
    255, 0, 0, 0, 17, 33, 71, 127, 0, 255, 0, 128, 0, 0, 255, 255,
];
const PALETTE: [PaletteEntry; 4] = [
    PaletteEntry::Color { rgb: [0; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
    PaletteEntry::Transparent {},
];

#[derive(Default)]
struct Boundary<'a> {
    copies: usize,
    completions: usize,
    failure: u8,
    data: Option<&'a [u8]>,
}
impl QuantizeBoundary for Boundary<'_> {
    type Output = IndexedImage;
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.data.unwrap_or(&SOURCE).len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        self.copies += 1;
        if self.failure == 1 {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::SourceData,
            ));
        }
        destination.copy_from_slice(self.data.unwrap_or(&SOURCE));
        Ok(())
    }
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        palette: &PreparedPalette,
    ) -> Result<IndexedImage, Failure> {
        self.completions += 1;
        if self.failure == 2 {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Output,
            ));
        }
        Ok(IndexedImage {
            indices: ImageBuf::<PaletteIndex8>::from_vec_packed(indices.to_vec(), dimensions)
                .unwrap(),
            palette: palette.palette.clone(),
            warnings: palette.warnings.clone(),
        })
    }
}

impl ditherette_wasm::prod::pipeline::processor::Boundary for Boundary<'_> {
    type Output = Vec<u8>;
    fn input_len(&mut self) -> Result<usize, Failure> {
        QuantizeBoundary::input_len(self)
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        QuantizeBoundary::copy_input(self, destination)
    }
    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        Ok(bytes.to_vec())
    }
}

fn request(dither: DitherPolicy) -> ProcessRequest<'static> {
    ProcessRequest {
        source_width: 2,
        source_height: 2,
        palette: &PALETTE,
        recipe: RecipeV1 {
            version: 1,
            output: Output {
                width: 3,
                height: 4,
                resize: ResizePolicy::Nearest {
                    anchor: Anchor::BottomRight,
                },
            },
            alpha: AlphaPolicy::Preserve {
                threshold: 127.9999999,
            },
            matching: MatchPolicy::SrgbEuclidean,
            dither,
        },
    }
}

fn dithers() -> [DitherPolicy; 4] {
    [
        DitherPolicy::None {},
        DitherPolicy::Separable {
            perturb: PerturbPolicy {
                field: Field::BlueNoise {},
                space: WorkingSpace::Oklab,
                strength: 0.7,
                placement: Placement::Everywhere {},
            },
        },
        DitherPolicy::Diffusion {
            kernel: Diffusion::SierraLite,
            feedback: DiffusionFeedback::Matching,
            strength: 0.7,
            serpentine: true,
            placement: Placement::Everywhere {},
        },
        DitherPolicy::Yliluoma {
            size: BayerSize::Four,
            placement: Placement::Everywhere {},
        },
    ]
}

#[test]
fn process_keeps_frozen_resize_then_dither_bytes_and_metadata_for_every_family() {
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    for dither in dithers() {
        let request = request(dither);
        let expected = spec::pipeline::process(spec::contract::request::ProcessRequest {
            source: spec::contract::request::Source {
                width: 2,
                height: 2,
                data: &SOURCE,
            },
            palette: &PALETTE,
            recipe: serde_json::from_value(serde_json::to_value(request.recipe).unwrap()).unwrap(),
        })
        .unwrap();
        let mut boundary = Boundary::default();
        let actual = processor.process(request, &mut boundary).unwrap();
        assert_eq!(actual, expected, "{dither:?}");
        assert_eq!((boundary.copies, boundary.completions), (1, 1));
    }
}

#[test]
fn every_resize_family_composes_with_actual_staged_production_calls() {
    use ditherette_wasm::prod::pipeline::{processor::ResizeRequest, quantize::QuantizeRequest};
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    for resize in [
        ResizePolicy::Nearest {
            anchor: Anchor::BottomRight,
        },
        ResizePolicy::Area {},
        ResizePolicy::Bilinear {
            anchor: Anchor::Center,
        },
        ResizePolicy::Bicubic {
            anchor: Anchor::TopLeft,
            support: Support::Fixed,
        },
        ResizePolicy::Lanczos2 {
            anchor: Anchor::Bottom,
            support: Support::ScaleAware,
        },
        ResizePolicy::Lanczos3 {
            anchor: Anchor::Right,
            support: Support::Fixed,
        },
        ResizePolicy::Trilinear {
            anchor: Anchor::Center,
        },
    ] {
        for dither in dithers() {
            let mut request = request(dither);
            request.recipe.output.resize = resize;
            let resized = processor
                .resize(
                    ResizeRequest {
                        source_width: 2,
                        source_height: 2,
                        output: request.recipe.output,
                    },
                    &mut Boundary::default(),
                )
                .unwrap();
            let staged = processor
                .dither_and_quantize(
                    QuantizeRequest {
                        source_width: request.recipe.output.width,
                        source_height: request.recipe.output.height,
                        palette: request.palette,
                        alpha: request.recipe.alpha,
                        matching: request.recipe.matching,
                    },
                    dither,
                    &mut Boundary {
                        data: Some(&resized),
                        ..Boundary::default()
                    },
                )
                .unwrap();
            let actual = processor
                .process(request, &mut Boundary::default())
                .unwrap();
            assert_eq!(actual, staged, "{resize:?} {dither:?}");
        }
    }
}

#[derive(Default)]
struct Reservation {
    calls: usize,
    fail_at: usize,
    extra: usize,
}
impl Allocator for Reservation {
    fn reserve(&mut self, buffer: &mut Vec<u8>, count: usize) -> Result<(), Failure> {
        self.calls += 1;
        if self.calls == self.fail_at {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Wasm,
            ));
        }
        buffer.try_reserve_exact(count + self.extra).unwrap();
        Ok(())
    }
}

#[test]
fn no_dither_does_not_charge_the_separable_converter() {
    let separable = request(dithers()[1]);
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    processor
        .process(separable, &mut Boundary::default())
        .unwrap();
    let perturb_bytes =
        u64::from(separable.recipe.output.width) * u64::from(separable.recipe.output.height) * 4;
    let converter_bytes =
        std::mem::size_of::<ditherette_wasm::prod::color::packed::Converter>() as u64;
    let needed = processor.peak_capacity_bytes() - perturb_bytes - converter_bytes;
    let mut bounded = Processor::new(needed, 0).unwrap();
    bounded
        .process(request(DitherPolicy::None {}), &mut Boundary::default())
        .unwrap();
    assert_eq!(bounded.peak_capacity_bytes(), needed);
}

#[test]
fn whole_call_capacity_reservations_and_caught_failures_precede_publication_and_recover() {
    for dither in dithers() {
        let request = request(dither);
        let mut processor = Processor::new(1 << 20, 0).unwrap();
        let expected = processor
            .process(request, &mut Boundary::default())
            .unwrap();
        let needed = processor.peak_capacity_bytes();
        let mut exact = Processor::new(needed, 0).unwrap();
        assert_eq!(
            exact.process(request, &mut Boundary::default()).unwrap(),
            expected
        );
        let mut short = Processor::new(needed - 1, 0).unwrap();
        let mut boundary = Boundary::default();
        let mut allocations = Reservation::default();
        assert_eq!(
            short
                .process_with_allocator(request, &mut boundary, &mut allocations)
                .unwrap_err()
                .code,
            ErrorCode::MemoryLimit
        );
        assert_eq!(
            (allocations.calls, boundary.copies, boundary.completions),
            (0, 0, 0)
        );
        let smaller = ProcessRequest {
            recipe: RecipeV1 {
                output: Output {
                    width: 1,
                    height: 1,
                    ..request.recipe.output
                },
                ..request.recipe
            },
            ..request
        };
        short.process(smaller, &mut boundary).unwrap();
        for fail_at in 1..=if matches!(dither, DitherPolicy::Separable { .. }) {
            4
        } else {
            3
        } {
            let mut boundary = Boundary::default();
            let mut allocator = Reservation {
                fail_at,
                ..Reservation::default()
            };
            assert_eq!(
                exact
                    .process_with_allocator(request, &mut boundary, &mut allocator)
                    .unwrap_err()
                    .code,
                ErrorCode::WasmMemoryUnavailable
            );
            assert_eq!(
                (allocator.calls, boundary.copies, boundary.completions),
                (fail_at, 0, 0)
            );
            assert_eq!(exact.process(request, &mut boundary).unwrap(), expected);
        }
        let mut oversized = Reservation {
            extra: 4096,
            ..Reservation::default()
        };
        let mut boundary = Boundary::default();
        assert_eq!(
            exact
                .process_with_allocator(request, &mut boundary, &mut oversized)
                .unwrap_err()
                .code,
            ErrorCode::MemoryLimit
        );
        assert_eq!(
            (oversized.calls, boundary.copies, boundary.completions),
            (1, 0, 0)
        );
        for failure in 1..=2 {
            let mut boundary = Boundary {
                failure,
                ..Boundary::default()
            };
            assert_eq!(
                exact.process(request, &mut boundary).unwrap_err().code,
                ErrorCode::WasmMemoryUnavailable
            );
            assert_eq!(
                (boundary.copies, boundary.completions),
                (1, usize::from(failure == 2))
            );
            boundary.failure = 0;
            assert_eq!(exact.process(request, &mut boundary).unwrap(), expected);
        }
        exact.dispose().unwrap();
        exact.dispose().unwrap();
        assert_eq!(
            exact
                .process(request, &mut Boundary::default())
                .unwrap_err()
                .code,
            ErrorCode::Disposed
        );
    }
}

#[test]
fn invalid_complete_settings_never_reserve_or_copy_even_with_a_large_resize() {
    let valid = request(DitherPolicy::None {});
    let mut invalid = vec![];
    let mut value = valid;
    value.recipe.version = 2;
    invalid.push(value);
    let mut value = valid;
    value.recipe.output.width = 0;
    invalid.push(value);
    let mut value = valid;
    value.recipe.alpha = AlphaPolicy::Preserve {
        threshold: f64::NAN,
    };
    invalid.push(value);
    let mut value = valid;
    value.palette = &[];
    invalid.push(value);
    let mut value = valid;
    value.recipe.dither = DitherPolicy::Diffusion {
        kernel: Diffusion::Atkinson,
        feedback: DiffusionFeedback::SrgbBytes,
        strength: f32::INFINITY,
        serpentine: false,
        placement: Placement::Everywhere {},
    };
    invalid.push(value);
    let mut value = valid;
    value.recipe.dither = DitherPolicy::Yliluoma {
        size: BayerSize::Four,
        placement: Placement::Adaptive {
            radius: 0,
            threshold: 0.0,
            softness: 0.0,
        },
    };
    invalid.push(value);
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    for mut request in invalid {
        if request.recipe.output.width != 0 {
            request.recipe.output.width = 4096;
        }
        let mut boundary = Boundary::default();
        let mut allocator = Reservation::default();
        let error = processor
            .process_with_allocator(request, &mut boundary, &mut allocator)
            .unwrap_err();
        assert!(matches!(
            error.code,
            ErrorCode::InvalidRequest | ErrorCode::InvalidSettings | ErrorCode::InvalidPalette
        ));
        assert_eq!(
            (allocator.calls, boundary.copies, boundary.completions),
            (0, 0, 0)
        );
        processor.process(valid, &mut boundary).unwrap();
    }
}
