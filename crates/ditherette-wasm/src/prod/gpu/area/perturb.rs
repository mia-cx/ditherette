//! Standalone RGBA fields share the fused indexed field math, before matching
//! or alpha policy. Hidden RGB and every original alpha byte remain intact.

use super::super::{Backend, Fallback, GpuTiling, JobControl};
use super::{indexed, indexed_shader, AreaJob, AreaMemory, Dispatch, Parameters};
use crate::{
    image::{contracts::PaletteEntry, ImageDimensions},
    prod::{
        contract::request::{AlphaPolicy, MatchPolicy, PerturbRequest, Placement, Request},
        palette::PreparedPalette,
    },
};

pub struct PerturbMemory {
    pub required_bytes: u64,
}
impl PerturbMemory {
    pub fn new(dimensions: ImageDimensions) -> Self {
        let area = AreaMemory::new(dimensions);
        let rgba = u64::from(dimensions.width()) * u64::from(dimensions.height()) * 4;
        Self {
            required_bytes: area.required_bytes + 4 * rgba,
        }
    }
}

impl Backend {
    /// Apply independent fields and optional Adaptive placement, returning RGBA8
    /// directly. No palette search, matte or alpha threshold touches these pixels.
    pub async fn perturb_gpu(
        &mut self,
        request: PerturbRequest<'_>,
        tiling: GpuTiling,
        memory_limit: u64,
    ) -> Result<Vec<u8>, Fallback> {
        self.perturb_checked(request, tiling, memory_limit, None, false, &mut || Ok(()))
            .await
            .map(|(rgba, _)| rgba)
    }

    pub(crate) async fn perturb_checked(
        &mut self,
        request: PerturbRequest<'_>,
        tiling: GpuTiling,
        memory_limit: u64,
        incoming: Option<Vec<u8>>,
        retain: bool,
        check: &mut dyn JobControl,
    ) -> Result<(Vec<u8>, Option<Vec<u8>>), Fallback> {
        check.check()?;
        let layout = Request::Perturb(request)
            .validate()
            .map_err(|e| Fallback::Failed(e.to_string()))?;
        if PerturbMemory::new(layout.output).required_bytes > memory_limit {
            return Err(Fallback::MemoryLimit);
        }
        if matches!(tiling, GpuTiling::Rows(0 | 32769..)) {
            return Err(Fallback::Failed("invalid tile height".into()));
        }
        if request.perturb.strength > 4.0 {
            return Err(Fallback::UnsupportedStage);
        }
        let (sample, tolerance, adaptive) = match request.perturb.placement {
            Placement::Everywhere {} => (4, 0.0, false),
            Placement::SmoothAreaV1 {
                sample_size,
                contrast_threshold,
            } if sample_size <= 100 => (sample_size, contrast_threshold, true),
            _ => return Err(Fallback::UnsupportedStage),
        };
        let palette = PreparedPalette::try_new(
            &[PaletteEntry::Color { rgb: [0; 3] }],
            AlphaPolicy::Preserve { threshold: 0.0 },
            memory_limit,
        )
        .map_err(|e| Fallback::Failed(format!("palette preparation: {e:?}")))?;
        let mut settings = indexed::settings(
            &palette,
            MatchPolicy::SrgbEuclidean,
            AlphaPolicy::Preserve { threshold: 0.0 },
            Some(request.perturb.field),
            request.perturb.space,
            request.perturb.strength,
            adaptive,
        );
        let reused = adaptive && incoming.is_some();
        if reused {
            settings[6 * 4..7 * 4].copy_from_slice(&2u32.to_le_bytes());
        }
        let result = match self {
            Self::Cpu(reason) => Err(reason.clone()),
            Self::Gpu(gpu) => {
                gpu.scoped(async {
                    let mut job = AreaJob::new_with_output(
                        gpu,
                        layout.output,
                        request.source.data,
                        adaptive && !reused,
                        true,
                    )?;
                    job.settings = settings;
                    if let Some(bits) = incoming {
                        job.coverage = bits;
                    }
                    let p = Parameters::new(layout.output, sample, tolerance);
                    let mut coverage = None;
                    if adaptive && !reused {
                        job.analysis(p);
                        if retain {
                            let bytes = AreaMemory::new(layout.output).coverage_bytes;
                            job.dispatches.push(Dispatch {
                                shader: super::PACK,
                                entry: "pack",
                                parameters: p,
                                groups: [(bytes / 4).div_ceil(64) as u32, 1],
                            });
                            coverage =
                                Some(job.execute_read_checked(&job.work, bytes, check).await?);
                            job.dispatches.clear();
                        }
                    }
                    job.indexed_dispatches(
                        indexed_shader::PERTURB,
                        "perturb",
                        p,
                        tiling,
                        u32::MAX,
                        1,
                    );
                    let bytes =
                        u64::from(layout.output.width()) * u64::from(layout.output.height()) * 4;
                    let rgba = job.execute_read_checked(&job.work, bytes, check).await?;
                    Ok((rgba, coverage))
                })
                .await
            }
        };
        self.retire_failure(&result);
        result
    }
}
