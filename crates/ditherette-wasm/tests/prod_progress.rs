//! Compare actual production event delivery with the independent frozen lifecycle.

use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageDimensions},
    prod::{
        contract::{
            failure::Failure,
            lifecycle::Progress,
            request::{AlphaPolicy, MatchPolicy},
        },
        pipeline::{
            processor::Processor,
            progress::Callback,
            quantize::{IndexedMetadataRef, QuantizeBoundary, QuantizeRequest},
        },
    },
    spec::contract::lifecycle as frozen,
};

const TIMES: [u64; 7] = [0, 2, 51, 52, 102, 103, 104];

#[derive(Default)]
struct Boundary {
    clock_reads: usize,
    output_ready: bool,
    events: Vec<Progress>,
}
impl Callback for Boundary {
    fn now_ms(&mut self) -> Result<u64, Failure> {
        let now = TIMES[self.clock_reads];
        self.clock_reads += 1;
        Ok(now)
    }
    fn report(&mut self, event: Progress) -> Result<(), ()> {
        if event.stage == ditherette_wasm::prod::contract::lifecycle::Stage::Complete {
            assert!(self.output_ready);
        }
        self.events.push(event);
        Ok(())
    }
}
impl QuantizeBoundary for Boundary {
    type Output = Vec<u8>;
    fn progress(&mut self) -> Option<&mut dyn Callback> {
        Some(self)
    }
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(3 * 4 * 4)
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        destination.fill(255);
        Ok(())
    }
    fn complete(
        &mut self,
        bytes: &[u8],
        _: ImageDimensions,
        _: IndexedMetadataRef<'_>,
    ) -> Result<Vec<u8>, Failure> {
        let result = bytes.to_vec();
        self.output_ready = true;
        Ok(result)
    }
}

#[test]
fn real_quantize_rows_match_frozen_stage_changes_and_fifty_ms_gate() {
    let mut boundary = Boundary::default();
    let mut processor = Processor::new(4 << 20, 0).unwrap();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let result = processor
        .quantize(
            QuantizeRequest {
                source_width: 3,
                source_height: 4,
                palette: &palette,
                alpha: AlphaPolicy::Premultiplied {},
                matching: MatchPolicy::SrgbEuclidean,
            },
            &mut boundary,
        )
        .unwrap();
    assert_eq!(result, [1; 12]);
    assert_eq!(boundary.clock_reads, TIMES.len());

    let mut reference = frozen::InstanceModel::default();
    reference.begin(true).unwrap();
    let mut expected = Vec::new();
    let attempts = [
        (frozen::Stage::Prepare, 0, 1),
        (frozen::Stage::Quantize, 0, 4),
        (frozen::Stage::Quantize, 1, 4),
        (frozen::Stage::Quantize, 2, 4),
        (frozen::Stage::Quantize, 3, 4),
        (frozen::Stage::Quantize, 4, 4),
        (frozen::Stage::Complete, 1, 1),
    ];
    for ((stage, completed, total), now) in attempts.into_iter().zip(TIMES) {
        if stage == frozen::Stage::Complete {
            reference.output_ready().unwrap();
        }
        let event = frozen::Progress {
            stage,
            completed: Some(completed),
            total: Some(total),
        };
        if reference.report(event, now).unwrap() {
            expected.push(event);
            reference.callback_succeeded().unwrap();
        }
    }
    reference.finish().unwrap();
    assert_eq!(
        serde_json::to_value(&boundary.events).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_eq!(boundary.events.len(), 5);
}
