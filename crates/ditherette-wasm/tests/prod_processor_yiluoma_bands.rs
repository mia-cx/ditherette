#![cfg(feature = "bench-subjects")]

use ditherette_wasm::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageDimensions, PaletteIndex8,
    },
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            lifecycle::{Progress, Stage},
            request::*,
        },
        pipeline::{
            execution::{ExecutionPolicy, RowBandPolicy},
            process::ProcessRequest,
            processor::Processor,
            progress::Callback,
            quantize::{IndexedMetadataRef, QuantizeBoundary, QuantizeRequest},
        },
        tiling::WorkerBudget,
    },
    spec,
};

#[path = "support/budget.rs"]
mod budget;

const PALETTE: [PaletteEntry; 4] = [
    PaletteEntry::Color { rgb: [0; 3] },
    PaletteEntry::Color { rgb: [128; 3] },
    PaletteEntry::Color { rgb: [64; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
];

struct Boundary<'a> {
    source: &'a [u8],
    events: Vec<Progress>,
    fail_stage: Option<Stage>,
    fail_copy: bool,
    fail_output: bool,
    ready: bool,
    caller: std::thread::ThreadId,
}

impl Callback for Boundary<'_> {
    fn now_ms(&mut self) -> Result<u64, Failure> {
        Ok(self.events.len() as u64 * 51)
    }
    fn report(&mut self, event: Progress) -> Result<(), ()> {
        assert_eq!(std::thread::current().id(), self.caller);
        if event.stage == Stage::Complete {
            assert!(self.ready);
        }
        self.events.push(event);
        if self.fail_stage == Some(event.stage) && event.completed.unwrap_or(0) > 0 {
            return Err(());
        }
        Ok(())
    }
}

impl QuantizeBoundary for Boundary<'_> {
    type Output = IndexedImage;
    fn progress(&mut self) -> Option<&mut dyn Callback> {
        Some(self)
    }
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.source.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
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
        metadata: IndexedMetadataRef<'_>,
    ) -> Result<IndexedImage, Failure> {
        if self.fail_output {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Output,
            ));
        }
        self.ready = true;
        Ok(IndexedImage {
            indices: ImageBuf::<PaletteIndex8>::from_vec_packed(indices.to_vec(), dimensions)
                .unwrap(),
            palette: metadata.palette.clone(),
            warnings: metadata.warnings.to_vec(),
        })
    }
}

fn boundary(source: &[u8]) -> Boundary<'_> {
    Boundary {
        source,
        events: Vec::new(),
        fail_stage: None,
        fail_copy: false,
        fail_output: false,
        ready: false,
        caller: std::thread::current().id(),
    }
}

fn source() -> Vec<u8> {
    (0..21)
        .flat_map(|i| {
            [
                (i * 37) as u8,
                (i * 73) as u8,
                (i * 113) as u8,
                [0, 127, 128, 255][i % 4],
            ]
        })
        .collect()
}

fn recipe() -> RecipeV1 {
    RecipeV1 {
        version: 1,
        output: Output {
            width: 5,
            height: 9,
            resize: ResizePolicy::Nearest {
                anchor: Anchor::Center,
            },
        },
        alpha: AlphaPolicy::Preserve {
            threshold: 127.9999999,
        },
        matching: MatchPolicy::OklchCircularHue,
        dither: DitherPolicy::Yliluoma {
            size: BayerSize::Four,
            placement: Placement::Adaptive {
                radius: 2,
                threshold: 5.0,
                softness: 10.0,
            },
        },
    }
}

fn run(
    processor: &mut Processor,
    process: bool,
    boundary: &mut Boundary<'_>,
) -> Result<IndexedImage, Failure> {
    let recipe = recipe();
    if process {
        return processor.process(
            ProcessRequest {
                source_width: 3,
                source_height: 7,
                palette: &PALETTE,
                recipe,
            },
            boundary,
        );
    }
    processor.dither_and_quantize(
        QuantizeRequest {
            source_width: 3,
            source_height: 7,
            palette: &PALETTE,
            alpha: recipe.alpha,
            matching: recipe.matching,
        },
        recipe.dither,
        boundary,
    )
}

fn processor(limit: u64, workers: u32, height: u32) -> Result<Processor, Failure> {
    let mut processor = Processor::new(limit, 0)?;
    processor.set_execution_policy(ExecutionPolicy {
        mixing: Some(RowBandPolicy {
            height,
            workers: WorkerBudget::new(8),
            active_workers: workers,
        }),
        ..ExecutionPolicy::default()
    })?;
    Ok(processor)
}

fn oracle(source: &[u8], process: bool) -> IndexedImage {
    let recipe: spec::contract::request::RecipeV1 =
        serde_json::from_value(serde_json::to_value(recipe()).unwrap()).unwrap();
    let source = spec::contract::request::Source {
        width: 3,
        height: 7,
        data: source,
    };
    if process {
        return spec::pipeline::process(spec::contract::request::ProcessRequest {
            source,
            palette: &PALETTE,
            recipe,
        })
        .unwrap();
    }
    spec::pipeline::dither_and_quantize(spec::contract::request::DitherQuantizeRequest {
        quantize: spec::contract::request::QuantizeRequest {
            version: 1,
            source,
            palette: &PALETTE,
            alpha: recipe.alpha,
            matching: recipe.matching,
        },
        dither: recipe.dither,
    })
    .unwrap()
}

#[test]
fn complete_calls_match_frozen_metadata_and_share_policy_independent_cache_identity() {
    let bytes = source();
    for process in [false, true] {
        let expected = oracle(&bytes, process);
        for workers in [1, 2, 4, 8] {
            for height in [1, 3, 99] {
                let mut processor = processor(1 << 20, workers, height).unwrap();
                let actual = run(&mut processor, process, &mut boundary(&bytes)).unwrap();
                assert_eq!(actual, expected);
                processor
                    .set_execution_policy(ExecutionPolicy::default())
                    .unwrap();
                let mut warm = boundary(&bytes);
                assert_eq!(run(&mut processor, process, &mut warm).unwrap(), expected);
                assert!(!warm
                    .events
                    .iter()
                    .any(|event| event.stage == Stage::DitherAndQuantize));
                assert_eq!(actual, expected, "earlier owned result stays valid");
            }
        }
    }
}

#[test]
fn failed_copies_joined_callbacks_and_completion_publish_no_cached_output() {
    let bytes = source();
    for process in [false, true] {
        for failure in 0..4 {
            let mut processor = processor(1 << 20, 4, 1).unwrap();
            let mut failed = boundary(&bytes);
            match failure {
                0 => failed.fail_copy = true,
                1 => failed.fail_output = true,
                2 => failed.fail_stage = Some(Stage::DitherAndQuantize),
                _ => failed.fail_stage = Some(Stage::Complete),
            }
            let error = run(&mut processor, process, &mut failed).unwrap_err();
            assert_eq!(
                error.code,
                if failure < 2 {
                    ErrorCode::WasmMemoryUnavailable
                } else {
                    ErrorCode::Callback
                }
            );
            let mut retry = boundary(&bytes);
            assert_eq!(
                run(&mut processor, process, &mut retry).unwrap(),
                oracle(&bytes, process)
            );
            assert!(retry
                .events
                .iter()
                .any(|event| event.stage == Stage::DitherAndQuantize
                    && event.completed.unwrap_or(0) > 0));
            if process {
                assert!(retry
                    .events
                    .iter()
                    .any(|event| event.stage == Stage::Resize));
            }
        }
    }
}

#[test]
fn complete_capacity_boundary_rejects_one_byte_under_before_mixing_and_recovers() {
    let bytes = source();
    for process in [false, true] {
        let mut probe = processor(1 << 20, 4, 1).unwrap();
        run(&mut probe, process, &mut boundary(&bytes)).unwrap();
        let required = budget::minimum(probe.peak_capacity_bytes(), |limit| {
            processor(limit, 4, 1)
                .and_then(|mut p| run(&mut p, process, &mut boundary(&bytes)))
                .is_ok()
        });
        let mut exact = processor(required, 4, 1).unwrap();
        assert_eq!(
            run(&mut exact, process, &mut boundary(&bytes)).unwrap(),
            oracle(&bytes, process)
        );
        let mut below = processor(required - 1, 4, 1).unwrap();
        let mut rejected = boundary(&bytes);
        assert_eq!(
            run(&mut below, process, &mut rejected).unwrap_err().code,
            ErrorCode::MemoryLimit
        );
        assert!(!rejected
            .events
            .iter()
            .any(|event| event.stage == Stage::DitherAndQuantize));
        below
            .set_execution_policy(ExecutionPolicy::default())
            .unwrap();
        assert_eq!(
            run(&mut below, process, &mut boundary(&bytes)).unwrap(),
            oracle(&bytes, process)
        );
    }
}
