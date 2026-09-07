use ditherette_wasm::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageDimensions, PaletteIndex8,
    },
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{
                AlphaPolicy, BayerSize, DitherPolicy, Field, MatchPolicy, PerturbPolicy, Placement,
                WorkingSpace,
            },
        },
        palette::PreparedPalette,
        pipeline::{
            perturb::PerturbRequest,
            processor::{Allocator, Boundary as RgbaBoundary, Processor},
            quantize::{QuantizeBoundary, QuantizeRequest},
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

struct Boundary {
    copies: usize,
    completions: usize,
    fail_copy: bool,
    fail_complete: bool,
}
impl Boundary {
    fn new() -> Self {
        Self {
            copies: 0,
            completions: 0,
            fail_copy: false,
            fail_complete: false,
        }
    }
    fn copy(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        self.copies += 1;
        if self.fail_copy {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::SourceData,
            ));
        }
        destination.copy_from_slice(&SOURCE);
        Ok(())
    }
    fn finish(&mut self) -> Result<(), Failure> {
        self.completions += 1;
        if self.fail_complete {
            Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Output,
            ))
        } else {
            Ok(())
        }
    }
}
impl RgbaBoundary for Boundary {
    type Output = Vec<u8>;
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(SOURCE.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        self.copy(destination)
    }
    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        self.finish()?;
        Ok(bytes.to_vec())
    }
}
impl QuantizeBoundary for Boundary {
    type Output = IndexedImage;
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(SOURCE.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        self.copy(destination)
    }
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        palette: &PreparedPalette,
    ) -> Result<IndexedImage, Failure> {
        self.finish()?;
        Ok(IndexedImage {
            indices: ImageBuf::<PaletteIndex8>::from_vec_packed(indices.to_vec(), dimensions)
                .unwrap(),
            palette: palette.palette.clone(),
            warnings: palette.warnings.clone(),
        })
    }
}

fn policy() -> PerturbPolicy {
    PerturbPolicy {
        field: Field::Random { seed: u32::MAX },
        space: WorkingSpace::Oklch,
        strength: 0.7,
        placement: Placement::Adaptive {
            radius: 1,
            threshold: 4.0,
            softness: 7.0,
        },
    }
}
fn request() -> PerturbRequest {
    PerturbRequest {
        source_width: 2,
        source_height: 2,
        perturb: policy(),
    }
}
fn quantize() -> QuantizeRequest<'static> {
    QuantizeRequest {
        source_width: 2,
        source_height: 2,
        palette: &PALETTE,
        alpha: AlphaPolicy::Preserve {
            threshold: 127.9999999,
        },
        matching: MatchPolicy::CielchHueArc,
    }
}

#[test]
fn bounded_calls_match_frozen_bytes_and_complete_separable_results() {
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Oklch,
        WorkingSpace::Cielab,
        WorkingSpace::Cielch,
        WorkingSpace::Ycbcr,
    ] {
        for field in [
            Field::Random { seed: 0 },
            Field::Random { seed: u32::MAX },
            Field::Bayer {
                size: BayerSize::Two,
            },
            Field::Bayer {
                size: BayerSize::Four,
            },
            Field::Bayer {
                size: BayerSize::Eight,
            },
            Field::Bayer {
                size: BayerSize::Sixteen,
            },
        ] {
            let policy = PerturbPolicy {
                space,
                field,
                ..policy()
            };
            let source = ditherette_wasm::image::ImageView::packed(
                &SOURCE,
                ImageDimensions::new(2, 2).unwrap(),
            )
            .unwrap();
            let expected = spec::dither::perturb::perturb(
                source,
                serde_json::from_value(serde_json::to_value(policy).unwrap()).unwrap(),
            )
            .unwrap();
            let actual = processor
                .perturb(
                    PerturbRequest {
                        perturb: policy,
                        ..request()
                    },
                    &mut Boundary::new(),
                )
                .unwrap();
            assert_eq!(actual, expected.data());
            for matching in [
                MatchPolicy::SrgbEuclidean,
                MatchPolicy::SrgbCompuphase,
                MatchPolicy::CielabCiede2000,
                MatchPolicy::OklchCircularHue,
                MatchPolicy::CielchHueArc,
            ] {
                let request = QuantizeRequest {
                    matching,
                    ..quantize()
                };
                let actual = processor
                    .dither_and_quantize(
                        request,
                        DitherPolicy::Separable { perturb: policy },
                        &mut Boundary::new(),
                    )
                    .unwrap();
                let expected = spec::quantize::quantize(spec::contract::request::QuantizeRequest {
                    version: 1,
                    source: spec::contract::request::Source {
                        width: 2,
                        height: 2,
                        data: expected.data(),
                    },
                    palette: &PALETTE,
                    alpha: serde_json::from_value(serde_json::to_value(request.alpha).unwrap())
                        .unwrap(),
                    matching: serde_json::from_value(serde_json::to_value(matching).unwrap())
                        .unwrap(),
                })
                .unwrap();
                assert_eq!(actual, expected);
            }
        }
    }
    assert_eq!(
        processor
            .dither_and_quantize(quantize(), DitherPolicy::None {}, &mut Boundary::new())
            .unwrap(),
        processor
            .quantize(quantize(), &mut Boundary::new())
            .unwrap()
    );
}

#[test]
fn exact_capacity_one_under_and_caught_failure_recovery_cover_both_result_shapes() {
    let mut probe = Processor::new(1 << 20, 0).unwrap();
    let rgba = probe.perturb(request(), &mut Boundary::new()).unwrap();
    let rgba_capacity = probe.peak_capacity_bytes();
    let indexed = probe
        .dither_and_quantize(
            quantize(),
            DitherPolicy::Separable { perturb: policy() },
            &mut Boundary::new(),
        )
        .unwrap();
    let indexed_capacity = probe.peak_capacity_bytes();
    for (fused, capacity) in [(false, rgba_capacity), (true, indexed_capacity)] {
        let mut exact = Processor::new(capacity, 0).unwrap();
        let mut under = Processor::new(capacity - 1, 0).unwrap();
        let mut boundary = Boundary::new();
        let error = if fused {
            under
                .dither_and_quantize(
                    quantize(),
                    DitherPolicy::Separable { perturb: policy() },
                    &mut boundary,
                )
                .unwrap_err()
        } else {
            under.perturb(request(), &mut boundary).unwrap_err()
        };
        assert_eq!(error.code, ErrorCode::MemoryLimit);
        assert_eq!((boundary.copies, boundary.completions), (0, 0));
        for (fail_copy, fail_complete) in [(true, false), (false, true)] {
            let mut boundary = Boundary {
                fail_copy,
                fail_complete,
                ..Boundary::new()
            };
            let error = if fused {
                exact
                    .dither_and_quantize(
                        quantize(),
                        DitherPolicy::Separable { perturb: policy() },
                        &mut boundary,
                    )
                    .unwrap_err()
            } else {
                exact.perturb(request(), &mut boundary).unwrap_err()
            };
            assert_eq!(error.code, ErrorCode::WasmMemoryUnavailable);
            if fused {
                assert_eq!(
                    exact
                        .dither_and_quantize(
                            quantize(),
                            DitherPolicy::Separable { perturb: policy() },
                            &mut Boundary::new()
                        )
                        .unwrap(),
                    indexed
                );
            } else {
                assert_eq!(
                    exact.perturb(request(), &mut Boundary::new()).unwrap(),
                    rgba
                );
            }
            assert_eq!(exact.peak_capacity_bytes(), capacity);
        }
        exact.dispose().unwrap();
        exact.dispose().unwrap();
        assert_eq!(
            exact
                .perturb(request(), &mut Boundary::new())
                .unwrap_err()
                .code,
            ErrorCode::Disposed
        );
        assert_eq!(
            exact
                .dither_and_quantize(quantize(), DitherPolicy::None {}, &mut Boundary::new())
                .unwrap_err()
                .code,
            ErrorCode::Disposed
        );
    }
}

struct Allocation {
    fail_at: usize,
    calls: usize,
    extra: usize,
}
impl Allocator for Allocation {
    fn reserve(&mut self, buffer: &mut Vec<u8>, additional: usize) -> Result<(), Failure> {
        let call = self.calls;
        self.calls += 1;
        if call == self.fail_at {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Wasm,
            ));
        }
        buffer.try_reserve_exact(additional + self.extra).unwrap();
        Ok(())
    }
}

#[test]
fn every_buffer_reservation_failure_and_extra_capacity_fail_before_source_copy() {
    for fused in [false, true] {
        let mut processor = Processor::new(1 << 20, 0).unwrap();
        for fail_at in 0..if fused { 3 } else { 2 } {
            let mut boundary = Boundary::new();
            let mut allocator = Allocation {
                fail_at,
                calls: 0,
                extra: 0,
            };
            let error = if fused {
                processor
                    .dither_and_quantize_with_allocator(
                        quantize(),
                        DitherPolicy::Separable { perturb: policy() },
                        &mut boundary,
                        &mut allocator,
                    )
                    .unwrap_err()
            } else {
                processor
                    .perturb_with_allocator(request(), &mut boundary, &mut allocator)
                    .unwrap_err()
            };
            assert_eq!(error.code, ErrorCode::WasmMemoryUnavailable);
            assert_eq!((boundary.copies, boundary.completions), (0, 0));
            processor.perturb(request(), &mut Boundary::new()).unwrap();
        }
        if fused {
            processor
                .dither_and_quantize(
                    quantize(),
                    DitherPolicy::Separable { perturb: policy() },
                    &mut Boundary::new(),
                )
                .unwrap();
        } else {
            processor.perturb(request(), &mut Boundary::new()).unwrap();
        }
        let mut exact = Processor::new(processor.peak_capacity_bytes(), 0).unwrap();
        let mut allocator = Allocation {
            fail_at: usize::MAX,
            calls: 0,
            extra: 32,
        };
        let mut boundary = Boundary::new();
        let error = if fused {
            exact
                .dither_and_quantize_with_allocator(
                    quantize(),
                    DitherPolicy::Separable { perturb: policy() },
                    &mut boundary,
                    &mut allocator,
                )
                .unwrap_err()
        } else {
            exact
                .perturb_with_allocator(request(), &mut boundary, &mut allocator)
                .unwrap_err()
        };
        assert_eq!(error.code, ErrorCode::MemoryLimit);
        assert_eq!((boundary.copies, boundary.completions), (0, 0));
    }
}

#[test]
fn unsupported_and_nonfinite_policies_fail_without_allocating_or_copying() {
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    for perturb in [
        PerturbPolicy {
            field: Field::BlueNoise {},
            ..policy()
        },
        PerturbPolicy {
            strength: f32::NAN,
            ..policy()
        },
        PerturbPolicy {
            strength: -1.0,
            ..policy()
        },
        PerturbPolicy {
            placement: Placement::Adaptive {
                radius: 0,
                threshold: 0.0,
                softness: 0.0,
            },
            ..policy()
        },
    ] {
        let mut boundary = Boundary::new();
        let mut allocator = Allocation {
            fail_at: usize::MAX,
            calls: 0,
            extra: 0,
        };
        let error = processor
            .perturb_with_allocator(
                PerturbRequest {
                    perturb,
                    ..request()
                },
                &mut boundary,
                &mut allocator,
            )
            .unwrap_err();
        assert!(matches!(
            error.code,
            ErrorCode::InvalidSettings | ErrorCode::UnsupportedOperation
        ));
        assert_eq!(
            (allocator.calls, boundary.copies, boundary.completions),
            (0, 0, 0)
        );
        processor.perturb(request(), &mut Boundary::new()).unwrap();
    }
}
