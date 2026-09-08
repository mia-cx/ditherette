use ditherette_wasm::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageDimensions, PaletteIndex8,
    },
    prod::{
        contract::{failure::Failure, request::*},
        palette::PreparedPalette,
        pipeline::{process::ProcessRequest, processor::Processor, quantize::QuantizeBoundary},
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
struct Boundary {
    copies: usize,
    completions: usize,
}
impl QuantizeBoundary for Boundary {
    type Output = IndexedImage;
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(SOURCE.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        self.copies += 1;
        destination.copy_from_slice(&SOURCE);
        Ok(())
    }
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        palette: &PreparedPalette,
    ) -> Result<IndexedImage, Failure> {
        self.completions += 1;
        Ok(IndexedImage {
            indices: ImageBuf::<PaletteIndex8>::from_vec_packed(indices.to_vec(), dimensions)
                .unwrap(),
            palette: palette.palette.clone(),
            warnings: palette.warnings.clone(),
        })
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

#[test]
fn process_keeps_frozen_resize_then_dither_bytes_and_metadata_for_every_family() {
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    for dither in [
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
    ] {
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
