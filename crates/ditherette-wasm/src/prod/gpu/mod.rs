//! GPU backend on `wgpu`, with the CPU path as reference and fallback.
//!
//! The same WGSL runs on WebGPU in the browser and on Vulkan, Metal, or
//! DirectX 12 natively. [`Backend::detect`] probes once at startup: it requests
//! an adapter and device, then runs [`ChannelLookup::known_answer`] and keeps
//! the GPU only when the result matches the CPU byte for byte. Anything else,
//! including a lost device later on, leaves a [`Backend::Cpu`] that says why.
//!
//! Results come back only through mapped buffers, never a canvas, because
//! privacy features such as Helium's add noise to canvas readback.
//! Indexed jobs return palette indices; palette colours stay exact.
//!
//! The opt-in package effects processor uses [`ChannelLookup::from_effects`]
//! and [`Backend::lookup`] for complete enabled per-channel chains. Other
//! [`Backend::dither_and_quantize`] runs independent fields, matching and complete
//! Adaptive selection. Scan-sensitive diffusion remains on CPU.

mod area;
mod device;
mod lookup;
mod memory;
mod parity;
mod resize;
mod stage;
#[cfg(test)]
mod tests;

pub use area::{AreaMemory, IndexedMemory, MixMemory};
pub use device::{Adapters, Gpu};
pub use lookup::ChannelLookup;
pub use memory::LookupMemory;
pub use parity::Parity;
pub use resize::{GpuTiling, ResizeMemory};
pub use stage::Stage;

/// Cancellation is checked between GPU batches and ordinary CPU diffusion rows.
pub(crate) trait JobControl {
    fn check(&mut self) -> Result<(), Fallback>;
    fn row(&mut self, _completed: u32) -> Result<(), Fallback> {
        self.check()
    }
}
impl<F: FnMut() -> Result<(), Fallback>> JobControl for F {
    fn check(&mut self) -> Result<(), Fallback> {
        self()
    }
}

use crate::image::ImageDimensions;

/// Why the CPU ran instead of the GPU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fallback {
    /// No adapter, or no WebGPU in this browser.
    NoAdapter,
    /// Only a software rasterizer, which is slower than the CPU path.
    Software(String),
    /// The adapter refused a device.
    NoDevice(String),
    /// The startup known-answer check did not round-trip exactly.
    Mismatch,
    /// The device was lost or reported an error outside a call.
    Lost(String),
    /// A call failed: validation, out of memory, or a failed buffer map.
    Failed(String),
    /// The image exceeds this device's texture or buffer limits. The GPU stays usable.
    TooLarge,
    /// Planned owned capacity exceeds the caller's memory limit. The GPU stays usable.
    MemoryLimit,
    /// This dither family or placement is retained on the CPU.
    UnsupportedStage,
    /// The host cancelled this job; no result may be published.
    Cancelled,
    /// A caller progress/control callback threw. This is not a GPU failure.
    Callback,
}

/// Where stages run. Starts from [`Backend::detect`]; a failed GPU call demotes it to the CPU.
pub enum Backend {
    Gpu(Gpu),
    Cpu(Fallback),
}

impl Backend {
    /// Attempts real effects lookup with all transient image capacity charged
    /// before upload. The host runs its CPU processor on any rejection.
    pub async fn lookup(
        &mut self,
        stage: &ChannelLookup,
        dimensions: ImageDimensions,
        rgba: &[u8],
        memory_limit: u64,
    ) -> Result<Vec<u8>, Fallback> {
        LookupMemory::new(dimensions).check(memory_limit)?;
        let result = match self {
            Self::Cpu(reason) => Err(reason.clone()),
            Self::Gpu(gpu) => gpu.process(stage, dimensions, rgba).await,
        };
        self.retire_failure(&result);
        result
    }

    /// Releases a failed device, keeping request-specific size/budget rejections recoverable.
    pub(crate) fn retire_failure<T>(&mut self, result: &Result<T, Fallback>) {
        if let Err(reason) = result {
            if !matches!(
                reason,
                Fallback::TooLarge
                    | Fallback::MemoryLimit
                    | Fallback::Cancelled
                    | Fallback::Callback
            ) {
                *self = Self::Cpu(reason.clone());
            }
        }
    }

    /// Probes the platform's default adapter. Software adapters count as no GPU.
    pub async fn detect() -> Self {
        Self::detect_on(&Gpu::instance(), Adapters::Hardware).await
    }

    /// Probes `instance`, keeping the GPU only if the known-answer check matches exactly.
    pub async fn detect_on(instance: &wgpu::Instance, adapters: Adapters) -> Self {
        let gpu = match Gpu::request(instance, adapters).await {
            Ok(gpu) => gpu,
            Err(reason) => return Self::Cpu(reason),
        };
        let (stage, dimensions, rgba) = ChannelLookup::known_answer();
        match gpu.parity(&stage, dimensions, &rgba).await {
            Ok(parity) if parity.exact() => Self::Gpu(gpu),
            Ok(_) => Self::Cpu(Fallback::Mismatch),
            Err(reason) => Self::Cpu(reason),
        }
    }

    /// Runs `stage` on packed RGBA8 in place. Returns `None` when the GPU ran it,
    /// or why the CPU did. Any GPU failure except [`Fallback::TooLarge`] demotes
    /// this backend to the CPU for later calls; call [`Backend::detect`] to probe again.
    pub async fn run<S: Stage>(
        &mut self,
        stage: &S,
        dimensions: ImageDimensions,
        rgba: &mut [u8],
    ) -> Option<Fallback> {
        let reason = match self {
            Self::Cpu(reason) => reason.clone(),
            Self::Gpu(gpu) => match gpu.process(stage, dimensions, rgba).await {
                Ok(output) => {
                    rgba.copy_from_slice(&output);
                    return None;
                }
                Err(Fallback::TooLarge) => Fallback::TooLarge,
                Err(reason) => {
                    *self = Self::Cpu(reason.clone());
                    reason
                }
            },
        };
        stage.cpu(rgba);
        Some(reason)
    }
}
