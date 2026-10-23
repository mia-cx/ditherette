//! GPU Adaptive passes. Skew tiles preserve the four CPU chamfer sweep orders.

use std::{collections::HashMap, num::NonZeroU64};

use crate::image::ImageDimensions;

use super::{device::map_read, Fallback, Gpu, JobControl};

mod analysis;
mod common;
mod components;
mod indexed_shader;
mod pack;
mod sweeps;
use analysis::WGSL as ANALYSIS;
use common::WGSL as COMMON;
use components::WGSL as COMPONENTS;
use pack::WGSL as PACK;
use sweeps::WGSL as SWEEPS;
mod diffusion;
mod indexed;
mod perturb;
mod yliluoma;
pub use diffusion::DiffusionMemory;
pub use indexed::IndexedMemory;
pub use perturb::PerturbMemory;
pub use yliluoma::MixMemory;

/// Source snapshots/wire copies, GPU source, three packed triplet work planes,
/// packed coverage staging and independently owned host results. Compiler and
/// driver internals are outside this requested-capacity accounting.
#[derive(Debug, Clone, Copy)]
pub struct AreaMemory {
    pub required_bytes: u64,
    pub coverage_bytes: u64,
}

impl AreaMemory {
    pub const FIXED_BYTES: u64 = 4 * 1024 * 1024;

    pub fn new(dimensions: ImageDimensions) -> Self {
        let pixels = u64::from(dimensions.width()) * u64::from(dimensions.height());
        let coverage_bytes = pixels.div_ceil(32) * 4;
        let analysis_dispatches = ((dimensions.width() + dimensions.height() - 1).div_ceil(16)
            + dimensions.height().div_ceil(16)
            - 1) as u64
            * 8
            + 16;
        // Without retained coverage, analysis and indexed dispatches coexist.
        // Rows(1) needs at most two Yliluoma batches per row at MAX_OUTPUT_SIDE;
        // fields and RGBA outputs need no more. Include both Vec capacity and
        // uniform copies before allocating any job metadata.
        let dispatches = analysis_dispatches + 2 * u64::from(dimensions.height());
        // Two immutable uniform copies plus bounded Vec dispatch metadata.
        let fixed =
            Self::FIXED_BYTES + dispatches * (512 + 2 * std::mem::size_of::<Dispatch>() as u64);
        Self {
            required_bytes: fixed + pixels * 52 + coverage_bytes * 4,
            coverage_bytes,
        }
    }
}

impl super::Backend {
    /// Computes the complete accepted Adaptive selector on GPU and returns
    /// row-major coverage bits (least significant bit first). Only 4–100 pixel
    /// samples are admitted by this first GPU implementation. Rejection leaves
    /// the input unchanged; the host chooses its existing CPU fallback.
    pub async fn adaptive_coverage(
        &mut self,
        dimensions: ImageDimensions,
        rgba: &[u8],
        sample_size: u32,
        contrast_threshold: f32,
        memory_limit: u64,
    ) -> Result<Vec<u8>, Fallback> {
        self.adaptive_coverage_checked(
            dimensions,
            rgba,
            sample_size,
            contrast_threshold,
            memory_limit,
            &mut || Ok(()),
        )
        .await
    }

    pub(crate) async fn adaptive_coverage_checked(
        &mut self,
        dimensions: ImageDimensions,
        rgba: &[u8],
        sample_size: u32,
        contrast_threshold: f32,
        memory_limit: u64,
        check: &mut dyn JobControl,
    ) -> Result<Vec<u8>, Fallback> {
        check.check()?;
        let memory = AreaMemory::new(dimensions);
        if memory.required_bytes > memory_limit {
            return Err(Fallback::MemoryLimit);
        }
        if !(4..=100).contains(&sample_size)
            || !contrast_threshold.is_finite()
            || !(0.0..=1.0).contains(&contrast_threshold)
        {
            return Err(Fallback::TooLarge);
        }
        let result = match self {
            Self::Cpu(reason) => Err(reason.clone()),
            Self::Gpu(gpu) => {
                gpu.scoped(async {
                    let mut job = AreaJob::new(gpu, dimensions, rgba)?;
                    let parameters = Parameters::new(dimensions, sample_size, contrast_threshold);
                    job.analysis(parameters);
                    job.dispatches.push(Dispatch {
                        shader: PACK,
                        entry: "pack",
                        parameters,
                        groups: [(memory.coverage_bytes / 4).div_ceil(64) as u32, 1],
                    });
                    job.execute_read_checked(&job.work, memory.coverage_bytes, check)
                        .await
                })
                .await
            }
        };
        self.retire_failure(&result);
        result
    }
}

/// Uniform records are immutable for each dispatch. Rewriting one uniform for
/// several encoded dispatches would make every dispatch see the final value.
#[derive(Clone, Copy)]
struct Parameters {
    width: u32,
    height: u32,
    sample_size: u32,
    minimum: u32,
    tolerance: f32,
    reach: f32,
    mode: u32,
    diagonal: u32,
    reverse_x: u32,
    reverse_y: u32,
    tiles_u: u32,
}

impl Parameters {
    fn new(dimensions: ImageDimensions, sample_size: u32, tolerance: f32) -> Self {
        Self {
            width: dimensions.width(),
            height: dimensions.height(),
            sample_size,
            minimum: (sample_size.min(dimensions.width()) * sample_size.min(dimensions.height()))
                .div_ceil(16),
            tolerance,
            reach: sample_size as f32,
            mode: 0,
            diagonal: 0,
            reverse_x: 0,
            reverse_y: 0,
            tiles_u: (dimensions.width() + dimensions.height() - 1).div_ceil(16),
        }
    }

    fn bytes(self) -> [u8; 48] {
        let words = [
            self.width,
            self.height,
            self.sample_size,
            self.minimum,
            self.tolerance.to_bits(),
            self.reach.to_bits(),
            self.mode,
            self.diagonal,
            self.reverse_x,
            self.reverse_y,
            self.tiles_u,
            0,
        ];
        let mut bytes = [0; 48];
        for (word, target) in words.into_iter().zip(bytes.chunks_exact_mut(4)) {
            target.copy_from_slice(&word.to_le_bytes());
        }
        bytes
    }
}

struct Dispatch {
    shader: &'static str,
    entry: &'static str,
    parameters: Parameters,
    groups: [u32; 2],
}

/// One admitted job owns all intermediate storage. No CPU image plane or
/// per-pass readback is involved; the work plane becomes union/find storage.
struct AreaJob<'a> {
    gpu: &'a Gpu,
    #[cfg(test)]
    dimensions: ImageDimensions,
    source: wgpu::Buffer,
    lab: wgpu::Buffer,
    cells: wgpu::Buffer,
    work: wgpu::Buffer,
    dispatches: Vec<Dispatch>,
    settings: Vec<u8>,
    coverage: Vec<u8>,
    mixes: Option<wgpu::Texture>,
}

impl Drop for AreaJob<'_> {
    fn drop(&mut self) {
        self.source.destroy();
        self.lab.destroy();
        self.cells.destroy();
        self.work.destroy();
        if let Some(mixes) = &self.mixes {
            mixes.destroy();
        }
    }
}

impl<'a> AreaJob<'a> {
    /// Packed-word halos keep odd widths exact. Each word is written once, even
    /// when a row-band boundary falls inside it; field coordinates stay global.
    fn indexed_dispatches(
        &mut self,
        shader: &'static str,
        entry: &'static str,
        parameters: Parameters,
        tiling: super::GpuTiling,
        word_limit: u32,
        packing: u32,
    ) {
        let rows = match tiling {
            super::GpuTiling::Auto => 256,
            super::GpuTiling::Disabled => parameters.height,
            super::GpuTiling::Rows(rows) => rows.max(1),
        };
        let capacity = word_limit.min(
            self.gpu
                .device
                .limits()
                .max_compute_workgroups_per_dimension
                * 64,
        );
        let mut first = 0;
        for y in (0..parameters.height).step_by(rows as usize) {
            let end = (parameters.width * (y + rows).min(parameters.height)).div_ceil(packing);
            while first < end {
                let mut p = parameters;
                p.diagonal = first;
                p.minimum = (end - first).min(capacity);
                self.dispatches.push(Dispatch {
                    shader,
                    entry,
                    parameters: p,
                    groups: [p.minimum.div_ceil(64), 1],
                });
                first += p.minimum;
            }
        }
    }
    fn analysis(&mut self, parameters: Parameters) {
        for entry in [
            "colors",
            "boundaries",
            "mean_horizontal",
            "contrast",
            "maximum_horizontal",
            "maximum_vertical",
            "seeds",
        ] {
            self.pixel_pass(ANALYSIS, entry, parameters);
        }
        for reverse in [false, true] {
            self.sweep(parameters, reverse, reverse);
        }
        self.pixel_pass(ANALYSIS, "edge_shadow", parameters);
        for reverse in [false, true] {
            self.sweep(parameters, reverse, reverse);
        }
        self.pixel_pass(ANALYSIS, "regional_density", parameters);
        self.retain(parameters);
        let mut reconstruction = parameters;
        reconstruction.mode = 1;
        for (reverse_y, reverse_x) in [(false, false), (true, true), (false, true), (true, false)] {
            self.sweep(reconstruction, reverse_x, reverse_y);
        }
        self.pixel_pass(ANALYSIS, "finish", parameters);
    }

    fn pixel_pass(&mut self, shader: &'static str, entry: &'static str, parameters: Parameters) {
        self.dispatches.push(Dispatch {
            shader,
            entry,
            parameters,
            groups: [parameters.width.div_ceil(8), parameters.height.div_ceil(8)],
        });
    }

    fn new(gpu: &'a Gpu, dimensions: ImageDimensions, source: &[u8]) -> Result<Self, Fallback> {
        Self::new_with_analysis(gpu, dimensions, source, true)
    }

    fn new_with_analysis(
        gpu: &'a Gpu,
        dimensions: ImageDimensions,
        source: &[u8],
        analysis: bool,
    ) -> Result<Self, Fallback> {
        Self::new_with_output(gpu, dimensions, source, analysis, false)
    }

    fn new_with_output(
        gpu: &'a Gpu,
        dimensions: ImageDimensions,
        source: &[u8],
        analysis: bool,
        rgba: bool,
    ) -> Result<Self, Fallback> {
        gpu.check()?;
        let pixels = u64::from(dimensions.width()) * u64::from(dimensions.height());
        let limits = gpu.device.limits();
        let binding_bytes = pixels * if analysis { 12 } else { 4 };
        if binding_bytes > limits.max_storage_buffer_binding_size as u64
            || binding_bytes > limits.max_buffer_size
            || dimensions.width().div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || dimensions.height().div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || limits.min_uniform_buffer_offset_alignment > 256
        {
            return Err(Fallback::TooLarge);
        }
        if source.len() as u64 != pixels * 4 {
            return Err(Fallback::Failed(
                "source length does not match dimensions".into(),
            ));
        }
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let source_buffer = gpu.buffer(pixels * 4, storage);
        gpu.queue.write_buffer(&source_buffer, 0, source);
        Ok(Self {
            gpu,
            #[cfg(test)]
            dimensions,
            source: source_buffer,
            // Reconstruction revisits neighbours in four directions. Keep the
            // unchanged source coordinates for this job instead of converting
            // RGB again at every visit. Admission includes this temporary plane.
            lab: gpu.buffer(if analysis { pixels * 12 } else { 4 }, storage),
            cells: gpu.buffer(
                if analysis { pixels * 12 } else { 4 },
                storage | wgpu::BufferUsages::COPY_SRC,
            ),
            work: gpu.buffer(
                if analysis {
                    pixels * 12
                } else if rgba {
                    pixels * 4
                } else {
                    pixels.div_ceil(4) * 4
                },
                storage | wgpu::BufferUsages::COPY_SRC,
            ),
            dispatches: Vec::new(),
            settings: vec![0; 4],
            coverage: vec![0; 4],
            mixes: None,
        })
    }

    fn retain(&mut self, parameters: Parameters) {
        for entry in ["initialize", "connect", "tally", "retain"] {
            self.dispatches.push(Dispatch {
                shader: COMPONENTS,
                entry,
                parameters,
                groups: if entry == "connect" {
                    [parameters.width.div_ceil(8), parameters.height.div_ceil(8)]
                } else {
                    // Two dimensions avoid the portable 65535 workgroup limit.
                    [parameters.width.div_ceil(64), parameters.height]
                },
            });
        }
    }

    fn sweep(&mut self, mut parameters: Parameters, reverse_x: bool, reverse_y: bool) {
        parameters.reverse_x = reverse_x as u32;
        parameters.reverse_y = reverse_y as u32;
        let tiles_y = parameters.height.div_ceil(16);
        for diagonal in 0..parameters.tiles_u + tiles_y - 1 {
            parameters.diagonal = diagonal;
            let first_y = diagonal.saturating_sub(parameters.tiles_u - 1);
            let last_y = diagonal.min(tiles_y - 1);
            self.dispatches.push(Dispatch {
                shader: SWEEPS,
                entry: "sweep",
                parameters,
                groups: [last_y - first_y + 1, 1],
            });
        }
    }

    #[cfg(test)]
    async fn execute_read(
        &self,
        output: &wgpu::Buffer,
        output_bytes: u64,
    ) -> Result<Vec<u8>, Fallback> {
        self.execute_read_checked(output, output_bytes, &mut || Ok(()))
            .await
    }

    async fn execute_read_checked(
        &self,
        output: &wgpu::Buffer,
        output_bytes: u64,
        check: &mut dyn JobControl,
    ) -> Result<Vec<u8>, Fallback> {
        let device = &self.gpu.device;
        let limit = device.limits().max_compute_workgroups_per_dimension;
        if self
            .dispatches
            .iter()
            .any(|dispatch| dispatch.groups.iter().any(|&groups| groups > limit))
        {
            return Err(Fallback::TooLarge);
        }
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let alignment = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let stride = 48u64.div_ceil(alignment) * alignment;
        let mut bytes = vec![0; self.dispatches.len() * stride as usize];
        for (dispatch, target) in self
            .dispatches
            .iter()
            .zip(bytes.chunks_exact_mut(stride as usize))
        {
            target[..48].copy_from_slice(&dispatch.parameters.bytes());
        }
        let parameters = self.gpu.buffer(
            bytes.len() as u64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        self.gpu.queue.write_buffer(&parameters, 0, &bytes);
        let settings = self.gpu.buffer(
            self.settings.len() as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        self.gpu.queue.write_buffer(&settings, 0, &self.settings);
        let coverage = self.gpu.buffer(
            self.coverage.len() as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        self.gpu.queue.write_buffer(&coverage, 0, &self.coverage);
        let mut entries: Vec<_> = [
            (0, true, false),
            (1, false, false),
            (2, false, false),
            (3, false, true),
            (4, true, false),
            (5, true, false),
            (6, false, false),
        ]
        .map(|(binding, read_only, uniform)| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: if uniform {
                    wgpu::BufferBindingType::Uniform
                } else {
                    wgpu::BufferBindingType::Storage { read_only }
                },
                has_dynamic_offset: uniform,
                min_binding_size: if uniform { NonZeroU64::new(48) } else { None },
            },
            count: None,
        })
        .into_iter()
        .collect();
        if self.mixes.is_some() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 7,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Adaptive storage"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let mut resources = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: self.source.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: self.cells.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: self.work.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &parameters,
                    offset: 0,
                    size: NonZeroU64::new(48),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: settings.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: coverage.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: self.lab.as_entire_binding(),
            },
        ];
        let view = self
            .mixes
            .as_ref()
            .map(|texture| texture.create_view(&Default::default()));
        if let Some(view) = &view {
            resources.push(wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &resources,
        });
        let mut pipelines = HashMap::new();
        for dispatch in &self.dispatches {
            pipelines.entry(dispatch.entry).or_insert_with(|| {
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(dispatch.entry),
                    source: wgpu::ShaderSource::Wgsl(
                        format!(
                            "{COMMON}\n{}\n{}\n{}\n{}",
                            dispatch.shader,
                            if [
                                "indexed",
                                "perturb",
                                "yliluoma",
                                "prepare_row",
                                "diffuse",
                                "wavefront",
                                "cooperative"
                            ]
                            .contains(&dispatch.entry)
                            {
                                indexed_shader::WGSL
                            } else {
                                ""
                            },
                            if ["prepare_row", "diffuse", "wavefront", "cooperative"]
                                .contains(&dispatch.entry)
                            {
                                diffusion::shader::COMMON
                            } else {
                                ""
                            },
                            if dispatch.shader == PACK
                                || dispatch.entry == "indexed"
                                || dispatch.entry == "perturb"
                                || dispatch.entry == "yliluoma"
                                || dispatch.entry == "prepare_row"
                                || dispatch.entry == "diffuse"
                                || dispatch.entry == "wavefront"
                                || dispatch.entry == "cooperative"
                            {
                                noise_table()
                            } else {
                                String::new()
                            }
                        )
                        .into(),
                    ),
                });
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(dispatch.entry),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some(dispatch.entry),
                    compilation_options: Default::default(),
                    cache: None,
                })
            });
        }
        let readback = self.gpu.buffer(
            output_bytes,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );
        let checkpoint = self.gpu.buffer(
            4,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );
        let memory = memory.pop();
        let validation = validation.pop();
        let mut readback_mapped = false;
        let result = async {
            if let Some(error) = validation.await.or(memory.await) {
                return Err(Fallback::Failed(error.to_string()));
            }
            // A bounded batch completes before the host checks cancellation.
            // No workgroup spins on work owned by an unscheduled workgroup.
            for (batch, dispatches) in self.dispatches.chunks(64).enumerate() {
                check.check()?;
                let mut encoder = device.create_command_encoder(&Default::default());
                {
                    let mut pass = encoder.begin_compute_pass(&Default::default());
                    for (local, dispatch) in dispatches.iter().enumerate() {
                        pass.set_pipeline(&pipelines[dispatch.entry]);
                        pass.set_bind_group(
                            0,
                            &bindings,
                            &[((batch * 64 + local) as u64 * stride) as u32],
                        );
                        pass.dispatch_workgroups(dispatch.groups[0], dispatch.groups[1], 1);
                    }
                }
                encoder.copy_buffer_to_buffer(&self.cells, 0, &checkpoint, 0, 4);
                self.gpu.queue.submit([encoder.finish()]);
                let mapping = map_read(checkpoint.slice(..));
                super::device::unwound(|| device.poll(wgpu::PollType::wait_indefinitely()))?
                    .map_err(|error| Fallback::Failed(error.to_string()))?;
                mapping
                    .await
                    .map_err(|error| Fallback::Failed(error.to_string()))?;
                checkpoint.unmap();
                self.gpu.check()?;
                check.check()?;
            }
            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(output, 0, &readback, 0, output_bytes);
            self.gpu.queue.submit([encoder.finish()]);
            let mapping = map_read(readback.slice(..));
            super::device::unwound(|| device.poll(wgpu::PollType::wait_indefinitely()))?
                .map_err(|error| Fallback::Failed(error.to_string()))?;
            mapping
                .await
                .map_err(|error| Fallback::Failed(error.to_string()))?;
            readback_mapped = true;
            let mapped = super::device::unwound(|| readback.slice(..).get_mapped_range().to_vec())?;
            self.gpu.check()?;
            check.check()?;
            Ok(mapped)
        }
        .await;
        if readback_mapped {
            readback.unmap();
        }
        readback.destroy();
        parameters.destroy();
        settings.destroy();
        coverage.destroy();
        checkpoint.destroy();
        result
    }

    #[cfg(test)]
    async fn execute(&self) -> Result<Vec<[f32; 3]>, Fallback> {
        let bytes = u64::from(self.dimensions.width()) * u64::from(self.dimensions.height()) * 12;
        Ok(self
            .execute_read(&self.cells, bytes)
            .await?
            .chunks_exact(12)
            .map(|pixel| {
                std::array::from_fn(|axis| {
                    f32::from_le_bytes(pixel[axis * 4..axis * 4 + 4].try_into().unwrap())
                })
            })
            .collect())
    }
}

fn noise_table() -> String {
    use std::fmt::Write;
    let mut source = String::from("const BLUE_NOISE = array<u32, 1024>(");
    for rank in crate::prod::dither::blue_noise::BLUE_NOISE_32X32 {
        write!(source, "{rank}u,").expect("writing to a string");
    }
    source.push_str(");");
    source
}

#[cfg(test)]
mod tests;
