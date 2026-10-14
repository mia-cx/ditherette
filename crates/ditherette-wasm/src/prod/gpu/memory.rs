//! Owned lookup capacity, preflighted before any source upload or readback allocation.

use std::{any::TypeId, cell::RefCell, mem::size_of};

use crate::{image::ImageDimensions, prod::effects::operation::BOOKKEEPING_BYTES};

use super::{Backend, ChannelLookup, Fallback};

/// Complete lookup allocation plan. Caller-owned input and returned JS output
/// are excluded, as in the scalar contract. The async host's source snapshot
/// is owned, so it is included along with the Wasm source copy and result Vec.
/// Driver/compiler internal memory remains outside package ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookupMemory {
    pub rgba_bytes: u64,
    pub aligned_texture_bytes: u64,
    pub required_bytes: u64,
}

impl LookupMemory {
    /// Retained bridge slot and wasm-bindgen's first 128 four-byte handle slots.
    /// This remains owned even after the device is released for CPU fallback.
    pub const BRIDGE_BYTES: u64 = 128 * 4 + size_of::<RefCell<Option<Backend>>>() as u64;

    /// Reserves effect decode/planning, the backend and one cached lookup pipeline.
    /// The three parameter copies are CPU bytes, GPU storage and queue staging.
    pub const FIXED_BYTES: u64 = BOOKKEEPING_BYTES
        + Self::BRIDGE_BYTES
        + size_of::<Backend>() as u64
        + size_of::<ChannelLookup>() as u64
        + 4 * size_of::<(TypeId, wgpu::ComputePipeline)>() as u64
        + 3 * 3 * 256 * 4;

    /// Plans packed buffers and a conservative padded texture/upload allocation.
    pub fn new(dimensions: ImageDimensions) -> Self {
        let row = u64::from(dimensions.width()) * 4;
        let height = u64::from(dimensions.height());
        let rgba_bytes = row * height;
        let aligned_texture_bytes = row.div_ceil(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT))
            * u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * height;
        // Texture and upload staging use padded rows. Storage output and mapped
        // readback are packed buffers; snapshot, Wasm source and result Vec are packed too.
        let required_bytes = Self::FIXED_BYTES + 2 * aligned_texture_bytes + 5 * rgba_bytes;
        Self {
            rgba_bytes,
            aligned_texture_bytes,
            required_bytes,
        }
    }

    /// Rejects before allocation, leaving the CPU fallback its own complete budget.
    pub fn check(self, memory_limit: u64) -> Result<(), Fallback> {
        if self.required_bytes > memory_limit {
            return Err(Fallback::MemoryLimit);
        }
        Ok(())
    }
}
