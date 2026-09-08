use super::{Boundary, Processor, ResizeRequest};
use crate::image::ImageDimensions;
use crate::prod::contract::{
    error::ErrorCode,
    failure::{ErrorPath, Failure},
    lifecycle::Stage,
};
use crate::{
    image::contracts::PaletteEntry,
    prod::{
        contract::{lifecycle::Progress, request::*},
        pipeline::{
            perturb::PerturbRequest,
            process::ProcessRequest,
            progress::Callback,
            quantize::{IndexedMetadataRef, QuantizeBoundary, QuantizeRequest},
        },
    },
};

struct Io {
    input: Vec<u8>,
    enabled: bool,
    ready: bool,
    fail_copy: bool,
    fail_stage: Option<Stage>,
    fail_after_work: bool,
    time: u64,
    events: Vec<Progress>,
    caller: std::thread::ThreadId,
    metadata: Option<(
        crate::image::contracts::NormalizedPalette,
        Vec<crate::image::contracts::ProcessWarning>,
    )>,
}
impl Io {
    fn new(enabled: bool) -> Self {
        Self {
            input: (0..64).map(|n| (n * 73 + 17) as u8).collect(),
            enabled,
            ready: false,
            fail_copy: false,
            fail_stage: None,
            fail_after_work: false,
            time: 0,
            events: Vec::new(),
            caller: std::thread::current().id(),
            metadata: None,
        }
    }
    fn reset(&mut self) {
        self.ready = false;
        self.events.clear();
    }
}
impl Callback for Io {
    fn now_ms(&mut self) -> Result<u64, Failure> {
        self.time += if self.fail_after_work { 60 } else { 20 };
        Ok(self.time)
    }
    fn report(&mut self, event: Progress) -> Result<(), ()> {
        assert_eq!(std::thread::current().id(), self.caller);
        if event.stage == Stage::Complete {
            assert!(self.ready, "durable copy must precede completion");
        }
        self.events.push(event);
        if self.fail_stage == Some(event.stage)
            && (!self.fail_after_work || event.completed.is_some_and(|completed| completed > 0))
        {
            Err(())
        } else {
            Ok(())
        }
    }
}
impl Boundary for Io {
    type Output = Vec<u8>;
    fn progress(&mut self) -> Option<&mut dyn Callback> {
        if self.enabled {
            Some(self)
        } else {
            None
        }
    }
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.input.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        destination.copy_from_slice(&self.input);
        Ok(())
    }
    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        if self.fail_copy {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Output,
            ));
        }
        let result = bytes.to_vec();
        self.ready = true;
        Ok(result)
    }
}
impl QuantizeBoundary for Io {
    type Output = Vec<u8>;
    fn progress(&mut self) -> Option<&mut dyn Callback> {
        Boundary::progress(self)
    }
    fn input_len(&mut self) -> Result<usize, Failure> {
        Boundary::input_len(self)
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        Boundary::copy_input(self, destination)
    }
    fn complete(
        &mut self,
        bytes: &[u8],
        dimensions: ImageDimensions,
        metadata: IndexedMetadataRef<'_>,
    ) -> Result<Vec<u8>, Failure> {
        self.metadata = Some((metadata.palette.clone(), metadata.warnings.to_vec()));
        Boundary::complete(self, bytes, dimensions)
    }
}

fn band_policy(workers: u32, height: u32) -> crate::prod::pipeline::execution::ExecutionPolicy {
    use crate::prod::{
        pipeline::execution::{ExecutionPolicy, RowBandPolicy},
        tiling::WorkerBudget,
    };
    ExecutionPolicy {
        indexed: Some(RowBandPolicy {
            height,
            workers: WorkerBudget::new(workers),
            active_workers: workers,
        }),
        ..ExecutionPolicy::default()
    }
}

#[test]
fn automatic_field_calls_preserve_scalar_bytes_metadata_and_cache_identity() {
    let check = || {
        let mut palette: Vec<_> = (0..63)
            .map(|n| PaletteEntry::Color {
                rgb: [(n * 73) as u8, (n * 31 + 19) as u8, (n * 17 + 113) as u8],
            })
            .collect();
        palette.push(PaletteEntry::Transparent {});
        let request = QuantizeRequest {
            source_width: 65,
            source_height: 49,
            palette: &palette,
            alpha: AlphaPolicy::Preserve { threshold: 127.5 },
            matching: MatchPolicy::OklabEuclidean,
        };
        let field = PerturbPolicy {
            space: WorkingSpace::Oklab,
            strength: 0.7,
            field: Field::BlueNoise {},
            placement: Placement::Adaptive {
                radius: 2,
                threshold: 0.05,
                softness: 0.025,
            },
        };
        for method in 0..3 {
            let request = if method == 2 {
                QuantizeRequest {
                    source_width: 769,
                    source_height: 513,
                    palette: &palette[48..],
                    matching: MatchPolicy::SrgbEuclidean,
                    ..request
                }
            } else {
                request
            };
            let mut io = Io::new(true);
            io.input = (0..request.source_width * request.source_height * 4)
                .map(|n| (n * 73 + 17) as u8)
                .collect();
            io.fail_after_work = true;
            let run = |processor: &mut Processor, io: &mut Io| {
                if method == 2 {
                    processor.quantize(request, io)
                } else if method == 1 {
                    processor.dither_and_quantize(
                        request,
                        DitherPolicy::Separable { perturb: field },
                        io,
                    )
                } else {
                    processor.perturb(
                        PerturbRequest {
                            source_width: 65,
                            source_height: 49,
                            perturb: field,
                        },
                        io,
                    )
                }
            };
            let mut scalar = Processor::new(32 << 20, 0).unwrap();
            scalar.set_execution_policy(Default::default()).unwrap();
            let expected = run(&mut scalar, &mut io).unwrap();
            let metadata = io.metadata.clone();
            let scalar_peak = scalar.peak_capacity_bytes();
            let mut candidate = Processor::new(32 << 20, 0).unwrap();
            io.reset();
            assert_eq!(run(&mut candidate, &mut io).unwrap(), expected);
            assert_eq!(io.metadata, metadata);
            #[cfg(feature = "threads")]
            assert!(
                candidate.peak_capacity_bytes() > scalar_peak,
                "automatic worker ownership is charged"
            );
            #[cfg(not(feature = "threads"))]
            assert_eq!(candidate.peak_capacity_bytes(), scalar_peak);
            let retained = candidate.preparation.stats().0;
            candidate.set_execution_policy(Default::default()).unwrap();
            io.reset();
            assert_eq!(run(&mut candidate, &mut io).unwrap(), expected);
            assert_eq!(candidate.preparation.stats().0, retained);
            assert_eq!(
                io.events
                    .iter()
                    .map(|event| event.stage)
                    .collect::<Vec<_>>(),
                [Stage::Prepare, Stage::Complete]
            );

            let mut failed = Processor::new(32 << 20, 0).unwrap();
            io.reset();
            io.fail_stage = Some(if method == 2 {
                Stage::Quantize
            } else {
                Stage::Perturb
            });
            assert!(run(&mut failed, &mut io).is_err());
            assert_eq!(failed.preparation.stats().0, 0);
            assert!(!io.ready);
            io.fail_stage = None;
            io.reset();
            assert_eq!(run(&mut failed, &mut io).unwrap(), expected);
        }
    };
    #[cfg(feature = "threads")]
    rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap()
        .install(check);
    #[cfg(not(feature = "threads"))]
    check();
}

#[test]
fn field_band_public_calls_preserve_results_caches_and_callback_transactions() {
    for method in 1..5 {
        let mut scalar = Io::new(true);
        let expected = run(
            &mut Processor::new(4 << 20, 0).unwrap(),
            &mut scalar,
            method,
        )
        .unwrap();
        for workers in [1, 2, 4] {
            for height in [1, 2, 3] {
                let mut processor = Processor::new(4 << 20, 0).unwrap();
                processor
                    .set_execution_policy(band_policy(workers, height))
                    .unwrap();
                let mut io = Io::new(true);
                assert_eq!(run(&mut processor, &mut io, method).unwrap(), expected);
                assert_eq!(io.metadata, scalar.metadata);
                let retained = processor.preparation.stats().0;
                processor.set_execution_policy(Default::default()).unwrap();
                io.reset();
                assert_eq!(run(&mut processor, &mut io, method).unwrap(), expected);
                assert_eq!(
                    io.events
                        .iter()
                        .map(|event| event.stage)
                        .collect::<Vec<_>>(),
                    [Stage::Prepare, Stage::Complete]
                );
                assert_eq!(processor.preparation.stats().0, retained);
                for fail_stage in [Stage::Quantize, Stage::Perturb, Stage::Complete] {
                    if (method == 1 && fail_stage == Stage::Quantize)
                        || (method == 2 && fail_stage == Stage::Perturb)
                    {
                        continue;
                    }
                    let mut processor = Processor::new(4 << 20, 0).unwrap();
                    processor
                        .set_execution_policy(band_policy(workers, height))
                        .unwrap();
                    io.reset();
                    io.fail_stage = Some(fail_stage);
                    assert_eq!(
                        run(&mut processor, &mut io, method),
                        Err(Failure::new(ErrorCode::Callback, ErrorPath::OnProgress))
                    );
                    assert_eq!(processor.preparation.stats().0, 0);
                    io.reset();
                    io.fail_stage = None;
                    assert_eq!(run(&mut processor, &mut io, method).unwrap(), expected);
                    assert_eq!(processor.preparation.image_stats().1, 0);
                }
                let mut processor = Processor::new(4 << 20, 0).unwrap();
                processor
                    .set_execution_policy(band_policy(workers, height))
                    .unwrap();
                io.reset();
                io.fail_stage = Some(if method == 2 {
                    Stage::Quantize
                } else {
                    Stage::Perturb
                });
                io.fail_after_work = true;
                io.time = 0;
                assert_eq!(
                    run(&mut processor, &mut io, method),
                    Err(Failure::new(ErrorCode::Callback, ErrorPath::OnProgress))
                );
                assert!(!io.ready);
                assert_eq!(processor.preparation.stats().0, 0);
                io.fail_stage = None;
                io.fail_after_work = false;
                io.reset();
                io.fail_copy = true;
                assert!(run(&mut processor, &mut io, method).is_err());
                assert_eq!(processor.preparation.stats().0, 0);
                io.fail_copy = false;
                io.reset();
                assert_eq!(run(&mut processor, &mut io, method).unwrap(), expected);
            }
        }
    }
}

#[test]
fn field_band_complete_calls_preflight_exact_capacity_before_processing() {
    for method in 1..5 {
        let execute = |limit| {
            let mut processor = Processor::new(limit, 0).unwrap();
            processor.set_execution_policy(band_policy(4, 1)).unwrap();
            let mut io = Io::new(true);
            let result = run(&mut processor, &mut io, method);
            (processor, io, result)
        };
        let mut lower = Processor::bookkeeping_bytes(0);
        let mut upper = 1 << 20;
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            if execute(middle).2.is_ok() {
                upper = middle;
            } else {
                lower = middle + 1;
            }
        }
        let (processor, _, result) = execute(upper);
        assert!(result.is_ok());
        assert!(processor.peak_capacity_bytes() <= upper);
        let (processor, io, result) = execute(upper - 1);
        assert_eq!(
            result,
            Err(Failure::new(
                ErrorCode::MemoryLimit,
                ErrorPath::MemoryLimitBytes
            ))
        );
        assert!(io.events.iter().all(|event| event.stage == Stage::Prepare));
        assert_eq!(processor.preparation.stats().0, 0);
    }
}

fn run(processor: &mut Processor, io: &mut Io, method: u8) -> Result<Vec<u8>, Failure> {
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let quantize = QuantizeRequest {
        source_width: 4,
        source_height: 4,
        palette: &palette,
        alpha: AlphaPolicy::Premultiplied {},
        matching: MatchPolicy::SrgbEuclidean,
    };
    let output = Output {
        width: 3,
        height: 3,
        resize: ResizePolicy::Bilinear {
            anchor: Anchor::Center,
        },
    };
    let perturb = PerturbPolicy {
        field: Field::Random { seed: 7 },
        space: WorkingSpace::Srgb,
        strength: 0.75,
        placement: Placement::Everywhere {},
    };
    match method {
        0 => processor.resize(
            ResizeRequest {
                source_width: 4,
                source_height: 4,
                output,
            },
            io,
        ),
        1 => processor.perturb(
            PerturbRequest {
                source_width: 4,
                source_height: 4,
                perturb,
            },
            io,
        ),
        2 => processor.quantize(quantize, io),
        3 => processor.dither_and_quantize(quantize, DitherPolicy::Separable { perturb }, io),
        4 => processor.process(
            ProcessRequest {
                source_width: 4,
                source_height: 4,
                palette: &palette,
                recipe: RecipeV1 {
                    version: 1,
                    output,
                    alpha: quantize.alpha,
                    matching: quantize.matching,
                    dither: DitherPolicy::Separable { perturb },
                },
            },
            io,
        ),
        _ => unreachable!(),
    }
}

#[test]
fn all_methods_complete_after_copy_before_publication_and_recover_from_callbacks() {
    for method in 0..5 {
        let mut disabled = Io::new(false);
        let expected = run(
            &mut Processor::new(4 << 20, 0).unwrap(),
            &mut disabled,
            method,
        )
        .unwrap();
        assert_eq!(disabled.time, 0, "disabled calls do not read the clock");
        let mut probe = Io::new(true);
        assert_eq!(
            run(&mut Processor::new(4 << 20, 0).unwrap(), &mut probe, method).unwrap(),
            expected
        );
        let intermediate = probe
            .events
            .iter()
            .find(|event| event.stage != Stage::Prepare && event.stage != Stage::Complete)
            .unwrap()
            .stage;
        for fail_stage in [Stage::Prepare, intermediate, Stage::Complete] {
            let mut processor = Processor::new(4 << 20, 0).unwrap();
            let mut io = Io::new(true);
            io.fail_stage = Some(fail_stage);
            assert_eq!(
                run(&mut processor, &mut io, method),
                Err(Failure::new(ErrorCode::Callback, ErrorPath::OnProgress))
            );
            assert_eq!(
                processor.preparation.stats().0,
                0,
                "failed callbacks publish no preparation or image entries"
            );
            assert_eq!(io.ready, fail_stage == Stage::Complete);
            io.fail_stage = None;
            io.reset();
            assert_eq!(run(&mut processor, &mut io, method).unwrap(), expected);
            assert_eq!(
                processor.preparation.image_stats().1,
                0,
                "recovery cannot hit a failed call's images"
            );
            let retained = processor.preparation.stats().0;
            assert!(retained > 0);
            io.reset();
            io.fail_stage = Some(Stage::Complete);
            assert_eq!(
                run(&mut processor, &mut io, method),
                Err(Failure::new(ErrorCode::Callback, ErrorPath::OnProgress))
            );
            assert!(processor.preparation.image_stats().1 > 0);
            assert_eq!(processor.preparation.stats().0, retained);
            assert_eq!(
                io.events
                    .iter()
                    .map(|event| event.stage)
                    .collect::<Vec<_>>(),
                [Stage::Prepare, Stage::Complete]
            );
            io.fail_stage = None;
            io.reset();
            assert_eq!(run(&mut processor, &mut io, method).unwrap(), expected);
        }
    }
}

#[test]
fn final_copy_failure_never_delivers_completion_or_publishes() {
    for method in 0..5 {
        let mut processor = Processor::new(4 << 20, 0).unwrap();
        let mut io = Io::new(true);
        io.fail_copy = true;
        assert_eq!(
            run(&mut processor, &mut io, method),
            Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Output
            ))
        );
        assert!(io.events.iter().all(|event| event.stage != Stage::Complete));
        assert_eq!(processor.preparation.stats().0, 0);
        io.fail_copy = false;
        io.reset();
        run(&mut processor, &mut io, method).unwrap();
        assert_eq!(io.events.last().unwrap().stage, Stage::Complete);
    }
}
