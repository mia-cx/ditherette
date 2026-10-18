//! GPU resize with global axis plans and bounded halo-aware image tiles.

mod plan;
mod shader;
#[cfg(test)]
mod tests;

use super::{
    device::{map_read, unwound},
    Backend, Fallback, Gpu, JobControl,
};
use crate::{
    image::ImageDimensions,
    prod::contract::request::{Request, ResizePolicy, ResizeRequest},
};
use plan::{pixels, Axis, Filter};
use std::collections::HashMap;

/// Image tiling is independent of hardware workgroups and never resets filter coordinates.
#[derive(Debug, Clone, Copy, Default)]
pub enum GpuTiling {
    #[default]
    Auto,
    Disabled,
    Rows(u32),
}
impl GpuTiling {
    fn tile(self, output: ImageDimensions) -> (u32, u32) {
        match self {
            Self::Disabled => (output.width(), output.height()),
            Self::Auto => (output.width().min(2048), output.height().min(256)),
            Self::Rows(rows) => (output.width().min(2048), output.height().min(rows.max(1))),
        }
    }
}

/// Requested ownership, including input capture/wire/upload, mip levels, plans,
/// halo scratch, staging and independent output. Driver internals are excluded.
#[derive(Debug, Clone, Copy)]
pub struct ResizeMemory {
    pub required_bytes: u64,
}
impl ResizeMemory {
    pub fn new(
        source: ImageDimensions,
        output: ImageDimensions,
        policy: ResizePolicy,
        tiling: GpuTiling,
    ) -> Self {
        let mut workspace = resize_workspace(source, output, Filter::new(policy), tiling);
        let mips = if matches!(policy, ResizePolicy::Trilinear { .. }) {
            let (_, upper, _) = levels(source, output);
            (0..upper)
                .scan(source, |current, _| {
                    let next = half(*current);
                    // Mip construction uses its own output tiles and Area plans,
                    // which may dwarf the final 1x1 filtering workspace.
                    workspace = workspace.max(resize_workspace(
                        *current,
                        next,
                        Filter::new(ResizePolicy::Area {}),
                        tiling,
                    ));
                    *current = next;
                    Some(
                        (u64::from(current.width()) * 16).div_ceil(256)
                            * 256
                            * u64::from(current.height()),
                    )
                })
                .sum()
        } else {
            0
        };
        let source_padding =
            (u64::from(source.width()) * 4).div_ceil(256) * 256 * u64::from(source.height())
                - 4 * pixels(source);
        Self {
            required_bytes: 4 * 1024 * 1024
                + 16 * pixels(source)
                + 8 * pixels(output)
                + workspace
                + mips
                + 2 * source_padding,
        }
    }
}

fn resize_workspace(
    source: ImageDimensions,
    output: ImageDimensions,
    filter: Filter,
    tiling: GpuTiling,
) -> u64 {
    let (width, height) = tiling.tile(output);
    let scale = (f64::from(source.height()) / f64::from(output.height())).max(2.);
    let rows =
        u64::from(source.height()).min((f64::from(height) * scale + 6. * scale + 4.).ceil() as u64);
    let plans = Axis::required(source.width(), output.width(), filter)
        + Axis::required(source.height(), output.height(), filter);
    let padding = (u64::from(width) * 16).div_ceil(256) * 256 - u64::from(width) * 16;
    32 * u64::from(width) * rows
        + 64 * u64::from(width) * u64::from(height)
        + 10 * plans
        + 4 * padding * rows.max(u64::from(height))
}

struct Texture(wgpu::Texture);
impl Drop for Texture {
    fn drop(&mut self) {
        self.0.destroy();
    }
}
struct Buffer(wgpu::Buffer);
impl Drop for Buffer {
    fn drop(&mut self) {
        self.0.destroy();
    }
}

struct Input<'a> {
    texture: &'a wgpu::Texture,
    dimensions: ImageDimensions,
    bytes: bool,
    premultiply: bool,
}
#[derive(Clone, Copy)]
struct Tile {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}
struct Plans {
    y: Axis,
    buffer: Buffer,
    y_start: u32,
}
struct Pipeline {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}
struct Job<'a> {
    gpu: &'a Gpu,
    pipelines: HashMap<&'static str, Pipeline>,
}

impl Backend {
    /// Runs every resize policy on GPU. Coverage mode is the recipe-v2 carrier;
    /// ordinary resize preserves hidden RGB. No fallback is executed internally.
    pub async fn resize(
        &mut self,
        request: ResizeRequest<'_>,
        coverage: bool,
        tiling: GpuTiling,
        memory_limit: u64,
    ) -> Result<Vec<u8>, Fallback> {
        self.resize_checked(request, coverage, tiling, memory_limit, &mut || Ok(()))
            .await
    }
    pub(crate) async fn resize_checked(
        &mut self,
        request: ResizeRequest<'_>,
        coverage: bool,
        tiling: GpuTiling,
        memory_limit: u64,
        check: &mut dyn JobControl,
    ) -> Result<Vec<u8>, Fallback> {
        let layout = Request::Resize(request)
            .validate()
            .map_err(|e| Fallback::Failed(e.to_string()))?;
        let source = ImageDimensions::new(request.source.width, request.source.height)
            .expect("validated source");
        if matches!(tiling, GpuTiling::Rows(0 | 32769..)) {
            return Err(Fallback::Failed("tile height must be 1–32768".into()));
        }
        if ResizeMemory::new(source, layout.output, request.output.resize, tiling).required_bytes
            > memory_limit
        {
            return Err(Fallback::MemoryLimit);
        }
        check.check()?;
        let result = match self {
            Self::Cpu(reason) => Err(reason.clone()),
            Self::Gpu(gpu) => {
                gpu.scoped(async {
                    let mut job = Job::new(gpu);
                    job.run(request, source, layout.output, coverage, tiling, check)
                        .await
                })
                .await
            }
        };
        self.retire_failure(&result);
        result
    }
}

impl<'a> Job<'a> {
    fn new(gpu: &'a Gpu) -> Self {
        Self {
            gpu,
            pipelines: HashMap::new(),
        }
    }
    fn texture(
        &self,
        dimensions: ImageDimensions,
        format: wgpu::TextureFormat,
        extra: wgpu::TextureUsages,
    ) -> Result<Texture, Fallback> {
        let edge = self.gpu.device.limits().max_texture_dimension_2d;
        if dimensions.width() > edge || dimensions.height() > edge {
            return Err(Fallback::TooLarge);
        }
        Ok(Texture(self.gpu.device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("resize carrier"),
                size: wgpu::Extent3d {
                    width: dimensions.width(),
                    height: dimensions.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | extra,
                view_formats: &[],
            },
        )))
    }
    fn float_texture(&self, width: u32, height: u32) -> Result<Texture, Fallback> {
        self.texture(
            ImageDimensions::new(width, height).expect("tile dimensions"),
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::STORAGE_BINDING,
        )
    }
    fn buffer(&self, bytes: u64, usage: wgpu::BufferUsages) -> Result<Buffer, Fallback> {
        let limits = self.gpu.device.limits();
        if bytes > limits.max_buffer_size
            || usage.contains(wgpu::BufferUsages::STORAGE)
                && bytes > u64::from(limits.max_storage_buffer_binding_size)
        {
            return Err(Fallback::TooLarge);
        }
        Ok(Buffer(self.gpu.buffer(bytes.max(4), usage)))
    }
    fn upload(&self, bytes: &[u8], usage: wgpu::BufferUsages) -> Result<Buffer, Fallback> {
        let buffer = self.buffer(bytes.len() as u64, usage | wgpu::BufferUsages::COPY_DST)?;
        self.gpu.queue.write_buffer(&buffer.0, 0, bytes);
        Ok(buffer)
    }
    fn plans(
        &self,
        input: ImageDimensions,
        output: ImageDimensions,
        filter: Filter,
    ) -> Result<Plans, Fallback> {
        let (xa, ya) = plan::axes(filter.anchor);
        let x = Axis::new(input.width(), output.width(), xa, filter)?;
        let y = Axis::new(input.height(), output.height(), ya, filter)?;
        let words = x.words.iter().chain(&y.words);
        let bytes = words.flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>();
        let buffer = self.upload(&bytes, wgpu::BufferUsages::STORAGE)?;
        let y_start = x.words.len() as u32;
        Ok(Plans { y, buffer, y_start })
    }
    fn pipeline(&mut self, entry: &'static str) -> &Pipeline {
        self.pipelines.entry(entry).or_insert_with(|| {
            let bindings: &[u32] = match entry {
                "horizontal_pass" => &[0, 2, 3, 5],
                "vertical_float" => &[1, 3, 5, 6],
                "vertical_bytes" => &[0, 1, 3, 4, 5],
                "blend_bytes" => &[0, 1, 4, 5],
                _ => unreachable!(),
            };
            let entries = bindings
                .iter()
                .map(|&binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    count: None,
                    ty: match binding {
                        0 | 1 => wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        2 | 6 => wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba32Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        _ => wgpu::BindingType::Buffer {
                            ty: if binding == 5 {
                                wgpu::BufferBindingType::Uniform
                            } else {
                                wgpu::BufferBindingType::Storage {
                                    read_only: binding == 3,
                                }
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                    },
                })
                .collect::<Vec<_>>();
            let layout =
                self.gpu
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some(entry),
                        entries: &entries,
                    });
            let pipeline_layout =
                self.gpu
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some(entry),
                        bind_group_layouts: &[Some(&layout)],
                        immediate_size: 0,
                    });
            let module = self
                .gpu
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(entry),
                    source: wgpu::ShaderSource::Wgsl(shader::WGSL.into()),
                });
            let pipeline =
                self.gpu
                    .device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some(entry),
                        layout: Some(&pipeline_layout),
                        module: &module,
                        entry_point: Some(entry),
                        compilation_options: Default::default(),
                        cache: None,
                    });
            Pipeline { layout, pipeline }
        })
    }
    fn dispatch(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        entry: &'static str,
        resources: &[(u32, wgpu::BindingResource<'_>)],
        width: u32,
        height: u32,
    ) {
        let device = &self.gpu.device;
        let pipeline = self.pipeline(entry);
        let entries = resources
            .iter()
            .map(|(binding, resource)| wgpu::BindGroupEntry {
                binding: *binding,
                resource: resource.clone(),
            })
            .collect::<Vec<_>>();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(entry),
            layout: &pipeline.layout,
            entries: &entries,
        });
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
    }
    fn parameters(
        &self,
        input: &Input<'_>,
        output: ImageDimensions,
        tile: Tile,
        plans: &Plans,
        coverage: bool,
        round: bool,
        global: bool,
        signed: bool,
    ) -> Result<Buffer, Fallback> {
        let (first, last) = plans.y.rows(tile.y, tile.height);
        let flags = u32::from(input.bytes)
            | u32::from(input.premultiply) * 2
            | u32::from(round) * 4
            | u32::from(coverage) * 8
            | u32::from(signed) * 16
            | u32::from(global) * 32;
        let words = [
            input.dimensions.width(),
            input.dimensions.height(),
            output.width(),
            output.height(),
            tile.x,
            tile.y,
            tile.width,
            tile.height,
            first,
            last - first + 1,
            plans.y_start,
            flags,
            0,
            0,
            0,
            0,
        ];
        self.upload(
            &words
                .into_iter()
                .flat_map(u32::to_le_bytes)
                .collect::<Vec<_>>(),
            wgpu::BufferUsages::UNIFORM,
        )
    }
    fn filter(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: &Input<'_>,
        plans: &Plans,
        tile: Tile,
        parameters: &Buffer,
        output: Option<&Texture>,
        bytes: Option<&Buffer>,
    ) -> Result<Texture, Fallback> {
        let (first, last) = plans.y.rows(tile.y, tile.height);
        let intermediate = self.float_texture(tile.width, last - first + 1)?;
        let source = input.texture.create_view(&Default::default());
        let view = intermediate.0.create_view(&Default::default());
        self.dispatch(
            encoder,
            "horizontal_pass",
            &[
                (0, wgpu::BindingResource::TextureView(&source)),
                (2, wgpu::BindingResource::TextureView(&view)),
                (3, plans.buffer.0.as_entire_binding()),
                (5, parameters.0.as_entire_binding()),
            ],
            tile.width,
            last - first + 1,
        );
        if let Some(output) = output {
            let target = output.0.create_view(&Default::default());
            self.dispatch(
                encoder,
                "vertical_float",
                &[
                    (1, wgpu::BindingResource::TextureView(&view)),
                    (3, plans.buffer.0.as_entire_binding()),
                    (5, parameters.0.as_entire_binding()),
                    (6, wgpu::BindingResource::TextureView(&target)),
                ],
                tile.width,
                tile.height,
            );
        } else {
            self.dispatch(
                encoder,
                "vertical_bytes",
                &[
                    (0, wgpu::BindingResource::TextureView(&source)),
                    (1, wgpu::BindingResource::TextureView(&view)),
                    (3, plans.buffer.0.as_entire_binding()),
                    (4, bytes.expect("packed target").0.as_entire_binding()),
                    (5, parameters.0.as_entire_binding()),
                ],
                tile.width,
                tile.height,
            );
        }
        Ok(intermediate)
    }
    async fn read(
        &self,
        encoder: wgpu::CommandEncoder,
        source: &Buffer,
        bytes: u64,
        check: &mut dyn JobControl,
    ) -> Result<Vec<u8>, Fallback> {
        check.check()?;
        let staging = self.buffer(
            bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        )?;
        let mut encoder = encoder;
        encoder.copy_buffer_to_buffer(&source.0, 0, &staging.0, 0, bytes);
        self.gpu.queue.submit([encoder.finish()]);
        let mapping = map_read(staging.0.slice(..));
        unwound(|| self.gpu.device.poll(wgpu::PollType::wait_indefinitely()))?
            .map_err(|e| Fallback::Failed(e.to_string()))?;
        mapping.await.map_err(|e| Fallback::Failed(e.to_string()))?;
        let result = unwound(|| staging.0.slice(..).get_mapped_range().to_vec())?;
        staging.0.unmap();
        self.gpu.check()?;
        check.check()?;
        Ok(result)
    }
    async fn run(
        &mut self,
        request: ResizeRequest<'_>,
        source: ImageDimensions,
        output: ImageDimensions,
        coverage: bool,
        tiling: GpuTiling,
        check: &mut dyn JobControl,
    ) -> Result<Vec<u8>, Fallback> {
        let policy = request.output.resize;
        let coverage = coverage && !matches!(policy, ResizePolicy::Nearest { .. });
        let original = self.texture(
            source,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::COPY_DST,
        )?;
        self.gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &original.0,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            request.source.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(source.width() * 4),
                rows_per_image: None,
            },
            original.0.size(),
        );
        let mut mips: Vec<Texture> = Vec::new();
        let (lower, upper, blend) = if matches!(policy, ResizePolicy::Trilinear { .. }) {
            levels(source, output)
        } else {
            (0, 0, 0.)
        };
        let mut dimensions = source;
        for level in 0..upper.max(lower) {
            let input = Input {
                texture: if level == 0 {
                    &original.0
                } else {
                    &mips[level - 1].0
                },
                dimensions,
                bytes: level == 0,
                premultiply: coverage && level == 0,
            };
            let next = half(dimensions);
            let texture = self.float_texture(next.width(), next.height())?;
            let plans = self.plans(dimensions, next, Filter::new(ResizePolicy::Area {}))?;
            for tile in tiles(next, tiling) {
                check.check()?;
                let p =
                    self.parameters(&input, next, tile, &plans, false, !coverage, true, false)?;
                let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
                let _temporary =
                    self.filter(&mut encoder, &input, &plans, tile, &p, Some(&texture), None)?;
                let checkpoint = self.buffer(
                    4,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                )?;
                self.read(encoder, &checkpoint, 4, check).await?;
            }
            dimensions = next;
            mips.push(texture);
        }
        // Explicit destruction is guaranteed even on cancellation or a failed map.
        let level = |lod: usize| {
            let mut dimensions = source;
            for _ in 0..lod {
                dimensions = half(dimensions);
            }
            Input {
                texture: if lod == 0 {
                    &original.0
                } else {
                    &mips[lod - 1].0
                },
                dimensions,
                bytes: lod == 0,
                premultiply: coverage && lod == 0,
            }
        };
        let first = level(lower);
        let second = level(upper);
        let filter = Filter::new(policy);
        let a = self.plans(first.dimensions, output, filter)?;
        let b = if lower != upper {
            Some(self.plans(second.dimensions, output, filter)?)
        } else {
            None
        };
        let mut result = Vec::new();
        result
            .try_reserve_exact((pixels(output) * 4) as usize)
            .map_err(|_| Fallback::MemoryLimit)?;
        result.resize((pixels(output) * 4) as usize, 0);
        for tile in tiles(output, tiling) {
            check.check()?;
            let packed = self.buffer(
                u64::from(tile.width) * u64::from(tile.height) * 4,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            )?;
            let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
            let p = self.parameters(
                &first,
                output,
                tile,
                &a,
                coverage,
                false,
                false,
                coverage && filter.kind >= 3,
            )?;
            let mut live = Vec::new();
            if let Some(b) = &b {
                let low = self.float_texture(tile.width, tile.height)?;
                let high = self.float_texture(tile.width, tile.height)?;
                let p_low =
                    self.parameters(&first, output, tile, &a, false, !coverage, false, false)?;
                live.push(self.filter(&mut encoder, &first, &a, tile, &p_low, Some(&low), None)?);
                let p_high =
                    self.parameters(&second, output, tile, b, false, !coverage, false, false)?;
                live.push(self.filter(
                    &mut encoder,
                    &second,
                    b,
                    tile,
                    &p_high,
                    Some(&high),
                    None,
                )?);
                let mut words = [0u32; 16];
                words[6] = tile.width;
                words[7] = tile.height;
                words[11] = u32::from(coverage) * 8;
                words[12] = (blend as f32).to_bits();
                let mix = self.upload(
                    &words
                        .into_iter()
                        .flat_map(u32::to_le_bytes)
                        .collect::<Vec<_>>(),
                    wgpu::BufferUsages::UNIFORM,
                )?;
                let lv = low.0.create_view(&Default::default());
                let hv = high.0.create_view(&Default::default());
                self.dispatch(
                    &mut encoder,
                    "blend_bytes",
                    &[
                        (0, wgpu::BindingResource::TextureView(&lv)),
                        (1, wgpu::BindingResource::TextureView(&hv)),
                        (4, packed.0.as_entire_binding()),
                        (5, mix.0.as_entire_binding()),
                    ],
                    tile.width,
                    tile.height,
                );
                // Keep every sampled texture and uniform alive through submission/map.
                let bytes = self
                    .read(
                        encoder,
                        &packed,
                        u64::from(tile.width) * u64::from(tile.height) * 4,
                        check,
                    )
                    .await?;
                scatter(&mut result, output, tile, &bytes);
            } else {
                live.push(self.filter(&mut encoder, &first, &a, tile, &p, None, Some(&packed))?);
                let bytes = self
                    .read(
                        encoder,
                        &packed,
                        u64::from(tile.width) * u64::from(tile.height) * 4,
                        check,
                    )
                    .await?;
                scatter(&mut result, output, tile, &bytes);
            }
            if tile.x + tile.width == output.width() {
                check.row(tile.y + tile.height)?;
            }
        }
        Ok(result)
    }
}

fn half(d: ImageDimensions) -> ImageDimensions {
    ImageDimensions::new(d.width().div_ceil(2), d.height().div_ceil(2)).unwrap()
}
fn levels(source: ImageDimensions, output: ImageDimensions) -> (usize, usize, f64) {
    let factor = (f64::from(source.width()) / f64::from(output.width()))
        .max(f64::from(source.height()) / f64::from(output.height()));
    if factor <= 1. {
        return (0, 0, 0.);
    }
    let lod = factor.log2();
    let mut dimensions = source;
    let mut maximum = 0;
    while dimensions.width() != 1 || dimensions.height() != 1 {
        dimensions = half(dimensions);
        maximum += 1;
    }
    let lower = (lod.floor() as usize).min(maximum);
    let upper = (lod.ceil() as usize).min(maximum);
    (lower, upper, lod - lod.floor())
}
fn tiles(output: ImageDimensions, policy: GpuTiling) -> impl Iterator<Item = Tile> {
    let (width, height) = policy.tile(output);
    (0..output.height())
        .step_by(height as usize)
        .flat_map(move |y| {
            (0..output.width())
                .step_by(width as usize)
                .map(move |x| Tile {
                    x,
                    y,
                    width: width.min(output.width() - x),
                    height: height.min(output.height() - y),
                })
        })
}
fn scatter(output: &mut [u8], dimensions: ImageDimensions, tile: Tile, bytes: &[u8]) {
    for y in 0..tile.height {
        let from = y as usize * tile.width as usize * 4;
        let to = ((tile.y + y) as usize * dimensions.width() as usize + tile.x as usize) * 4;
        output[to..to + tile.width as usize * 4]
            .copy_from_slice(&bytes[from..from + tile.width as usize * 4]);
    }
}
