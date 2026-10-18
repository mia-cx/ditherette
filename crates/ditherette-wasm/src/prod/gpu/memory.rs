//! Owned lookup capacity, preflighted before any source upload or readback allocation.

use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    mem::size_of,
};

use crate::{image::ImageDimensions, prod::effects::operation::BOOKKEEPING_BYTES};

use super::{Backend, ChannelLookup, Fallback, GpuTiling};

// Admission metadata in the package pins this locked Wasm representation.
#[cfg(target_arch = "wasm32")]
const _: () = assert!(size_of::<serde_json::Value>() <= 24);

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
    /// Conservative whole-image admission also charges the independently owned
    /// assembled result and transport tile when the client enables image tiling.
    pub fn with_tiling(dimensions: ImageDimensions, tiling: GpuTiling) -> Self {
        let mut memory = Self::new(dimensions);
        let (width, height) = tiling.tile(dimensions);
        if width != dimensions.width() || height != dimensions.height() {
            memory.required_bytes += 2 * memory.rgba_bytes;
        }
        memory
    }
    /// Retained bridge slot and wasm-bindgen's first 128 four-byte handle slots.
    /// Retiring the artifact releases ownership of these records before CPU fallback.
    pub const BRIDGE_BYTES: u64 =
        128 * 4 + size_of::<RefCell<Option<Backend>>>() as u64 + size_of::<Cell<bool>>() as u64;

    /// Reserves effect decode/planning, the backend and one cached lookup pipeline.
    /// The three parameter copies are CPU bytes, GPU storage and queue staging.
    const RECORD_BYTES: u64 = BOOKKEEPING_BYTES
        + Self::BRIDGE_BYTES
        + size_of::<Backend>() as u64
        + size_of::<ChannelLookup>() as u64
        + 4 * size_of::<(TypeId, wgpu::ComputePipeline)>() as u64
        + 3 * 3 * 256 * 4;

    /// Package admission metadata, mirrored without loading GPU bindings.
    /// Includes the bounded step/plan records before either decoder is entered.
    pub const DECODER_FIXED_BYTES: u64 = 512 * 1024;
    /// Canonical ASCII input only. Wasm Value/Content sizes are 24/16 bytes.
    /// 96 bounds Value Vec/BTree/string ownership, 3*64 bounds the flatten,
    /// internally tagged and untagged Content layers, 32 bounds decoded
    /// curve/grid Vec storage, and 4 bounds string ABI/scratch storage.
    /// These ceilings include minimum Vec capacity and old+new realloc layouts.
    pub const DECODER_BYTES_PER_JSON_BYTE: u64 = 96 + 3 * 64 + 32 + 4;

    pub const FIXED_BYTES: u64 = if Self::RECORD_BYTES > Self::DECODER_FIXED_BYTES {
        Self::RECORD_BYTES
    } else {
        Self::DECODER_FIXED_BYTES
    };

    /// Scalar-only admission before wasm-bindgen copies the effects string or
    /// Rust decodes the chain and builds its tables. The package supplies the
    /// byte length of its validated canonical ASCII JSON, without copying it.
    pub const fn planning_bytes(effects_bytes: u64) -> u64 {
        Self::FIXED_BYTES + Self::DECODER_BYTES_PER_JSON_BYTE * effects_bytes
    }

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
        // wgpu's WebBufferMappedRange copies mapped JS bytes into a temporary
        // Wasm Vec before get_mapped_range().to_vec() creates our result Vec.
        let required_bytes = Self::FIXED_BYTES + 2 * aligned_texture_bytes + 6 * rgba_bytes;
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
