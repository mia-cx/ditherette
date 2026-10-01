//! Adapter and device setup, source upload, dispatch, and mapped-buffer readback.

use std::{
    any::TypeId,
    collections::HashMap,
    future::{poll_fn, Future},
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use crate::image::ImageDimensions;

use super::{
    stage::{PRELUDE, WORKGROUP},
    Fallback, Parity, Stage,
};

/// Which adapters [`Gpu::request`] accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adapters {
    /// Real GPUs only. A software rasterizer is slower than the CPU path.
    Hardware,
    /// Software rasterizers too, such as lavapipe or SwiftShader. For tests and the harness.
    Any,
}

/// An open device that passed or awaits the known-answer check.
pub struct Gpu {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    info: wgpu::AdapterInfo,
    /// Set by the device-lost callback and the uncaptured-error handler.
    failure: Arc<Mutex<Option<String>>>,
    pipelines: Mutex<HashMap<TypeId, wgpu::ComputePipeline>>,
}

impl Gpu {
    /// An instance with every backend this build supports.
    pub fn instance() -> wgpu::Instance {
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle())
    }

    /// Opens the preferred adapter's device with the adapter's own limits.
    pub async fn request(instance: &wgpu::Instance, adapters: Adapters) -> Result<Self, Fallback> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .map_err(|_| Fallback::NoAdapter)?;
        let info = adapter.get_info();
        if adapters == Adapters::Hardware && info.device_type == wgpu::DeviceType::Cpu {
            return Err(Fallback::Software(info.name));
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ditherette"),
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await
            .map_err(|error| Fallback::NoDevice(error.to_string()))?;
        let failure = Arc::new(Mutex::new(None));
        let lost = Arc::clone(&failure);
        device.set_device_lost_callback(move |reason, message| {
            record(&lost, format!("{reason:?}: {message}"));
        });
        let uncaptured = Arc::clone(&failure);
        device.on_uncaptured_error(Arc::new(move |error| {
            record(&uncaptured, error.to_string());
        }));
        Ok(Self {
            device,
            queue,
            info,
            failure,
            pipelines: Mutex::default(),
        })
    }

    /// The adapter's name, backend, and type, for reports.
    pub fn info(&self) -> &wgpu::AdapterInfo {
        &self.info
    }

    /// Uploads packed RGBA8 as an `rgba8unorm` texture.
    pub fn upload(
        &self,
        dimensions: ImageDimensions,
        rgba: &[u8],
    ) -> Result<wgpu::Texture, Fallback> {
        let texture = self.source_texture(dimensions, wgpu::TextureUsages::empty())?;
        self.queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.width()),
                rows_per_image: None,
            },
            texture.size(),
        );
        Ok(texture)
    }

    /// Uploads a decoded bitmap with `copyExternalImageToTexture`, without
    /// premultiplying or converting colour. Create the bitmap with
    /// `premultiplyAlpha: "none"` and `colorSpaceConversion: "none"` so its bytes are the file's.
    #[cfg(target_arch = "wasm32")]
    pub fn upload_bitmap(
        &self,
        bitmap: &wgpu::web_sys::ImageBitmap,
    ) -> Result<wgpu::Texture, Fallback> {
        let dimensions = ImageDimensions::new(bitmap.width(), bitmap.height())
            .map_err(|_| Fallback::TooLarge)?;
        let texture = self.source_texture(dimensions, wgpu::TextureUsages::RENDER_ATTACHMENT)?;
        self.queue.copy_external_image_to_texture(
            &wgpu::CopyExternalImageSourceInfo {
                source: wgpu::ExternalImageSource::ImageBitmap(bitmap.clone()),
                origin: wgpu::Origin2d::ZERO,
                flip_y: false,
            },
            wgpu::CopyExternalImageDestInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
                color_space: wgpu::PredefinedColorSpace::Srgb,
                premultiplied_alpha: false,
            },
            texture.size(),
        );
        Ok(texture)
    }

    /// Uploads `rgba`, runs `stage`, and reads the packed RGBA8 result back.
    pub async fn process<S: Stage>(
        &self,
        stage: &S,
        dimensions: ImageDimensions,
        rgba: &[u8],
    ) -> Result<Vec<u8>, Fallback> {
        let source = self.upload(dimensions, rgba)?;
        self.run(stage, &source).await
    }

    /// Runs `stage` on both backends and compares the results.
    pub async fn parity<S: Stage>(
        &self,
        stage: &S,
        dimensions: ImageDimensions,
        rgba: &[u8],
    ) -> Result<Parity, Fallback> {
        let gpu = self.process(stage, dimensions, rgba).await?;
        let mut cpu = rgba.to_vec();
        stage.cpu(&mut cpu);
        Ok(Parity::between(&cpu, &gpu))
    }

    /// Runs `stage` on an uploaded source and reads the result back from a mapped buffer.
    pub async fn run<S: Stage>(
        &self,
        stage: &S,
        source: &wgpu::Texture,
    ) -> Result<Vec<u8>, Fallback> {
        self.check()?;
        let size = source.size();
        let bytes = u64::from(size.width) * u64::from(size.height) * 4;
        let limits = self.device.limits();
        if bytes
            > limits
                .max_storage_buffer_binding_size
                .min(limits.max_buffer_size)
        {
            return Err(Fallback::TooLarge);
        }
        let memory = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);

        let pipeline = self.pipeline::<S>();
        // Not `create_buffer_init`: writing a buffer mapped at creation panics
        // natively if the buffer is invalid. `write_buffer` reports to the scopes.
        let contents = stage.params();
        let params = self.buffer(
            contents.len() as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        self.queue.write_buffer(&params, 0, &contents);
        let output = self.buffer(
            bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let readback = self.buffer(
            bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let view = source.create_view(&Default::default());
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(
                size.width.div_ceil(WORKGROUP),
                size.height.div_ceil(WORKGROUP),
                1,
            );
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, bytes);
        self.queue.submit([encoder.finish()]);

        let validation = validation.pop();
        let memory = memory.pop();
        if let Some(error) = validation.await.or(memory.await) {
            return Err(Fallback::Failed(error.to_string()));
        }
        let mapped = map_read(readback.slice(..));
        unwound(|| self.device.poll(wgpu::PollType::wait_indefinitely()))?
            .map_err(|error| Fallback::Failed(error.to_string()))?;
        mapped
            .await
            .map_err(|error| Fallback::Failed(error.to_string()))?;
        let result = unwound(|| readback.slice(..).get_mapped_range().to_vec())?;
        readback.unmap();
        // A device lost mid-call may still have mapped stale memory.
        self.check()?;
        Ok(result)
    }

    /// Fails once the device was lost or reported an error outside a call.
    fn check(&self) -> Result<(), Fallback> {
        match &*self.failure.lock().expect("failure lock is never poisoned") {
            Some(message) => Err(Fallback::Lost(message.clone())),
            None => Ok(()),
        }
    }

    fn source_texture(
        &self,
        dimensions: ImageDimensions,
        extra: wgpu::TextureUsages,
    ) -> Result<wgpu::Texture, Fallback> {
        let edge = self.device.limits().max_texture_dimension_2d;
        if dimensions.width() > edge || dimensions.height() > edge {
            return Err(Fallback::TooLarge);
        }
        Ok(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ditherette source"),
            size: wgpu::Extent3d {
                width: dimensions.width(),
                height: dimensions.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | extra,
            view_formats: &[],
        }))
    }

    fn buffer(&self, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        })
    }

    /// Compiles each stage's pipeline once per device.
    fn pipeline<S: Stage>(&self) -> wgpu::ComputePipeline {
        let mut pipelines = self
            .pipelines
            .lock()
            .expect("pipeline lock is never poisoned");
        pipelines
            .entry(TypeId::of::<S>())
            .or_insert_with(|| {
                let module = self
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: None,
                        source: wgpu::ShaderSource::Wgsl(format!("{PRELUDE}{}", S::WGSL).into()),
                    });
                self.device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: None,
                        layout: None,
                        module: &module,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        cache: None,
                    })
            })
            .clone()
    }
}

/// Natively, wgpu panics instead of returning an error when the device fails
/// inside `Device::poll` or `BufferSlice::get_mapped_range`, such as a GPU reset
/// mid-call. Catching that unwind turns it into a fallback; the device is then
/// discarded. It needs `panic = "unwind"`, the native default. On the web
/// neither call fails this way, and Wasm panics abort, so the call runs as is.
pub(super) fn unwound<T>(call: impl FnOnce() -> T) -> Result<T, Fallback> {
    #[cfg(target_arch = "wasm32")]
    return Ok(call());
    #[cfg(not(target_arch = "wasm32"))]
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)).map_err(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()));
        Fallback::Lost(message.unwrap_or_else(|| "wgpu panicked".into()))
    })
}

/// Keeps the first failure; later ones usually follow from it.
fn record(failure: &Mutex<Option<String>>, message: String) {
    failure
        .lock()
        .expect("failure lock is never poisoned")
        .get_or_insert(message);
}

/// Resolves when the slice is mapped. Natively the device must be polled for it to finish;
/// on the web the browser resolves it from its event loop.
fn map_read(
    slice: wgpu::BufferSlice<'_>,
) -> impl Future<Output = Result<(), wgpu::BufferAsyncError>> {
    #[derive(Default)]
    struct Mapping {
        result: Option<Result<(), wgpu::BufferAsyncError>>,
        waker: Option<Waker>,
    }
    let shared = Arc::new(Mutex::new(Mapping::default()));
    let callback = Arc::clone(&shared);
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let mut mapping = callback.lock().expect("mapping lock is never poisoned");
        mapping.result = Some(result);
        if let Some(waker) = mapping.waker.take() {
            waker.wake();
        }
    });
    poll_fn(move |context| {
        let mut mapping = shared.lock().expect("mapping lock is never poisoned");
        match mapping.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                mapping.waker = Some(context.waker().clone());
                Poll::Pending
            }
        }
    })
}
