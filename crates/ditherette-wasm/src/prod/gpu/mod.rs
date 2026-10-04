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
//! The GPU never decides palette indices yet: palette output stays exact.
//!
//! Nothing calls this module in normal processing yet. Each stage of #249 adds
//! a [`Stage`] and is compared against the CPU with [`Gpu::parity`].

mod device;
mod lookup;
mod parity;
mod stage;
#[cfg(test)]
mod tests;

pub use device::{Adapters, Gpu};
pub use lookup::ChannelLookup;
pub use parity::Parity;
pub use stage::Stage;

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
}

/// Where stages run. Starts from [`Backend::detect`]; a failed GPU call demotes it to the CPU.
pub enum Backend {
    Gpu(Gpu),
    Cpu(Fallback),
}

impl Backend {
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
