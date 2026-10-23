//! Fused GPU fields and matching. RGBA reconstruction still uses the exact palette.

use super::{AreaJob, AreaMemory, Dispatch, Fallback, JobControl, Parameters};
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

use super::indexed_shader::INDEXED as SHADER;

/// Includes source snapshots and wire storage, all three GPU work planes, immutable
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
    /// Adaptive analysis on the verified device. Diffusion keeps its CPU recurrence
    /// with GPU coverage. Yliluoma searches its palette-mixture tree on GPU; legacy
    /// placement and samples outside 4–100 return `UnsupportedStage` for CPU fallback.
    pub async fn dither_and_quantize(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
    ) -> Result<IndexedImage, Fallback> {
        self.dither_and_quantize_checked(request, memory_limit, &mut || Ok(()))
            .await
    }

    pub(crate) async fn dither_and_quantize_checked(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
        check: &mut dyn JobControl,
    ) -> Result<IndexedImage, Fallback> {
        self.dither_prepared_checked(request, memory_limit, None, false, check)
            .await
            .map(|(image, _)| image)
    }

    /// The package associates these bits with its byte-verified source snapshot.
    /// Pending coverage is published only after completion callbacks succeed.
    pub(crate) async fn dither_prepared_checked(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
        coverage: Option<Vec<u8>>,
        retain: bool,
        check: &mut dyn JobControl,
    ) -> Result<(IndexedImage, Option<Vec<u8>>), Fallback> {
        self.dither_tiled_checked(
            request,
            memory_limit,
            coverage,
            retain,
            super::super::GpuTiling::Auto,
            check,
        )
        .await
    }

    /// Changes independent image row bands while preserving global field phase.
    /// Diffusion has a continuous scan and uses its separate scheduling path.
    pub async fn dither_and_quantize_tiled(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        tiling: super::super::GpuTiling,
        memory_limit: u64,
    ) -> Result<IndexedImage, Fallback> {
        self.dither_tiled_checked(request, memory_limit, None, false, tiling, &mut || Ok(()))
            .await
            .map(|(image, _)| image)
    }

    pub(crate) async fn dither_tiled_checked(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
        coverage: Option<Vec<u8>>,
        retain: bool,
        tiling: super::super::GpuTiling,
        check: &mut dyn JobControl,
    ) -> Result<(IndexedImage, Option<Vec<u8>>), Fallback> {
        if matches!(tiling, super::super::GpuTiling::Rows(0 | 32769..)) {
            return Err(Fallback::Failed("tile height must be 1–32768".into()));
        }
        check.check()?;
        let layout = Request::DitherAndQuantize(request)
            .validate()
            .map_err(|error| Fallback::Failed(error.to_string()))?;
        if let DitherPolicy::Yliluoma { size, placement } = request.dither {
            return self
                .yliluoma(
                    request,
                    size,
                    placement,
                    memory_limit,
                    coverage,
                    retain,
                    tiling,
                    check,
                )
                .await;
        }
        let memory = IndexedMemory::new(layout.output);
        if memory.required_bytes > memory_limit {
            return Err(Fallback::MemoryLimit);
        }
        if let DitherPolicy::Diffusion { placement, .. } = request.dither {
            if matches!(tiling, super::super::GpuTiling::Rows(_)) {
                return Err(Fallback::UnsupportedStage);
            }
            return self
                .diffusion(request, placement, memory_limit, coverage, retain, check)
                .await;
        }
        let (placement, field, space, strength) = match request.dither {
            DitherPolicy::None {} => (Placement::Everywhere {}, None, WorkingSpace::Srgb, 0.0),
            // The editor's byte-space 100% setting is 96/63.75, above one.
            // Larger valid package amplitudes retain ordinary scalar fallback.
            DitherPolicy::Separable { perturb } if perturb.strength <= 4.0 => (
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
        let mut settings = settings(
            &palette,
            request.quantize.matching,
            request.quantize.alpha,
            field,
            space,
            strength,
            adaptive,
        );
        if adaptive && coverage.is_some() {
            settings[6 * 4..7 * 4].copy_from_slice(&2u32.to_le_bytes());
        }
        let result = match self {
            Self::Cpu(reason) => Err(reason.clone()),
            Self::Gpu(gpu) => {
                gpu.scoped(async {
                    let mut job = AreaJob::new_with_analysis(
                        gpu,
                        layout.output,
                        request.quantize.source.data,
                        adaptive && coverage.is_none(),
                    )?;
                    job.settings = settings;
                    let incoming = adaptive && coverage.is_some();
                    if let Some(bits) = coverage {
                        job.coverage = bits;
                    }
                    let parameters = Parameters::new(layout.output, sample_size, tolerance);
                    let mut retained = None;
                    if adaptive && !incoming {
                        job.analysis(parameters);
                        if retain {
                            let bytes = AreaMemory::new(layout.output).coverage_bytes;
                            job.dispatches.push(Dispatch {
                                shader: super::PACK,
                                entry: "pack",
                                parameters,
                                groups: [(bytes / 4).div_ceil(64) as u32, 1],
                            });
                            retained =
                                Some(job.execute_read_checked(&job.work, bytes, check).await?);
                            job.dispatches.clear();
                        }
                    }
                    job.indexed_dispatches(SHADER, "indexed", parameters, tiling, u32::MAX);
                    let mut bytes = job
                        .execute_read_checked(&job.work, memory.index_bytes, check)
                        .await?;
                    bytes.truncate(layout.output.pixel_count().expect("validated dimensions"));
                    let indices = ImageBuf::from_vec_packed(bytes, layout.output)
                        .map_err(|error| Fallback::Failed(error.to_string()))?;
                    Ok((palette.into_indexed(indices), retained))
                })
                .await
            }
        };
        self.retire_failure(&result);
        result
    }
}

impl super::super::Backend {
    async fn diffusion(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        placement: Placement,
        memory_limit: u64,
        incoming: Option<Vec<u8>>,
        retain: bool,
        check: &mut dyn JobControl,
    ) -> Result<(IndexedImage, Option<Vec<u8>>), Fallback> {
        use crate::prod::dither::{
            error_diffusion::prepared::{DiffusionPolicy, PreparedDiffusion},
            smooth_area_v1::coverage::Coverage,
        };
        let source = request.quantize.source;
        let dimensions = crate::image::ImageDimensions::new(source.width, source.height)
            .map_err(|error| Fallback::Failed(error.to_string()))?;
        let reused = incoming.is_some();
        let coverage = match placement {
            Placement::Everywhere {} => None,
            Placement::SmoothAreaV1 {
                sample_size,
                contrast_threshold,
            } if sample_size <= 100 => {
                let bits = if let Some(bits) = incoming {
                    bits
                } else {
                    self.adaptive_coverage_checked(
                        dimensions,
                        source.data,
                        sample_size,
                        contrast_threshold,
                        memory_limit,
                        check,
                    )
                    .await?
                };
                Some(Coverage::from_gpu(dimensions, bits))
            }
            _ => return Err(Fallback::UnsupportedStage),
        };
        // All transient GPU planes have been released before CPU rows and
        // indices are allocated. The retained device is part of the fixed reserve.
        let count = dimensions.pixel_count().expect("validated dimensions");
        let retained = AreaMemory::FIXED_BYTES
            + count as u64 * 10
            + 3 * coverage.as_ref().map_or(0, Coverage::capacity_bytes);
        let available = memory_limit
            .checked_sub(retained)
            .ok_or(Fallback::MemoryLimit)?;
        let mut prepared = PreparedDiffusion::try_new(
            source.width,
            request.quantize.palette,
            request.quantize.alpha,
            request.quantize.matching,
            available,
        )
        .map_err(|_| Fallback::MemoryLimit)?;
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(count)
            .map_err(|_| Fallback::MemoryLimit)?;
        indices.resize(count, 0);
        check.check()?;
        let mut callback_error = None;
        prepared
            .execute_gpu_coverage(
                crate::image::ImageView::packed(source.data, dimensions).expect("validated source"),
                &mut indices,
                DiffusionPolicy::new(request.dither)
                    .map_err(|error| Fallback::Failed(format!("{error:?}")))?,
                coverage.as_ref(),
                |row| {
                    check.row(row).map_err(|error| {
                        callback_error = Some(error);
                        crate::prod::contract::failure::Failure::new(
                            crate::prod::contract::error::ErrorCode::Callback,
                            crate::prod::contract::failure::ErrorPath::OnProgress,
                        )
                    })
                },
            )
            .map_err(|error| {
                callback_error.unwrap_or_else(|| Fallback::Failed(format!("diffusion: {error:?}")))
            })?;
        check.check()?;
        let retained = if retain && !reused {
            coverage.map(Coverage::into_bits)
        } else {
            None
        };
        Ok((
            prepared.into_indexed(
                ImageBuf::from_vec_packed(indices, dimensions).expect("validated indices"),
            ),
            retained,
        ))
    }
}

pub(super) fn size(size: BayerSize) -> u32 {
    match size {
        BayerSize::Two => 2,
        BayerSize::Four => 4,
        BayerSize::Eight => 8,
        BayerSize::Sixteen => 16,
    }
}

pub(super) fn settings(
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
