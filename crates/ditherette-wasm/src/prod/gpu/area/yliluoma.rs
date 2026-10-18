//! Palette-only CPU index preparation followed by parallel GPU mixture searches.
//! No image matching or ordered selection runs on CPU.

use super::{indexed, AreaJob, AreaMemory, Fallback, IndexedMemory, JobControl, Parameters};
use crate::{
    image::{contracts::IndexedImage, ImageBuf, ImageDimensions},
    prod::{
        contract::request::{BayerSize, DitherQuantizeRequest, Placement, WorkingSpace},
        dither::yiluoma::index::MixIndex,
        quantize::prepared::PreparedQuantizer,
    },
};

mod shader;

const ATLAS_WIDTH: u32 = 4096;
const UPLOAD_ROWS: usize = 16;
const WORDS_PER_DISPATCH: u32 = 4096;

/// Admits the larger of analysis and palette-search lifetimes. The temporary
/// CPU tree, GPU atlas and bounded upload staging are charged before either is
/// allocated. No mixture index survives the call.
pub struct MixMemory {
    pub required_bytes: u64,
}

impl MixMemory {
    pub fn new(dimensions: ImageDimensions, palette_entries: u32, size: u32) -> Self {
        let colors = u64::from(palette_entries.min(256));
        let candidates = colors * (colors + 1) / 2 * (u64::from(size) * u64::from(size) + 1);
        let pixels = u64::from(dimensions.width()) * u64::from(dimensions.height());
        // Candidate + Bounds + CIEDE/hue scoring arrays fit in 80 bytes on
        // either native or Wasm. Texture rows and two upload copies are padded.
        let tree_and_atlas = candidates * 80
            + (candidates * 4).div_ceil(u64::from(ATLAS_WIDTH)) * u64::from(ATLAS_WIDTH) * 16
            + 2 * UPLOAD_ROWS as u64 * u64::from(ATLAS_WIDTH) * 16;
        Self {
            required_bytes: IndexedMemory::new(dimensions)
                .required_bytes
                .max(AreaMemory::FIXED_BYTES + pixels * 20 + tree_and_atlas),
        }
    }
}

impl super::super::Backend {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn yliluoma(
        &mut self,
        request: DitherQuantizeRequest<'_>,
        size: BayerSize,
        placement: Placement,
        memory_limit: u64,
        incoming: Option<Vec<u8>>,
        retain: bool,
        tiling: super::super::GpuTiling,
        check: &mut dyn JobControl,
    ) -> Result<(IndexedImage, Option<Vec<u8>>), Fallback> {
        if let Self::Cpu(reason) = self {
            return Err(reason.clone());
        }
        let source = request.quantize.source;
        let dimensions = ImageDimensions::new(source.width, source.height)
            .map_err(|error| Fallback::Failed(error.to_string()))?;
        let size = indexed::size(size);
        let memory = MixMemory::new(dimensions, request.quantize.palette.len() as u32, size);
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
                    dimensions,
                    source.data,
                    sample_size,
                    contrast_threshold,
                    memory_limit,
                    check,
                )
                .await?
            }),
            _ => return Err(Fallback::UnsupportedStage),
        };
        // Analysis planes are gone before the palette tree or atlas is created.
        let prepared = PreparedQuantizer::try_new(
            request.quantize.palette,
            request.quantize.alpha,
            request.quantize.matching,
            memory_limit,
        )
        .map_err(|_| Fallback::MemoryLimit)?;
        check.check()?;
        let tree = if prepared.palette().visible.is_empty() {
            None
        } else {
            let colors = prepared.palette().visible.len() as u32;
            let height = (colors * (colors + 1) / 2 * (size * size + 1) * 4).div_ceil(ATLAS_WIDTH);
            if let Self::Gpu(gpu) = self {
                let edge = gpu.device.limits().max_texture_dimension_2d;
                if ATLAS_WIDTH > edge || height > edge {
                    return Err(Fallback::TooLarge);
                }
            }
            Some(
                MixIndex::try_new(prepared.matcher(), size * size, memory_limit)
                    .ok_or(Fallback::MemoryLimit)?,
            )
        };
        check.check()?;
        let result = match self {
            Self::Cpu(reason) => Err(reason.clone()),
            Self::Gpu(gpu) => {
                gpu.scoped(async {
                    let mut job = AreaJob::new_with_analysis(gpu, dimensions, source.data, false)?;
                    job.settings = indexed::settings(
                        prepared.palette(),
                        request.quantize.matching,
                        request.quantize.alpha,
                        None,
                        WorkingSpace::Srgb,
                        0.0,
                        bits.is_some(),
                    );
                    job.settings[3 * 4..4 * 4].copy_from_slice(&size.to_le_bytes());
                    if let Some(bits) = &bits {
                        job.settings[6 * 4..7 * 4].copy_from_slice(&2u32.to_le_bytes());
                        job.coverage = bits.clone();
                    }
                    job.upload_mixtures(tree.as_ref(), check).await?;
                    let bytes = IndexedMemory::new(dimensions).index_bytes;
                    job.indexed_dispatches(
                        shader::WGSL,
                        "yliluoma",
                        Parameters::new(dimensions, 4, 0.0),
                        tiling,
                        WORDS_PER_DISPATCH,
                    );
                    let mut indices = job.execute_read_checked(&job.work, bytes, check).await?;
                    indices.truncate(dimensions.pixel_count().expect("validated dimensions"));
                    Ok(prepared.into_indexed(
                        ImageBuf::from_vec_packed(indices, dimensions).expect("validated indices"),
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
    async fn upload_mixtures(
        &mut self,
        tree: Option<&MixIndex>,
        check: &mut dyn JobControl,
    ) -> Result<(), Fallback> {
        let gpu = self.gpu;
        if let Some(tree) = tree {
            let count = tree.gpu_nodes().len() as u32;
            self.settings[15 * 4..16 * 4].copy_from_slice(&count.to_le_bytes());
            let height = (count * 4).div_ceil(ATLAS_WIDTH);
            self.mixes = Some(gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Yliluoma palette tree"),
                size: wgpu::Extent3d {
                    width: ATLAS_WIDTH,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Uint,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }));
            let mut nodes = tree.gpu_nodes();
            let mut upload = Vec::new();
            upload
                .try_reserve_exact(UPLOAD_ROWS * ATLAS_WIDTH as usize * 16)
                .map_err(|_| Fallback::MemoryLimit)?;
            let checkpoint = gpu.buffer(
                4,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            );
            for y in (0..height).step_by(UPLOAD_ROWS) {
                check.check()?;
                let rows = (height - y).min(UPLOAD_ROWS as u32);
                upload.clear();
                for node in nodes
                    .by_ref()
                    .take(rows as usize * ATLAS_WIDTH as usize / 4)
                {
                    upload.extend(node.into_iter().flat_map(u32::to_le_bytes));
                }
                upload.resize(rows as usize * ATLAS_WIDTH as usize * 16, 0);
                gpu.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: self.mixes.as_ref().unwrap(),
                        mip_level: 0,
                        origin: wgpu::Origin3d { x: 0, y, z: 0 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &upload,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(ATLAS_WIDTH * 16),
                        rows_per_image: None,
                    },
                    wgpu::Extent3d {
                        width: ATLAS_WIDTH,
                        height: rows,
                        depth_or_array_layers: 1,
                    },
                );
                // Bound queue-owned upload staging rather than accumulating an atlas copy.
                let mut encoder = gpu.device.create_command_encoder(&Default::default());
                encoder.copy_buffer_to_buffer(&self.cells, 0, &checkpoint, 0, 4);
                gpu.queue.submit([encoder.finish()]);
                let mapping = super::super::device::map_read(checkpoint.slice(..));
                super::super::device::unwound(|| {
                    gpu.device.poll(wgpu::PollType::wait_indefinitely())
                })?
                .map_err(|error| Fallback::Failed(error.to_string()))?;
                mapping
                    .await
                    .map_err(|error| Fallback::Failed(error.to_string()))?;
                checkpoint.unmap();
                gpu.check()?;
                check.check()?;
            }
            checkpoint.destroy();
        } else {
            // The transparent-only path exits before any atlas read,
            // but the shader still declares its texture binding.
            self.mixes = Some(gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("empty Yliluoma palette"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Uint,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }));
        }
        Ok(())
    }
}
