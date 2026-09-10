use ditherette_wasm::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageDimensions, PaletteIndex8,
    },
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{AlphaPolicy, MatchPolicy},
        },
        pipeline::quantize::IndexedMetadataRef,
        pipeline::{
            processor::{Allocator, Processor},
            quantize::{QuantizeBoundary, QuantizeRequest},
        },
    },
    spec,
};

struct Boundary<'a> {
    source: &'a [u8],
    copies: usize,
    fail_copy: bool,
    fail_complete: bool,
}
impl QuantizeBoundary for Boundary<'_> {
    type Output = IndexedImage;
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.source.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        self.copies += 1;
        if self.fail_copy {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::SourceData,
            ));
        }
        destination.copy_from_slice(self.source);
        Ok(())
    }
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        palette: IndexedMetadataRef<'_>,
    ) -> Result<IndexedImage, Failure> {
        if self.fail_complete {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Output,
            ));
        }
        Ok(IndexedImage {
            indices: ImageBuf::<PaletteIndex8>::from_vec_packed(indices.to_vec(), dimensions)
                .unwrap(),
            palette: palette.palette.clone(),
            warnings: palette.warnings.to_vec(),
        })
    }
}

fn boundary(source: &[u8]) -> Boundary<'_> {
    Boundary {
        source,
        copies: 0,
        fail_copy: false,
        fail_complete: false,
    }
}
fn request(palette: &[PaletteEntry]) -> QuantizeRequest<'_> {
    QuantizeRequest {
        source_width: 2,
        source_height: 1,
        palette,
        alpha: AlphaPolicy::Preserve {
            threshold: 127.9999999,
        },
        matching: MatchPolicy::SrgbEuclidean,
    }
}

#[test]
fn five_spaces_and_alpha_policies_match_complete_frozen_results() {
    let source = [255, 2, 3, 128, 17, 31, 53, 0];
    let palette = [
        PaletteEntry::Color { rgb: [255, 0, 0] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [0, 0, 0] },
    ];
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    for matching in [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::YcbcrEuclidean,
    ] {
        for alpha in [
            AlphaPolicy::Preserve {
                threshold: 127.9999999,
            },
            AlphaPolicy::Premultiplied {},
            AlphaPolicy::Matte {
                rgb: [255, 255, 255],
            },
        ] {
            let request = QuantizeRequest {
                matching,
                alpha,
                ..request(&palette)
            };
            let actual = processor.quantize(request, &mut boundary(&source)).unwrap();
            let expected = spec::quantize::quantize(spec::contract::request::QuantizeRequest {
                version: 1,
                source: spec::contract::request::Source {
                    width: 2,
                    height: 1,
                    data: &source,
                },
                palette: &palette,
                matching: serde_json::from_value(serde_json::to_value(matching).unwrap()).unwrap(),
                alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap()).unwrap(),
            })
            .unwrap();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn mandatory_capacity_succeeds_one_under_stops_after_snapshot_and_recovers() {
    let source = [255, 2, 3, 128, 17, 31, 53, 0];
    let palette = [PaletteEntry::Color { rgb: [0, 0, 0] }; 257];
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    let expected = processor
        .quantize(request(&palette), &mut boundary(&source))
        .unwrap();
    let capacity = budget_support::minimum(processor.peak_capacity_bytes(), |limit| {
        Processor::new(limit, 0)
            .and_then(|mut processor| processor.quantize(request(&palette), &mut boundary(&source)))
            .is_ok()
    });
    let mut exact = Processor::new(capacity, 0).unwrap();
    assert_eq!(
        exact
            .quantize(request(&palette), &mut boundary(&source))
            .unwrap(),
        expected
    );
    let mut under = Processor::new(capacity - 1, 0).unwrap();
    let mut input = boundary(&source);
    assert_eq!(
        under
            .quantize(request(&palette), &mut input)
            .unwrap_err()
            .code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(input.copies, 1);
    for (fail_copy, fail_complete) in [(true, false), (false, true)] {
        let mut input = Boundary {
            fail_copy,
            fail_complete,
            ..boundary(&source)
        };
        assert_eq!(
            exact
                .quantize(request(&palette), &mut input)
                .unwrap_err()
                .code,
            ErrorCode::WasmMemoryUnavailable
        );
        assert_eq!(
            exact
                .quantize(request(&palette), &mut boundary(&source))
                .unwrap(),
            expected
        );
    }
    exact.dispose().unwrap();
    assert_eq!(
        exact
            .quantize(request(&palette), &mut boundary(&source))
            .unwrap_err()
            .code,
        ErrorCode::Disposed
    );
}

#[test]
fn rgb_memo_preserves_full_results_at_optional_and_mandatory_public_budgets() {
    let source: Vec<u8> = (0..4096u32)
        .flat_map(|n| {
            [
                (n * 73) as u8,
                (n * 31 + n / 256) as u8,
                (n * 17) as u8,
                n as u8,
            ]
        })
        .collect();
    let palette = [
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color { rgb: [73, 31, 211] },
    ];
    let request = QuantizeRequest {
        source_width: 64,
        source_height: 64,
        ..request(&palette)
    };
    let expected = spec::quantize::quantize(spec::contract::request::QuantizeRequest {
        version: 1,
        source: spec::contract::request::Source {
            width: 64,
            height: 64,
            data: &source,
        },
        palette: &palette,
        alpha: serde_json::from_value(serde_json::to_value(request.alpha).unwrap()).unwrap(),
        matching: serde_json::from_value(serde_json::to_value(request.matching).unwrap()).unwrap(),
    })
    .unwrap();
    let mut roomy = Processor::new(1 << 20, 0).unwrap();
    assert_eq!(
        roomy.quantize(request, &mut boundary(&source)).unwrap(),
        expected
    );
    let minimum = budget_support::minimum(roomy.peak_capacity_bytes(), |limit| {
        Processor::new(limit, 0)
            .and_then(|mut processor| processor.quantize(request, &mut boundary(&source)))
            .is_ok()
    });
    // 4,096 pixels select an 8,192-entry table in the roomy call; its ownership is in the peak.
    assert!(roomy.peak_capacity_bytes() >= minimum + 8192 * 8);
    for limit in [minimum, minimum + 8191, minimum + 16384, 1 << 20] {
        let mut processor = Processor::new(limit, 0).unwrap();
        assert_eq!(
            processor.quantize(request, &mut boundary(&source)).unwrap(),
            expected
        );
        assert!(processor.peak_capacity_bytes() <= limit);
        let mut failing = boundary(&source);
        failing.fail_complete = true;
        assert!(processor.quantize(request, &mut failing).is_err());
        assert_eq!(
            processor.quantize(request, &mut boundary(&source)).unwrap(),
            expected
        );
        assert!(processor.peak_capacity_bytes() <= limit);
    }
    assert!(Processor::new(minimum - 1, 0)
        .unwrap()
        .quantize(request, &mut boundary(&source))
        .is_err());
}

#[test]
fn source_and_index_reservation_failures_respect_snapshot_and_never_publish() {
    struct FailAt {
        remaining: usize,
    }
    impl Allocator for FailAt {
        fn reserve(&mut self, buffer: &mut Vec<u8>, additional: usize) -> Result<(), Failure> {
            if self.remaining == 0 {
                return Err(Failure::new(
                    ErrorCode::WasmMemoryUnavailable,
                    ErrorPath::Wasm,
                ));
            }
            self.remaining -= 1;
            buffer.try_reserve_exact(additional).unwrap();
            Ok(())
        }
    }
    let palette = [PaletteEntry::Transparent {}];
    for remaining in 0..2 {
        let mut processor = Processor::new(1 << 20, 0).unwrap();
        let mut input = boundary(&[0; 8]);
        assert_eq!(
            processor
                .quantize_with_allocator(request(&palette), &mut input, &mut FailAt { remaining })
                .unwrap_err()
                .code,
            ErrorCode::WasmMemoryUnavailable
        );
        assert_eq!(input.copies, usize::from(remaining > 0));
        processor.quantize(request(&palette), &mut input).unwrap();
    }
}
#[path = "support/budget.rs"]
mod budget_support;
