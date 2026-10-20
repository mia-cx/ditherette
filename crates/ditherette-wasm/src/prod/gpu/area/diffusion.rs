//! Dependency-ordered GPU diffusion. Ordinary scans use wavefront tiles;
//! serpentine carries a three-row ring continuously through bounded dispatches.

use super::{indexed, AreaJob, Dispatch, Fallback, IndexedMemory, JobControl, Parameters};
use crate::{
    image::{contracts::IndexedImage, ImageBuf, ImageDimensions},
    prod::{
        contract::request::{
            DiffusionFeedback, DitherPolicy, DitherQuantizeRequest, Placement, Request,
            WorkingSpace,
        },
        dither::error_diffusion::ErrorDiffusionKernel,
        palette::PreparedPalette,
    },
};

mod cooperative;
pub(super) mod shader;
mod wavefront;
const PIXELS_PER_DISPATCH: u32 = 1024;

/// Source/wire snapshots, the error plane or serial row rings, packed output
/// and every immutable dispatch record are charged before storage is created.
pub struct DiffusionMemory {
    pub required_bytes: u64,
}
impl DiffusionMemory {
    pub fn new(dimensions: ImageDimensions) -> Self {
        let width = u64::from(dimensions.width());
        let height = u64::from(dimensions.height());
        let dispatches = (height * (width.div_ceil(u64::from(PIXELS_PER_DISPATCH)) + 1) + 3)
            .max((width + 5 * height).div_ceil(16));
        let setup = dispatches * (512 + 2 * std::mem::size_of::<Dispatch>() as u64);
        Self {
            required_bytes: IndexedMemory::new(dimensions).required_bytes + setup + width * 72,
        }
    }
}

impl super::super::Backend {
    /// Executes every kernel with its continuous scan and feedback on GPU.
    /// Serpentine is necessarily serial across pixels; this interface makes no
    /// latency promise relative to the synchronous CPU implementation.
    pub async fn diffuse_gpu(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
    ) -> Result<IndexedImage, Fallback> {
        self.diffuse_gpu_checked(request, memory_limit, None, false, &mut || Ok(()))
            .await
            .map(|(image, _)| image)
    }

    /// Select dependent submission batches without resetting the diffusion ring.
    pub async fn diffuse_gpu_tiled(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
        tiling: super::super::GpuTiling,
    ) -> Result<IndexedImage, Fallback> {
        self.diffuse_gpu_tiled_checked(request, memory_limit, None, false, tiling, &mut || Ok(()))
            .await
            .map(|(image, _)| image)
    }

    pub(crate) async fn diffuse_gpu_checked(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
        incoming: Option<Vec<u8>>,
        retain: bool,
        check: &mut dyn JobControl,
    ) -> Result<(IndexedImage, Option<Vec<u8>>), Fallback> {
        self.diffuse_gpu_scheduled(request, memory_limit, incoming, retain, check, false)
            .await
    }

    pub(super) async fn diffuse_gpu_scheduled(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
        incoming: Option<Vec<u8>>,
        retain: bool,
        check: &mut dyn JobControl,
        force_serial: bool,
    ) -> Result<(IndexedImage, Option<Vec<u8>>), Fallback> {
        self.diffuse_gpu_batched(
            request,
            memory_limit,
            incoming,
            retain,
            check,
            force_serial,
            super::MAX_DISPATCHES_PER_BATCH,
        )
        .await
    }

    /// Dependent tiles share the complete error state. Explicit rows cap queued
    /// dependency frontiers rather than starting independent diffusion images.
    pub(crate) async fn diffuse_gpu_tiled_checked(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
        incoming: Option<Vec<u8>>,
        retain: bool,
        tiling: super::super::GpuTiling,
        check: &mut dyn JobControl,
    ) -> Result<(IndexedImage, Option<Vec<u8>>), Fallback> {
        let batch = match tiling {
            super::super::GpuTiling::Rows(0 | 32769..) => return Err(Fallback::UnsupportedStage),
            super::super::GpuTiling::Rows(rows) => {
                (rows as usize).min(super::MAX_DISPATCHES_PER_BATCH)
            }
            _ => super::MAX_DISPATCHES_PER_BATCH,
        };
        self.diffuse_gpu_batched(request, memory_limit, incoming, retain, check, false, batch)
            .await
    }

    async fn diffuse_gpu_batched(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
        incoming: Option<Vec<u8>>,
        retain: bool,
        check: &mut dyn JobControl,
        force_serial: bool,
        batch_size: usize,
    ) -> Result<(IndexedImage, Option<Vec<u8>>), Fallback> {
        check.check()?;
        let layout = Request::DitherAndQuantize(request)
            .validate()
            .map_err(|error| Fallback::Failed(error.to_string()))?;
        let DitherPolicy::Diffusion {
            kernel,
            feedback,
            strength,
            serpentine,
            placement,
        } = request.dither
        else {
            return Err(Fallback::UnsupportedStage);
        };
        let serial = serpentine || force_serial;
        if DiffusionMemory::new(layout.output).required_bytes > memory_limit {
            return Err(Fallback::MemoryLimit);
        }
        let reused = incoming.is_some();
        let bits = match placement {
            Placement::Everywhere {} => None,
            Placement::SmoothAreaV1 {
                sample_size,
                contrast_threshold,
            } if sample_size <= 100 => Some(if let Some(bits) = incoming {
                bits
            } else {
                self.adaptive_coverage_checked(
                    layout.output,
                    request.quantize.source.data,
                    sample_size,
                    contrast_threshold,
                    memory_limit,
                    check,
                )
                .await?
            }),
            _ => return Err(Fallback::UnsupportedStage),
        };
        let palette = PreparedPalette::try_new(
            request.quantize.palette,
            request.quantize.alpha,
            memory_limit,
        )
        .map_err(|_| Fallback::MemoryLimit)?;
        let result = match self {
            Self::Cpu(reason) => Err(reason.clone()),
            Self::Gpu(gpu) => {
                gpu.scoped(async {
                    let pixels =
                        u64::from(layout.output.width()) * u64::from(layout.output.height());
                    if !serial
                        && pixels * 12
                            > (gpu.device.limits().max_storage_buffer_binding_size as u64)
                                .min(gpu.device.limits().max_buffer_size)
                    {
                        return Err(Fallback::TooLarge);
                    }
                    let mut job = AreaJob::new_with_analysis(
                        gpu,
                        layout.output,
                        request.quantize.source.data,
                        false,
                    )?;
                    job.cells.destroy();
                    job.cells = gpu.buffer(
                        if serial {
                            u64::from(layout.output.width()) * 36
                        } else {
                            pixels * 12
                        },
                        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    );
                    if serial {
                        job.lab.destroy();
                        job.lab = gpu.buffer(
                            u64::from(layout.output.width()) * 36,
                            wgpu::BufferUsages::STORAGE,
                        );
                    }
                    let output_bytes = IndexedMemory::new(layout.output).index_bytes;
                    job.work.destroy();
                    job.work = gpu.buffer(
                        output_bytes + 4,
                        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    );
                    job.settings = indexed::settings(
                        &palette,
                        request.quantize.matching,
                        request.quantize.alpha,
                        None,
                        WorkingSpace::Srgb,
                        0.0,
                        bits.is_some(),
                    );
                    if let Some(bits) = &bits {
                        job.coverage = bits.clone();
                        job.settings[24..28].copy_from_slice(&2u32.to_le_bytes());
                    }
                    let mut taps = ErrorDiffusionKernel::from(kernel).taps().to_vec();
                    let skew = taps
                        .iter()
                        .filter(|tap| tap.dy > 0)
                        .map(|tap| tap.dx.saturating_neg().max(0) as u32 / tap.dy + 1)
                        .max()
                        .unwrap_or(1);
                    if !serial {
                        // Gather from earlier source rows, then earlier x, matching
                        // the raster scatter's order of rounded additions.
                        taps.sort_by_key(|tap| std::cmp::Reverse((tap.dy, tap.dx)));
                    }
                    let offset = (job.settings.len() / 4) as u32;
                    for (position, value) in [
                        (1, skew),
                        (2, offset),
                        (3, (feedback == DiffusionFeedback::Matching) as u32),
                        (4, serpentine as u32),
                        (5, strength.to_bits()),
                        (15, taps.len() as u32),
                    ] {
                        job.settings[position * 4..(position + 1) * 4]
                            .copy_from_slice(&value.to_le_bytes());
                    }
                    for tap in &taps {
                        job.settings.extend(
                            [tap.dx as u32, tap.dy, tap.weight.to_bits()]
                                .into_iter()
                                .flat_map(u32::to_le_bytes),
                        );
                    }
                    if !serial {
                        job.wavefronts(layout.output, skew);
                    } else {
                        for y in 0..layout.output.height().min(3) {
                            job.prepare_row(y, layout.output);
                        }
                        for y in 0..layout.output.height() {
                            for first in
                                (0..layout.output.width()).step_by(PIXELS_PER_DISPATCH as usize)
                            {
                                let mut parameters = Parameters::new(layout.output, 4, 0.0);
                                parameters.diagonal = y;
                                parameters.reverse_x = first;
                                parameters.minimum =
                                    (layout.output.width() - first).min(PIXELS_PER_DISPATCH);
                                job.dispatches.push(Dispatch {
                                    shader: if force_serial {
                                        shader::WGSL
                                    } else {
                                        cooperative::WGSL
                                    },
                                    entry: if force_serial {
                                        "diffuse"
                                    } else {
                                        "cooperative"
                                    },
                                    parameters,
                                    groups: [1, 1],
                                });
                            }
                            if y + 3 < layout.output.height() {
                                job.prepare_row(y + 3, layout.output);
                            }
                        }
                    }
                    check.timing("GPU diffusion", true)?;
                    let mut bytes = job
                        .execute_read_batched_checked(
                            &job.work,
                            output_bytes + 4,
                            check,
                            batch_size,
                        )
                        .await?;
                    check.timing("GPU diffusion", false)?;
                    if bytes[output_bytes as usize..].iter().any(|&byte| byte != 0) {
                        return Err(Fallback::Failed("non-finite diffusion arithmetic".into()));
                    }
                    bytes.truncate(layout.output.pixel_count().expect("validated dimensions"));
                    Ok(palette.into_indexed(
                        ImageBuf::from_vec_packed(bytes, layout.output).expect("validated output"),
                    ))
                })
                .await
            }
        };
        self.retire_failure(&result);
        result.map(|image| (image, if retain && !reused { bits } else { None }))
    }
}

impl AreaJob<'_> {
    fn wavefronts(&mut self, dimensions: ImageDimensions, skew: u32) {
        let tiles_u = (dimensions.width() + skew * (dimensions.height() - 1)).div_ceil(16);
        let tiles_y = dimensions.height().div_ceil(16);
        for diagonal in 0..tiles_u + tiles_y - 1 {
            // Intersect this tile diagonal with 0 <= u-k*y < width. Empty
            // skew-grid tiles need no workgroup or barriers.
            let denominator = i64::from(16 * (skew + 1));
            let numerator =
                i64::from(16 * diagonal) - i64::from(15 * skew + dimensions.width() - 1);
            let first = ((numerator + denominator - 1).div_euclid(denominator).max(0) as u32)
                .max(diagonal.saturating_sub(tiles_u - 1));
            let last = ((16 * diagonal + 15) / (16 * (skew + 1)))
                .min(tiles_y - 1)
                .min(diagonal);
            if first > last {
                continue;
            }
            let mut parameters = Parameters::new(dimensions, 4, 0.0);
            parameters.diagonal = diagonal;
            parameters.minimum = first;
            self.dispatches.push(Dispatch {
                shader: wavefront::WGSL,
                entry: "wavefront",
                parameters,
                groups: [last - first + 1, 1],
            });
        }
    }

    fn prepare_row(&mut self, y: u32, dimensions: ImageDimensions) {
        let mut parameters = Parameters::new(dimensions, 4, 0.0);
        parameters.diagonal = y;
        self.dispatches.push(Dispatch {
            shader: shader::WGSL,
            entry: "prepare_row",
            parameters,
            groups: [dimensions.width().div_ceil(64), 1],
        });
    }
}
