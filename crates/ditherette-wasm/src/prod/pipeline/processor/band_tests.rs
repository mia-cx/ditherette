use super::{Boundary, Processor, ResizeRequest};
use crate::{
    image::ImageDimensions,
    prod::{
        contract::{
            failure::Failure,
            lifecycle::{Progress, Stage},
            request::*,
        },
        pipeline::{
            execution::{ExecutionPolicy, RowBandPolicy},
            progress::Callback,
        },
        tiling::WorkerBudget,
    },
};

struct Io {
    input: Vec<u8>,
    events: Vec<Progress>,
    caller: std::thread::ThreadId,
    fail: bool,
}

impl Callback for Io {
    fn now_ms(&mut self) -> Result<u64, Failure> {
        Ok(self.events.len() as u64 * 60)
    }
    fn report(&mut self, event: Progress) -> Result<(), ()> {
        assert_eq!(std::thread::current().id(), self.caller);
        self.events.push(event);
        if self.fail && event.stage == Stage::Resize && event.completed.unwrap_or(0) > 0 {
            Err(())
        } else {
            Ok(())
        }
    }
}

impl Boundary for Io {
    type Output = Vec<u8>;
    fn progress(&mut self) -> Option<&mut dyn Callback> {
        Some(self)
    }
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.input.len())
    }
    fn copy_input(&mut self, output: &mut [u8]) -> Result<(), Failure> {
        output.copy_from_slice(&self.input);
        Ok(())
    }
    fn complete(&mut self, output: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        Ok(output.to_vec())
    }
}

impl crate::prod::pipeline::quantize::QuantizeBoundary for Io {
    type Output = Vec<u8>;
    fn progress(&mut self) -> Option<&mut dyn Callback> {
        Boundary::progress(self)
    }
    fn input_len(&mut self) -> Result<usize, Failure> {
        Boundary::input_len(self)
    }
    fn copy_input(&mut self, output: &mut [u8]) -> Result<(), Failure> {
        Boundary::copy_input(self, output)
    }
    fn complete(
        &mut self,
        output: &[u8],
        dimensions: ImageDimensions,
        _: crate::prod::pipeline::quantize::IndexedMetadataRef<'_>,
    ) -> Result<Vec<u8>, Failure> {
        Boundary::complete(self, output, dimensions)
    }
}

#[test]
fn combined_resize_and_indexed_candidates_preserve_complete_process() {
    use crate::{image::contracts::PaletteEntry, prod::pipeline::process::ProcessRequest};
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let request = ProcessRequest {
        source_width: 71,
        source_height: 53,
        palette: &palette,
        recipe: RecipeV1 {
            version: 1,
            output: Output {
                width: 19,
                height: 17,
                resize: ResizePolicy::Lanczos3 {
                    anchor: Anchor::Center,
                    support: Support::ScaleAware,
                },
            },
            alpha: AlphaPolicy::Premultiplied {},
            matching: MatchPolicy::SrgbEuclidean,
            dither: DitherPolicy::None {},
        },
    };
    let mut io = Io {
        input: (0..71 * 53 * 4).map(|n| (n * 73) as u8).collect(),
        events: Vec::new(),
        caller: std::thread::current().id(),
        fail: false,
    };
    let expected = Processor::new(1 << 20, 0)
        .unwrap()
        .process(request, &mut io)
        .unwrap();
    let mut candidate = Processor::new(1 << 20, 0).unwrap();
    let mut execution = policy(3, 4);
    execution.indexed = execution.resize;
    candidate.set_execution_policy(execution).unwrap();
    io.events.clear();
    assert_eq!(candidate.process(request, &mut io).unwrap(), expected);
    assert_eq!(
        io.events
            .iter()
            .filter(|event| event.stage == Stage::Resize)
            .count(),
        3
    );
    assert!(candidate.peak_capacity_bytes() <= 1 << 20);
}

fn policy(height: u32, workers: u32) -> ExecutionPolicy {
    ExecutionPolicy {
        resize: Some(RowBandPolicy {
            height,
            workers: WorkerBudget::new(workers),
            active_workers: workers,
        }),
        ..Default::default()
    }
}

#[test]
fn complete_resize_bands_preserve_scalar_bytes_progress_and_failure_publication() {
    for resize in [
        ResizePolicy::Nearest {
            anchor: Anchor::BottomRight,
        },
        ResizePolicy::Area {},
        ResizePolicy::Bilinear {
            anchor: Anchor::TopLeft,
        },
        ResizePolicy::Bicubic {
            anchor: Anchor::Center,
            support: Support::ScaleAware,
        },
        ResizePolicy::Lanczos2 {
            anchor: Anchor::BottomRight,
            support: Support::Fixed,
        },
        ResizePolicy::Lanczos3 {
            anchor: Anchor::TopLeft,
            support: Support::ScaleAware,
        },
        ResizePolicy::Trilinear {
            anchor: Anchor::Center,
        },
    ] {
        let request = ResizeRequest {
            source_width: 71,
            source_height: 53,
            output: Output {
                width: 19,
                height: 17,
                resize,
            },
        };
        let mut io = Io {
            input: (0..71 * 53 * 4).map(|n| (n * 73 + n / 11) as u8).collect(),
            events: Vec::new(),
            caller: std::thread::current().id(),
            fail: false,
        };
        let mut scalar = Processor::new(1 << 20, 0).unwrap();
        let expected = scalar.resize(request, &mut io).unwrap();
        for workers in [1, 2, 4] {
            let mut candidate = Processor::new(1 << 20, 0).unwrap();
            candidate.set_execution_policy(policy(3, workers)).unwrap();
            io.events.clear();
            io.fail = true;
            assert!(candidate.resize(request, &mut io).is_err());
            assert_eq!(
                candidate.preparation.stats().0,
                0,
                "failed call publishes nothing"
            );
            io.fail = false;
            io.events.clear();
            assert_eq!(
                candidate.resize(request, &mut io).unwrap(),
                expected,
                "{resize:?} workers={workers}"
            );
            assert!(candidate.peak_capacity_bytes() <= 1 << 20);
            let resize_events: Vec<_> = io
                .events
                .iter()
                .filter(|event| event.stage == Stage::Resize)
                .collect();
            assert!(resize_events.len() >= 2);
            if !matches!(resize, ResizePolicy::Trilinear { .. }) {
                assert_eq!(resize_events.len(), 1 + 6usize.div_ceil(workers as usize));
                assert_eq!(resize_events.last().unwrap().total, Some(17));
            }
            assert_eq!(resize_events.first().unwrap().completed, Some(0));
            let final_event = resize_events.last().unwrap();
            assert_eq!(final_event.completed, final_event.total);
            // A different source reuses preparation and worker scratch, while bypassing image hits.
            io.input[0] ^= 93;
            let expected = scalar.resize(request, &mut io).unwrap();
            assert_eq!(candidate.resize(request, &mut io).unwrap(), expected);
            assert!(candidate.preparation.stats().1 > 0);
            io.input[0] ^= 93;
        }
    }
}

#[test]
fn complete_resize_falls_back_under_scalar_budget_and_recovers_after_pressure() {
    let request = ResizeRequest {
        source_width: 101,
        source_height: 79,
        output: Output {
            width: 43,
            height: 37,
            resize: ResizePolicy::Bilinear {
                anchor: Anchor::Center,
            },
        },
    };
    let mut io = Io {
        input: (0..101 * 79 * 4).map(|n| (n * 31) as u8).collect(),
        events: Vec::new(),
        caller: std::thread::current().id(),
        fail: false,
    };
    let mut scalar = Processor::new(1 << 20, 0).unwrap();
    let expected = scalar.resize(request, &mut io).unwrap();
    let scalar_events = std::mem::take(&mut io.events);
    let limit = scalar.peak_capacity_bytes();
    let mut candidate = Processor::new(limit, 0).unwrap();
    candidate.set_execution_policy(policy(1, 8)).unwrap();
    assert_eq!(candidate.resize(request, &mut io).unwrap(), expected);
    assert_eq!(
        io.events, scalar_events,
        "budget rejection executes the scalar work path"
    );
    assert!(candidate.peak_capacity_bytes() <= limit);
    candidate
        .set_execution_policy(ExecutionPolicy::default())
        .unwrap();
    io.input[0] ^= 1;
    assert_eq!(
        candidate.resize(request, &mut io).unwrap(),
        scalar.resize(request, &mut io).unwrap()
    );
}
