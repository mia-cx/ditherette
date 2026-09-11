//! Optional adaptive work must preserve complete public calls and recover after cancellation.

#[path = "support/budget.rs"]
mod budget_support;

use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageDimensions},
    prod::{
        contract::{
            error::ErrorCode,
            failure::Failure,
            lifecycle::{Progress, Stage},
            request::*,
        },
        pipeline::{
            perturb::PerturbRequest,
            process::ProcessRequest,
            processor::{Boundary as RgbaBoundary, Processor},
            progress::Callback,
            quantize::{IndexedMetadataRef, QuantizeBoundary, QuantizeRequest},
        },
    },
};

const PALETTE: [PaletteEntry; 3] = [
    PaletteEntry::Color { rgb: [0; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
    PaletteEntry::Transparent {},
];

struct Boundary<'a> {
    source: &'a [u8],
    enabled: bool,
    cancel: bool,
    clock: u64,
    completions: usize,
}

impl Callback for Boundary<'_> {
    fn now_ms(&mut self) -> Result<u64, Failure> {
        self.clock += 60;
        Ok(self.clock)
    }
    fn report(&mut self, event: Progress) -> Result<(), ()> {
        if self.cancel
            && matches!(event.stage, Stage::Perturb | Stage::DitherAndQuantize)
            && event.completed == Some(2)
        {
            return Err(());
        }
        Ok(())
    }
}
impl RgbaBoundary for Boundary<'_> {
    type Output = Vec<u8>;
    fn progress(&mut self) -> Option<&mut dyn Callback> {
        if self.enabled {
            Some(self)
        } else {
            None
        }
    }
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.source.len())
    }
    fn copy_input(&mut self, bytes: &mut [u8]) -> Result<(), Failure> {
        bytes.copy_from_slice(self.source);
        Ok(())
    }
    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        self.completions += 1;
        Ok(bytes.to_vec())
    }
}
impl QuantizeBoundary for Boundary<'_> {
    type Output = Vec<u8>;
    fn progress(&mut self) -> Option<&mut dyn Callback> {
        RgbaBoundary::progress(self)
    }
    fn input_len(&mut self) -> Result<usize, Failure> {
        RgbaBoundary::input_len(self)
    }
    fn copy_input(&mut self, bytes: &mut [u8]) -> Result<(), Failure> {
        RgbaBoundary::copy_input(self, bytes)
    }
    fn complete(
        &mut self,
        bytes: &[u8],
        dimensions: ImageDimensions,
        _: IndexedMetadataRef<'_>,
    ) -> Result<Vec<u8>, Failure> {
        RgbaBoundary::complete(self, bytes, dimensions)
    }
}

fn run(
    processor: &mut Processor,
    boundary: &mut Boundary<'_>,
    kind: usize,
    radius: u32,
) -> Result<Vec<u8>, Failure> {
    let placement = Placement::Adaptive {
        radius,
        threshold: 5.0,
        softness: 10.0,
    };
    let perturb = PerturbPolicy {
        field: Field::Random { seed: 71 },
        space: WorkingSpace::Oklab,
        strength: 0.75,
        placement,
    };
    if kind == 0 {
        return processor.perturb(
            PerturbRequest {
                source_width: 7,
                source_height: 9,
                perturb,
            },
            boundary,
        );
    }
    let dither = if kind % 2 == 1 {
        DitherPolicy::Separable { perturb }
    } else {
        DitherPolicy::Diffusion {
            kernel: Diffusion::Sierra,
            feedback: DiffusionFeedback::SrgbBytes,
            strength: 0.75,
            serpentine: true,
            placement,
        }
    };
    let alpha = AlphaPolicy::Preserve {
        threshold: 127.9999999,
    };
    let matching = MatchPolicy::OklabEuclidean;
    if kind < 3 {
        return processor.dither_and_quantize(
            QuantizeRequest {
                source_width: 7,
                source_height: 9,
                palette: &PALETTE,
                alpha,
                matching,
            },
            dither,
            boundary,
        );
    }
    processor.process(
        ProcessRequest {
            source_width: 7,
            source_height: 9,
            palette: &PALETTE,
            recipe: RecipeV1 {
                version: 1,
                output: Output {
                    width: 4,
                    height: 5,
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

#[test]
fn adaptive_public_calls_use_spare_capacity_and_recover_after_callback_failure() {
    let data: Vec<_> = (0..7 * 9 * 4).map(|n| (n * 73 + 17) as u8).collect();
    let boundary = || Boundary {
        source: &data,
        enabled: false,
        cancel: false,
        clock: 0,
        completions: 0,
    };
    for kind in 0..5 {
        for radius in [1, 2, 20] {
            let mut roomy = Processor::new(1 << 20, 0).unwrap();
            let expected = run(&mut roomy, &mut boundary(), kind, radius).unwrap();
            let minimum = budget_support::minimum(roomy.peak_capacity_bytes(), |limit| {
                Processor::new(limit, 0)
                    .and_then(|mut processor| run(&mut processor, &mut boundary(), kind, radius))
                    .is_ok()
            });
            assert!(
                roomy.peak_capacity_bytes() >= minimum + if kind < 3 { 7 * 36 } else { 4 * 36 }
            );
            for limit in [minimum, 1 << 20] {
                let mut processor = Processor::new(limit, 0).unwrap();
                let mut observer = boundary();
                observer.enabled = true;
                observer.cancel = true;
                assert_eq!(
                    run(&mut processor, &mut observer, kind, radius)
                        .unwrap_err()
                        .code,
                    ErrorCode::Callback
                );
                assert_eq!(observer.completions, 0);
                observer.cancel = false;
                assert_eq!(
                    run(&mut processor, &mut observer, kind, radius).unwrap(),
                    expected
                );
                assert!(processor.peak_capacity_bytes() <= limit);
                // Successful preparation may be reused; fresh coordinate tags must still bind this call.
                assert_eq!(
                    run(&mut processor, &mut observer, kind, radius).unwrap(),
                    expected
                );
            }
        }
    }
}
