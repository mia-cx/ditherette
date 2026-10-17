//! Fused GPU fields and matching. RGBA reconstruction still uses the exact palette.

use super::{AreaJob, AreaMemory, Dispatch, Fallback, Parameters};
use crate::{
    image::{contracts::IndexedImage, ImageBuf},
    prod::{
        color::packed::{Converter, PackedSpace},
        contract::request::{
            AlphaPolicy, BayerSize, DitherPolicy, DitherQuantizeRequest, Field, MatchPolicy,
            Placement, Request, WorkingSpace,
        },
        dither::placement::coordinate_domain,
        palette::PreparedPalette,
    },
};

const SHADER: &str = include_str!("indexed.wgsl");

/// Includes source snapshots and wire storage, both GPU work planes, immutable
/// dispatch uniforms, packed index staging and independent host/JS output.
#[derive(Debug, Clone, Copy)]
pub struct IndexedMemory {
    pub required_bytes: u64,
    pub index_bytes: u64,
}
impl IndexedMemory {
    pub fn new(dimensions: crate::image::ImageDimensions) -> Self {
        let area = AreaMemory::new(dimensions);
        let pixels = u64::from(dimensions.width()) * u64::from(dimensions.height());
        let index_bytes = pixels.div_ceil(4) * 4;
        Self {
            required_bytes: area.required_bytes - 4 * area.coverage_bytes + 4 * index_bytes,
            index_bytes,
        }
    }
}

impl super::super::Backend {
    /// Runs independent fields, all matching metrics and optional complete
    /// Adaptive analysis on the verified device. Diffusion, Yliluoma, legacy
    /// placement and samples outside 4–100 return `UnsupportedStage` for CPU fallback.
    pub async fn dither_and_quantize(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
    ) -> Result<IndexedImage, Fallback> {
        let layout = Request::DitherAndQuantize(request)
            .validate()
            .map_err(|error| Fallback::Failed(error.to_string()))?;
        let memory = IndexedMemory::new(layout.output);
        if memory.required_bytes > memory_limit {
            return Err(Fallback::MemoryLimit);
        }
        let (placement, field, space, strength) = match request.dither {
            DitherPolicy::None {} => (Placement::Everywhere {}, None, WorkingSpace::Srgb, 0.0),
            DitherPolicy::Separable { perturb } if perturb.strength <= 1.0 => (
                perturb.placement,
                Some(perturb.field),
                perturb.space,
                perturb.strength,
            ),
            _ => return Err(Fallback::UnsupportedStage),
        };
        let (sample_size, tolerance, adaptive) = match placement {
            Placement::Everywhere {} => (4, 0.0, false),
            Placement::SmoothAreaV1 {
                sample_size,
                contrast_threshold,
            } if sample_size <= 100 => (sample_size, contrast_threshold, true),
            _ => return Err(Fallback::UnsupportedStage),
        };
        let palette = PreparedPalette::try_new(
            request.quantize.palette,
            request.quantize.alpha,
            memory_limit,
        )
        .map_err(|error| Fallback::Failed(format!("palette preparation: {error:?}")))?;
        let settings = settings(
            &palette,
            request.quantize.matching,
            request.quantize.alpha,
            field,
            space,
            strength,
            adaptive,
        );
        let result = match self {
            Self::Cpu(reason) => Err(reason.clone()),
            Self::Gpu(gpu) => {
                gpu.scoped(async {
                    let mut job = AreaJob::new(gpu, layout.output, request.quantize.source.data)?;
                    job.settings = settings;
                    let parameters = Parameters::new(layout.output, sample_size, tolerance);
                    if adaptive {
                        job.analysis(parameters);
                    }
                    job.dispatches.push(Dispatch {
                        shader: SHADER,
                        entry: "indexed",
                        parameters,
                        groups: [(memory.index_bytes / 4).div_ceil(64) as u32, 1],
                    });
                    let mut bytes = job.execute_read(&job.work, memory.index_bytes).await?;
                    bytes.truncate(layout.output.pixel_count().expect("validated dimensions"));
                    let indices = ImageBuf::from_vec_packed(bytes, layout.output)
                        .map_err(|error| Fallback::Failed(error.to_string()))?;
                    Ok(palette.into_indexed(indices))
                })
                .await
            }
        };
        self.retire_failure(&result);
        result
    }
}

fn size(size: BayerSize) -> u32 {
    match size {
        BayerSize::Two => 2,
        BayerSize::Four => 4,
        BayerSize::Eight => 8,
        BayerSize::Sixteen => 16,
    }
}

fn settings(
    palette: &PreparedPalette,
    matching: MatchPolicy,
    alpha: AlphaPolicy,
    field: Option<Field>,
    space: WorkingSpace,
    strength: f32,
    adaptive: bool,
) -> Vec<u8> {
    let (kind, argument, seed) = match field {
        None => (0, 0, 0),
        Some(Field::Bayer { size: value }) => (1, size(value), 0),
        Some(Field::Ordered { tile }) => (2, tile as u32, 0),
        Some(Field::BlueNoise {}) => (3, 0, 0),
        Some(Field::Random { seed }) => (4, 0, seed),
    };
    let (alpha_mode, threshold, matte) = match alpha {
        AlphaPolicy::Preserve { threshold } => (0, threshold as u32, 0),
        AlphaPolicy::Premultiplied {} => (1, 0, 0),
        AlphaPolicy::Matte { rgb: [r, g, b] } => (2, 0, u32::from_le_bytes([r, g, b, 0])),
    };
    let threshold_index = match palette.prepare_pixel([0, 0, 0, 0]) {
        crate::prod::palette::PalettePixel::Index(index) => u32::from(index),
        _ => 0,
    };
    let ranges = coordinate_domain(space).ranges();
    // These ordinal tags are local to this shader contract; exhaustive tests
    // traverse all public matching/working policies before a tag can move.
    let mut words = vec![
        matching as u32,
        space as u32,
        kind,
        argument,
        seed,
        strength.to_bits(),
        adaptive as u32,
        palette.visible.len() as u32,
        threshold_index,
        alpha_mode,
        threshold,
        matte,
        ranges[0].to_bits(),
        ranges[1].to_bits(),
        ranges[2].to_bits(),
        0,
    ];
    let converter =
        Converter::new(PackedSpace::from_matching(matching).expect("all policies convert"));
    for entry in &palette.visible {
        let coordinates = converter.coordinates(entry.rgb);
        words.extend([
            u32::from(entry.index),
            u32::from_le_bytes([entry.rgb[0], entry.rgb[1], entry.rgb[2], 0]),
            coordinates[0].to_bits(),
            coordinates[1].to_bits(),
            coordinates[2].to_bits(),
            0,
            0,
            0,
        ]);
    }
    words.into_iter().flat_map(u32::to_le_bytes).collect()
}
