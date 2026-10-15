//! Proves area placement preserves real render arithmetic, capacity and cancellation contracts.
use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageDimensions},
    prod::{
        contract::{failure::Failure, request::*},
        pipeline::{
            processor::{InputBoundary, Processor},
            quantize::{IndexedMetadataRef, QuantizeBoundary, QuantizeRequest},
        },
    },
};

struct Boundary<'a>(&'a [u8]);
impl InputBoundary for Boundary<'_> {
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.0.len())
    }
    fn copy_input(&mut self, bytes: &mut [u8]) -> Result<(), Failure> {
        bytes.copy_from_slice(self.0);
        Ok(())
    }
}
impl QuantizeBoundary for Boundary<'_> {
    type Output = Vec<u8>;
    fn complete(
        &mut self,
        indices: &[u8],
        _: ImageDimensions,
        _: IndexedMetadataRef<'_>,
    ) -> Result<Vec<u8>, Failure> {
        Ok(indices.to_vec())
    }
}
impl ditherette_wasm::prod::pipeline::processor::Boundary for Boundary<'_> {
    type Output = Vec<u8>;
    fn complete(&mut self, rgba: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        Ok(rgba.to_vec())
    }
}

const WIDTH: u32 = 96;
const HEIGHT: u32 = 64;
fn render(data: &[u8], dither: DitherPolicy) -> Vec<u8> {
    let palette = [64, 96, 160, 192].map(|value| PaletteEntry::Color { rgb: [value; 3] });
    Processor::new(1 << 24, 0)
        .unwrap()
        .dither_and_quantize(
            QuantizeRequest {
                source_width: WIDTH,
                source_height: HEIGHT,
                palette: &palette,
                alpha: AlphaPolicy::Premultiplied {},
                matching: MatchPolicy::OklabEuclidean,
            },
            dither,
            &mut Boundary(data),
        )
        .unwrap()
}

#[test]
fn extended_render_matches_reference_across_spaces_alpha_and_feedback() {
    use ditherette_wasm::prod::pipeline::perturb::PerturbRequest;
    use ditherette_wasm::{image::ImageView, spec};
    let width = 64;
    let height = 9;
    let data: Vec<_> = (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                let value = if x < 48 { 64 + x * 2 } else { 80 };
                [
                    value as u8,
                    (value + 16) as u8,
                    (208 - value) as u8,
                    (x * 37 + y * 13) as u8,
                ]
            })
        })
        .collect();
    let dimensions = ImageDimensions::new(width, height).unwrap();
    let source = ImageView::packed(&data, dimensions).unwrap();
    let entries = [64, 96, 160, 192].map(|v| PaletteEntry::Color { rgb: [v; 3] });
    let placement = Placement::SmoothAreaV1 {
        sample_size: 24,
        contrast_threshold: 0.2,
    };
    let mut processor = Processor::new(1 << 24, 0).unwrap();
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Oklch,
        WorkingSpace::Cielab,
        WorkingSpace::Cielch,
        WorkingSpace::Ycbcr,
    ] {
        let ref_space = serde_json::from_value(serde_json::to_value(space).unwrap()).unwrap();
        let policy = PerturbPolicy {
            field: Field::Bayer {
                size: BayerSize::Sixteen,
            },
            space,
            strength: 0.3,
            placement,
        };
        let expected =
            spec::smooth_area_v1::render::perturb(source, ref_space, 0.3, 24, 0.2, |x, y, _| {
                spec::dither::ordered::bayer_noise_at(
                    x,
                    y,
                    spec::dither::ordered::BayerSize::Sixteen,
                )
            });
        let actual = processor
            .perturb(
                PerturbRequest {
                    source_width: width,
                    source_height: height,
                    perturb: policy,
                },
                &mut Boundary(&data),
            )
            .unwrap();
        assert_eq!(actual, expected.data(), "{space:?} RGBA8 boundary");
    }
    for matching in [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchHueArc,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::YcbcrEuclidean,
    ] {
        for alpha in [
            AlphaPolicy::Premultiplied {},
            AlphaPolicy::Preserve { threshold: 127.5 },
            AlphaPolicy::Matte { rgb: [32, 80, 144] },
        ] {
            for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
                let quantize = QuantizeRequest {
                    source_width: width,
                    source_height: height,
                    palette: &entries,
                    alpha,
                    matching,
                };
                let dither = DitherPolicy::Diffusion {
                    kernel: Diffusion::Sierra,
                    feedback,
                    strength: 0.3,
                    serpentine: true,
                    placement,
                };
                let reference = spec::smooth_area_v1::render::diffuse(
                    spec::contract::request::QuantizeRequest {
                        version: 1,
                        source: spec::contract::request::Source {
                            width,
                            height,
                            data: &data,
                        },
                        palette: &entries,
                        alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap())
                            .unwrap(),
                        matching: serde_json::from_value(serde_json::to_value(matching).unwrap())
                            .unwrap(),
                    },
                    spec::dither_modes::diffusion::DiffusionPolicy {
                        kernel: spec::dither_modes::diffusion::Kernel::Sierra,
                        strength: 0.3,
                        placement: spec::contract::request::Placement::Everywhere {},
                        serpentine: true,
                        feedback: serde_json::from_value(serde_json::to_value(feedback).unwrap())
                            .unwrap(),
                    },
                    24,
                    0.2,
                )
                .unwrap();
                let actual = processor
                    .dither_and_quantize(quantize, dither, &mut Boundary(&data))
                    .unwrap();
                assert_eq!(
                    actual,
                    reference.indices.data(),
                    "{matching:?} {alpha:?} {feedback:?}"
                );
            }
        }
    }
}

#[test]
fn public_area_calls_budget_analysis_cancel_and_invalidate_it_between_sources() {
    use ditherette_wasm::prod::{
        contract::{
            error::ErrorCode,
            lifecycle::{Progress, Stage},
        },
        pipeline::{
            perturb::PerturbRequest, process::ProcessRequest, processor::Boundary as RgbaBoundary,
            progress::Callback,
        },
    };
    #[path = "support/budget.rs"]
    mod budget;
    struct Observed<'a> {
        data: &'a [u8],
        cancel: bool,
        enabled: bool,
        zeros: usize,
        completed: usize,
        clock: u64,
    }
    impl Callback for Observed<'_> {
        fn now_ms(&mut self) -> Result<u64, Failure> {
            self.clock += 60;
            Ok(self.clock)
        }
        fn report(&mut self, event: Progress) -> Result<(), ()> {
            if matches!(event.stage, Stage::Perturb | Stage::DitherAndQuantize)
                && event.completed == Some(0)
            {
                self.zeros += 1;
                if self.cancel && self.zeros == 3 {
                    return Err(());
                }
            }
            Ok(())
        }
    }
    impl InputBoundary for Observed<'_> {
        fn progress(&mut self) -> Option<&mut dyn Callback> {
            if self.enabled {
                Some(self)
            } else {
                None
            }
        }
        fn input_len(&mut self) -> Result<usize, Failure> {
            Ok(self.data.len())
        }
        fn copy_input(&mut self, bytes: &mut [u8]) -> Result<(), Failure> {
            bytes.copy_from_slice(self.data);
            Ok(())
        }
    }
    impl RgbaBoundary for Observed<'_> {
        type Output = Vec<u8>;
        fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
            self.completed += 1;
            Ok(bytes.to_vec())
        }
    }
    impl QuantizeBoundary for Observed<'_> {
        type Output = Vec<u8>;
        fn complete(
            &mut self,
            bytes: &[u8],
            dims: ImageDimensions,
            _: IndexedMetadataRef<'_>,
        ) -> Result<Vec<u8>, Failure> {
            RgbaBoundary::complete(self, bytes, dims)
        }
    }
    fn run(
        processor: &mut Processor,
        boundary: &mut Observed<'_>,
        kind: u32,
        minimum: u32,
    ) -> Result<Vec<u8>, Failure> {
        let placement = Placement::SmoothAreaV1 {
            sample_size: minimum,
            contrast_threshold: 0.2,
        };
        let perturb = PerturbPolicy {
            field: Field::Bayer {
                size: BayerSize::Sixteen,
            },
            space: WorkingSpace::Srgb,
            strength: 0.3,
            placement,
        };
        if kind == 0 {
            return processor.perturb(
                PerturbRequest {
                    source_width: 70,
                    source_height: 12,
                    perturb,
                },
                boundary,
            );
        }
        let dither = if kind % 2 == 1 {
            DitherPolicy::Separable { perturb }
        } else {
            DitherPolicy::Diffusion {
                kernel: Diffusion::FloydSteinberg,
                feedback: DiffusionFeedback::SrgbBytes,
                strength: 0.3,
                serpentine: true,
                placement,
            }
        };
        let palette = [64, 96, 160, 192].map(|v| PaletteEntry::Color { rgb: [v; 3] });
        let alpha = AlphaPolicy::Premultiplied {};
        let matching = MatchPolicy::OklabEuclidean;
        if kind < 3 {
            return processor.dither_and_quantize(
                QuantizeRequest {
                    source_width: 70,
                    source_height: 12,
                    palette: &palette,
                    alpha,
                    matching,
                },
                dither,
                boundary,
            );
        }
        processor.process(
            ProcessRequest {
                source_width: 70,
                source_height: 12,
                palette: &palette,
                recipe: RecipeV1 {
                    version: 1,
                    output: Output {
                        width: 35,
                        height: 6,
                        resize: ResizePolicy::Nearest {
                            anchor: Anchor::Center,
                        },
                    },
                    alpha,
                    matching,
                    dither,
                },
            },
            boundary,
        )
    }
    let data: Vec<_> = (0..12)
        .flat_map(|_| {
            (0..70).flat_map(|x| {
                let v = if x < 40 { 80 + x as u8 } else { 200 };
                [v, v, v, 255]
            })
        })
        .collect();
    fn observe(data: &[u8]) -> Observed<'_> {
        Observed {
            data,
            cancel: false,
            enabled: false,
            zeros: 0,
            completed: 0,
            clock: 0,
        }
    }
    for kind in 0..5 {
        let mut roomy = Processor::new(1 << 20, 0).unwrap();
        let expected = run(&mut roomy, &mut observe(&data), kind, 8).unwrap();
        let peak = roomy.peak_capacity_bytes();
        let minimum = budget::minimum(peak, |limit| {
            Processor::new(limit, 0)
                .and_then(|mut p| run(&mut p, &mut observe(&data), kind, 8))
                .is_ok()
        });
        eprintln!(
            "kind{kind} peak{peak} minimum{minimum} analysis{} source{}",
            if kind < 3 {
                (70 * 12 + 3 * 70) * 12
            } else {
                (35 * 6 + 3 * 35) * 12
            },
            data.len()
        );
        let failure = Processor::new(minimum - 1, 0)
            .and_then(|mut p| run(&mut p, &mut observe(&data), kind, 8))
            .unwrap_err();
        assert_eq!(failure.code, ErrorCode::MemoryLimit);
        let mut processor = Processor::new(minimum, 0).unwrap();
        let mut observer = observe(&data);
        observer.enabled = true;
        observer.cancel = true;
        assert_eq!(
            run(&mut processor, &mut observer, kind, 8)
                .unwrap_err()
                .code,
            ErrorCode::Callback
        );
        assert_eq!(observer.zeros, 3, "cancel during selector lines");
        assert_eq!(observer.completed, 0);
        observer.cancel = false;
        assert_eq!(
            run(&mut processor, &mut observer, kind, 8).unwrap(),
            expected
        );
        assert!(processor.peak_capacity_bytes() <= minimum);
        let flat: Vec<_> = (0..70 * 12).flat_map(|_| [80, 80, 80, 255]).collect();
        let actual = run(&mut roomy, &mut observe(&flat), kind, 8).unwrap();
        let fresh = run(
            &mut Processor::new(1 << 20, 0).unwrap(),
            &mut observe(&flat),
            kind,
            32768,
        )
        .unwrap();
        assert_eq!(
            actual, fresh,
            "kind{kind}: no analysis retained across source changes"
        );
        let excluded = run(&mut roomy, &mut observe(&data), kind, 32768).unwrap();
        assert_ne!(
            excluded, expected,
            "kind{kind}: size participates in processing identity"
        );
        assert_eq!(
            run(&mut roomy, &mut observe(&data), kind, 8).unwrap(),
            expected
        );
    }
}

#[test]
fn complete_smooth_areas_preserve_bayer_and_sierra_and_reject_strong_boundaries() {
    let smooth: Vec<_> = (0..HEIGHT)
        .flat_map(|y| {
            (0..WIDTH).flat_map(move |x| {
                let value = 32 + (223 * x / (WIDTH - 1)) as u8;
                [value, value.saturating_sub((y % 2) as u8), value, 255]
            })
        })
        .collect();
    let edge: Vec<_> = (0..HEIGHT)
        .flat_map(|_| {
            (0..WIDTH).flat_map(|x| {
                let v = if x < WIDTH / 2 { 0 } else { 255 };
                [v, v, v, 255]
            })
        })
        .collect();
    for (name, data, qualifies) in [
        ("broad high-contrast ramp", smooth, true),
        ("same-contrast step", edge, false),
    ] {
        for (family, diffusion) in [("Bayer16", false), ("Sierra", true)] {
            let policy = |placement| {
                if diffusion {
                    DitherPolicy::Diffusion {
                        kernel: Diffusion::Sierra,
                        feedback: DiffusionFeedback::Matching,
                        strength: 1.0,
                        serpentine: true,
                        placement,
                    }
                } else {
                    DitherPolicy::Separable {
                        perturb: PerturbPolicy {
                            field: Field::Bayer {
                                size: BayerSize::Sixteen,
                            },
                            space: WorkingSpace::Oklab,
                            strength: 1.0,
                            placement,
                        },
                    }
                }
            };
            let plain = render(&data, DitherPolicy::None {});
            let normal = render(&data, policy(Placement::Everywhere {}));
            let actual = render(
                &data,
                policy(Placement::SmoothAreaV1 {
                    sample_size: 114,
                    contrast_threshold: 0.12,
                }),
            );
            assert_eq!(
                actual,
                if qualifies { normal.clone() } else { plain },
                "{name}/{family}"
            );
            assert_eq!(
                render(
                    &data,
                    policy(Placement::SmoothAreaV1 {
                        sample_size: 114,
                        contrast_threshold: 1.0
                    })
                ),
                normal
            );
        }
    }
}
