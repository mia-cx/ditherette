//! GPU diffusion. The faithful schedule keeps the original scan: ordinary scans
//! use dependency-ordered wavefront tiles, and serpentine carries a three-row
//! ring continuously through bounded dispatches. The banded schedule trades
//! index identity for parallel row segments; see `banded`.

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

mod banded;
mod cooperative;
pub(super) mod shader;
mod wavefront;
const PIXELS_PER_DISPATCH: u32 = 1024;

/// Entry points composed with the shared diffusion matching helpers.
pub(super) const ENTRIES: [&str; 7] = [
    "prepare_row",
    "diffuse",
    "wavefront",
    "cooperative",
    "banded_lookup",
    "banded_plain",
    "banded",
];

/// How GPU diffusion orders its recurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffusionSchedule {
    /// The original scan order: every pixel sees all its predecessors.
    /// Serpentine is one serial chain and therefore slow.
    Faithful,
    /// Parallel row segments inside bands of rows that carry exact errors
    /// downward. Looks like the original scan; indices differ.
    Banded,
}

/// Internal order; `Serial` forces the one-invocation reference chain for tests.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Order {
    Faithful,
    Serial,
    Banded,
}

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

    /// The banded schedule adds the byte-feedback lookup table and one dispatch
    /// record per band; its carried rows fit within the indexed work planes.
    pub fn banded(dimensions: ImageDimensions) -> Self {
        let dispatches = u64::from(dimensions.height().div_ceil(banded::BAND_ROWS)) + 2;
        let setup = dispatches * (512 + 2 * std::mem::size_of::<Dispatch>() as u64);
        Self {
            required_bytes: IndexedMemory::new(dimensions).required_bytes
                + setup
                + banded::LOOKUP_BYTES,
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

    /// Diffuses with the banded schedule: much faster than `diffuse_gpu`, with
    /// the same kernel, strength, feedback, scan direction, alpha and Adaptive
    /// masking, but different indices. The result is deterministic.
    pub async fn diffuse_gpu_banded(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
    ) -> Result<IndexedImage, Fallback> {
        self.diffuse_gpu_tiled_checked(
            request,
            memory_limit,
            None,
            false,
            super::super::GpuTiling::Auto,
            DiffusionSchedule::Banded,
            &mut || Ok(()),
        )
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
        self.diffuse_gpu_tiled_checked(
            request,
            memory_limit,
            None,
            false,
            tiling,
            DiffusionSchedule::Faithful,
            &mut || Ok(()),
        )
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
            if force_serial {
                Order::Serial
            } else {
                Order::Faithful
            },
            super::MAX_DISPATCHES_PER_BATCH,
        )
        .await
    }

    /// Dependent tiles or bands share the complete error state. Explicit rows cap
    /// queued dependency frontiers rather than starting independent diffusion images.
    pub(crate) async fn diffuse_gpu_tiled_checked(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
        incoming: Option<Vec<u8>>,
        retain: bool,
        tiling: super::super::GpuTiling,
        schedule: DiffusionSchedule,
        check: &mut dyn JobControl,
    ) -> Result<(IndexedImage, Option<Vec<u8>>), Fallback> {
        let batch = match tiling {
            super::super::GpuTiling::Rows(0 | 32769..) => return Err(Fallback::UnsupportedStage),
            super::super::GpuTiling::Rows(rows) => {
                (rows as usize).min(super::MAX_DISPATCHES_PER_BATCH)
            }
            _ => super::MAX_DISPATCHES_PER_BATCH,
        };
        let order = match schedule {
            DiffusionSchedule::Faithful => Order::Faithful,
            DiffusionSchedule::Banded => Order::Banded,
        };
        self.diffuse_gpu_batched(request, memory_limit, incoming, retain, check, order, batch)
            .await
    }

    async fn diffuse_gpu_batched(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        memory_limit: u64,
        incoming: Option<Vec<u8>>,
        retain: bool,
        check: &mut dyn JobControl,
        order: Order,
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
        let banded = order == Order::Banded;
        let serial = !banded && (serpentine || order == Order::Serial);
        let memory = if banded {
            DiffusionMemory::banded(layout.output)
        } else {
            DiffusionMemory::new(layout.output)
        };
        if memory.required_bytes > memory_limit {
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
                        && !banded
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
                    if banded {
                        // Two band parities of two carried rows; segment rings are private.
                        job.lab.destroy();
                        job.lab = gpu.buffer(
                            u64::from(layout.output.width()) * 48,
                            wgpu::BufferUsages::STORAGE,
                        );
                        job.lookup = Some(gpu.buffer(
                            if feedback == DiffusionFeedback::SrgbBytes {
                                banded::LOOKUP_BYTES
                            } else {
                                4
                            },
                            wgpu::BufferUsages::STORAGE,
                        ));
                    } else {
                        job.cells.destroy();
                        job.cells = gpu.buffer(
                            if serial {
                                u64::from(layout.output.width()) * 36
                            } else {
                                pixels * 12
                            },
                            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                        );
                    }
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
                    if !serial && !banded {
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
                    if banded {
                        job.bands(
                            layout.output,
                            feedback == DiffusionFeedback::SrgbBytes,
                            bits.is_some(),
                        );
                    } else if !serial {
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
                                    shader: if order == Order::Serial {
                                        shader::WGSL
                                    } else {
                                        cooperative::WGSL
                                    },
                                    entry: if order == Order::Serial {
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
    /// Builds the lookup table and plain Adaptive indices, then one dependent
    /// dispatch per band. A hashed shift moves every band's segment edges.
    fn bands(&mut self, dimensions: ImageDimensions, lookup: bool, masked: bool) {
        let parameters = Parameters::new(dimensions, 4, 0.0);
        if lookup {
            // 2^22 words of four slots: 1024 x 64 groups of 64 invocations.
            self.dispatches.push(Dispatch {
                shader: banded::WGSL,
                entry: "banded_lookup",
                parameters,
                groups: [1024, 64],
            });
        }
        if masked {
            let pixels = u64::from(dimensions.width()) * u64::from(dimensions.height());
            self.dispatches.push(Dispatch {
                shader: banded::WGSL,
                entry: "banded_plain",
                parameters,
                groups: [1024, pixels.div_ceil(65536) as u32],
            });
        }
        for band in 0..dimensions.height().div_ceil(banded::BAND_ROWS) {
            let mut parameters = Parameters::new(dimensions, banded::SEGMENT, 0.0);
            parameters.reverse_x = banded::BAND_ROWS;
            parameters.reverse_y = banded::HALO;
            parameters.diagonal = band;
            let mut shift = band.wrapping_mul(0x9e37_79b9) ^ 0x85eb_ca6b;
            shift = (shift ^ (shift >> 16)).wrapping_mul(0x7feb_352d);
            parameters.mode = (shift ^ (shift >> 15)) % banded::SEGMENT;
            parameters.tiles_u = (dimensions.width() + parameters.mode).div_ceil(banded::SEGMENT);
            self.dispatches.push(Dispatch {
                shader: banded::WGSL,
                entry: "banded",
                parameters,
                groups: [parameters.tiles_u, 1],
            });
        }
    }

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
