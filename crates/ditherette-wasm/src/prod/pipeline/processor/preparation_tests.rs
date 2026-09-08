use super::{Allocator, Boundary, Processor, ResizeRequest};
use crate::image::ImageDimensions;
use crate::prod::contract::{
    error::ErrorCode,
    failure::{ErrorPath, Failure},
};
use crate::{
    image::contracts::PaletteEntry,
    prod::{
        contract::request::*,
        pipeline::quantize::IndexedMetadataRef,
        pipeline::{
            process::ProcessRequest,
            quantize::{QuantizeBoundary, QuantizeRequest},
        },
    },
};

const PALETTE: [PaletteEntry; 2] = [
    PaletteEntry::Color { rgb: [0; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
];

struct Io {
    pixels: Vec<u8>,
    fail: bool,
}
impl Io {
    fn new(width: usize) -> Self {
        Self {
            pixels: vec![255; width * 4],
            fail: false,
        }
    }
}
impl Boundary for Io {
    type Output = Vec<u8>;
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.pixels.len())
    }
    fn copy_input(&mut self, to: &mut [u8]) -> Result<(), Failure> {
        to.copy_from_slice(&self.pixels);
        Ok(())
    }
    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Self::Output, Failure> {
        if self.fail {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Output,
            ));
        }
        Ok(bytes.to_vec())
    }
}
impl QuantizeBoundary for Io {
    type Output = Vec<u8>;
    fn input_len(&mut self) -> Result<usize, Failure> {
        Boundary::input_len(self)
    }
    fn copy_input(&mut self, to: &mut [u8]) -> Result<(), Failure> {
        Boundary::copy_input(self, to)
    }
    fn complete(
        &mut self,
        bytes: &[u8],
        dimensions: ImageDimensions,
        _: IndexedMetadataRef<'_>,
    ) -> Result<Self::Output, Failure> {
        Boundary::complete(self, bytes, dimensions)
    }
}

fn quantize(palette: &[PaletteEntry]) -> QuantizeRequest<'_> {
    QuantizeRequest {
        source_width: 4,
        source_height: 1,
        palette,
        alpha: AlphaPolicy::Premultiplied {},
        matching: MatchPolicy::SrgbEuclidean,
    }
}
fn output(width: u32) -> Output {
    Output {
        width,
        height: 1,
        resize: ResizePolicy::Nearest {
            anchor: Anchor::Center,
        },
    }
}
fn process(palette: &[PaletteEntry]) -> ProcessRequest<'_> {
    ProcessRequest {
        source_width: 4,
        source_height: 1,
        palette,
        recipe: RecipeV1 {
            version: 1,
            output: output(3),
            alpha: AlphaPolicy::Premultiplied {},
            matching: MatchPolicy::SrgbEuclidean,
            dither: DitherPolicy::None {},
        },
    }
}
struct NoAllocation;
impl Allocator for NoAllocation {
    fn reserve(&mut self, _: &mut Vec<u8>, _: usize) -> Result<(), Failure> {
        panic!("warm buffers must not reserve")
    }
}

#[test]
fn changed_sources_reuse_preparation_while_identical_stages_need_no_new_buffers() {
    let mut processor = Processor::new(4 << 20, 0).unwrap();
    let mut io = Io::new(4);
    let durable = processor.process(process(&PALETTE), &mut io).unwrap();
    assert_eq!(durable, [1; 3]);
    assert_eq!(processor.preparation.stats().0, 4);
    assert_eq!(processor.preparation.image_stats().0, 2);
    assert_eq!(
        processor
            .process_with_allocator(process(&PALETTE), &mut io, &mut NoAllocation)
            .unwrap(),
        durable
    );
    let hits = processor.preparation.stats().1;
    io.pixels.fill(0);
    assert_eq!(
        processor.process(process(&PALETTE), &mut io).unwrap(),
        [0; 3]
    );
    assert_eq!(processor.preparation.stats().1, hits + 2);
    assert_eq!(
        processor.quantize(quantize(&PALETTE), &mut io).unwrap(),
        [0; 4]
    );
    assert_eq!(processor.preparation.stats().1, hits + 3);
    let resize = ResizeRequest {
        source_width: 4,
        source_height: 1,
        output: output(3),
    };
    assert_eq!(processor.resize(resize, &mut io).unwrap(), [0; 12]);
    assert_eq!(processor.preparation.stats().1, hits + 4);
    for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
        let dither = DitherPolicy::Diffusion {
            kernel: Diffusion::Sierra,
            feedback,
            strength: 1.0,
            serpentine: true,
            placement: Placement::Everywhere {},
        };
        io.pixels.fill(255);
        assert_eq!(
            processor
                .dither_and_quantize(quantize(&PALETTE), dither, &mut io)
                .unwrap(),
            [1; 4]
        );
        io.pixels.fill(0);
        assert_eq!(
            processor
                .dither_and_quantize(quantize(&PALETTE), dither, &mut io)
                .unwrap(),
            [0; 4]
        );
    }
    let perturb = crate::prod::pipeline::perturb::PerturbRequest {
        source_width: 4,
        source_height: 1,
        perturb: PerturbPolicy {
            field: Field::Bayer {
                size: BayerSize::Two,
            },
            space: WorkingSpace::Srgb,
            strength: 0.0,
            placement: Placement::Everywhere {},
        },
    };
    processor.perturb(perturb, &mut io).unwrap();
    io.pixels.fill(255);
    assert_eq!(processor.perturb(perturb, &mut io).unwrap(), [255; 16]);
    assert_eq!(
        processor.preparation.stats().0 - processor.preparation.image_stats().0,
        2
    );
    assert_eq!(durable, [1; 3]);
    let other = Processor::new(4 << 20, 0).unwrap();
    assert_eq!(other.preparation.stats(), (0, 0, 0, 0, 0));
    processor.dispose().unwrap();
    assert_eq!(processor.preparation.stats(), (0, 0, 0, 0, 0));
    assert_eq!(
        processor
            .quantize(quantize(&PALETTE), &mut io)
            .unwrap_err()
            .code,
        ErrorCode::Disposed
    );
}

#[test]
fn final_copy_failures_drop_pending_and_active_scratch_but_keep_previous_hits() {
    let mut processor = Processor::new(4 << 20, 0).unwrap();
    let mut io = Io::new(4);
    io.fail = true;
    processor.process(process(&PALETTE), &mut io).unwrap_err();
    assert_eq!(processor.preparation.stats(), (0, 0, 4, 0, 0));
    assert_eq!(processor.preparation.image_stats().0, 0);
    io.fail = false;
    processor.process(process(&PALETTE), &mut io).unwrap();
    let retained = processor.preparation.stats().3;
    let reversed = [PALETTE[1], PALETTE[0]];
    io.fail = true;
    processor.process(process(&reversed), &mut io).unwrap_err();
    assert_eq!(processor.preparation.stats(), (4, 1, 10, retained, 0));
    io.fail = false;
    assert_eq!(
        processor.process(process(&reversed), &mut io).unwrap(),
        [0; 3]
    );
    assert_eq!(processor.preparation.stats().0, 6);
    assert_eq!(processor.preparation.stats().2, 12);
}

#[test]
fn entry_limit_is_one_lru_and_hits_update_eviction_order() {
    let mut processor = Processor::new(4 << 20, 0).unwrap();
    let mut io = Io::new(4);
    let request = |width| ResizeRequest {
        source_width: 4,
        source_height: 1,
        output: output(width),
    };
    for width in 2..130 {
        processor.resize(request(width), &mut io).unwrap();
    }
    assert_eq!(processor.preparation.stats().0, 128);
    processor.resize(request(2), &mut io).unwrap();
    processor.quantize(quantize(&PALETTE), &mut io).unwrap();
    assert_eq!(processor.preparation.stats().0, 128);
    let hits = processor.preparation.stats().1;
    processor.resize(request(2), &mut io).unwrap();
    assert_eq!(processor.preparation.stats().1, hits + 1);
    processor.resize(request(3), &mut io).unwrap();
    assert_eq!(processor.preparation.stats().1, hits + 1);
}

#[test]
fn trilinear_scratch_overwrites_changed_source_on_hits_and_after_failure() {
    let mut processor = Processor::new(4 << 20, 0).unwrap();
    let mut io = Io::new(32);
    let request = ResizeRequest {
        source_width: 32,
        source_height: 1,
        output: Output {
            resize: ResizePolicy::Trilinear {
                anchor: Anchor::Center,
            },
            ..output(3)
        },
    };
    assert_eq!(processor.resize(request, &mut io).unwrap(), [255; 12]);
    io.pixels.fill(0);
    assert_eq!(processor.resize(request, &mut io).unwrap(), [0; 12]);
    io.fail = true;
    io.pixels.fill(41);
    processor.resize(request, &mut io).unwrap_err();
    assert_eq!(processor.preparation.stats().4, 0);
    io.fail = false;
    io.pixels.fill(73);
    assert_eq!(processor.resize(request, &mut io).unwrap(), [73; 12]);
    assert_eq!(processor.preparation.stats().1, 3);
}

#[test]
fn actual_cross_method_calls_hit_materialized_stages_for_every_family() {
    let perturb = PerturbPolicy {
        field: Field::Bayer {
            size: BayerSize::Two,
        },
        space: WorkingSpace::Srgb,
        strength: 0.1,
        placement: Placement::Everywhere {},
    };
    for dither in [
        DitherPolicy::None {},
        DitherPolicy::Separable { perturb },
        DitherPolicy::Diffusion {
            kernel: Diffusion::Sierra,
            feedback: DiffusionFeedback::Matching,
            strength: 1.0,
            serpentine: true,
            placement: Placement::Everywhere {},
        },
        DitherPolicy::Yliluoma {
            size: BayerSize::Two,
            placement: Placement::Everywhere {},
        },
    ] {
        let mut processor = Processor::new(4 << 20, 0).unwrap();
        let mut io = Io::new(4);
        let resized = processor
            .resize(
                ResizeRequest {
                    source_width: 4,
                    source_height: 1,
                    output: output(3),
                },
                &mut io,
            )
            .unwrap();
        let mut request = process(&PALETTE);
        request.recipe.dither = dither;
        let hits = processor.preparation.image_stats().1;
        let result = processor.process(request, &mut io).unwrap();
        assert_eq!(processor.preparation.image_stats().1, hits + 1);
        let hits = processor.preparation.image_stats().1;
        let mut returned = Io {
            pixels: resized,
            fail: false,
        };
        let quantize = QuantizeRequest {
            source_width: 3,
            ..quantize(&PALETTE)
        };
        assert_eq!(
            processor
                .dither_and_quantize(quantize, dither, &mut returned)
                .unwrap(),
            result
        );
        assert_eq!(
            processor.preparation.image_stats().1,
            hits + if matches!(dither, DitherPolicy::Separable { .. }) {
                2
            } else {
                1
            }
        );
        processor.preparation.evict_preparation();
        let hits = processor.preparation.image_stats().1;
        let mut mutated = result.clone();
        mutated.fill(99);
        assert_eq!(
            processor
                .dither_and_quantize_with_allocator(
                    quantize,
                    dither,
                    &mut returned,
                    &mut NoAllocation
                )
                .unwrap(),
            result
        );
        assert!(processor.preparation.image_stats().1 > hits);
    }
    let mut processor = Processor::new(4 << 20, 0).unwrap();
    let mut io = Io::new(4);
    let perturbed = processor
        .perturb(
            crate::prod::pipeline::perturb::PerturbRequest {
                source_width: 4,
                source_height: 1,
                perturb,
            },
            &mut io,
        )
        .unwrap();
    let result = processor
        .dither_and_quantize(
            quantize(&PALETTE),
            DitherPolicy::Separable { perturb },
            &mut io,
        )
        .unwrap();
    assert_eq!(processor.preparation.image_stats().1, 1);
    io.pixels = perturbed;
    assert_eq!(
        processor.quantize(quantize(&PALETTE), &mut io).unwrap(),
        result
    );
    assert_eq!(processor.preparation.image_stats().1, 2);
    io.pixels.fill(0);
    assert_eq!(
        processor.quantize(quantize(&PALETTE), &mut io).unwrap(),
        [0; 4]
    );
    assert_eq!(processor.preparation.image_stats().1, 2);
}
