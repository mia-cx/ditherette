//! The contract between a pipeline stage and the GPU backend.

/// WGSL every stage shader starts with. It binds the RGBA8 source texture and
/// one packed RGBA8 output word per pixel, and converts between them exactly.
pub(super) const PRELUDE: &str = r"
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

// Unorm texels are k / 255, so rounding recovers the source byte exactly.
fn load(xy: vec2<u32>) -> vec4<u32> {
    return vec4<u32>(round(textureLoad(source, xy, 0) * 255.0));
}

fn inside(xy: vec2<u32>) -> bool {
    return all(xy < textureDimensions(source));
}

fn store(xy: vec2<u32>, rgba: vec4<u32>) {
    let bytes = min(rgba, vec4<u32>(255u));
    output[xy.y * textureDimensions(source).x + xy.x] =
        bytes.r | (bytes.g << 8u) | (bytes.b << 16u) | (bytes.a << 24u);
}
";

/// Workgroup edge every stage declares: `@compute @workgroup_size(8, 8)`.
pub(super) const WORKGROUP: u32 = 8;

/// One pipeline stage with a CPU reference and a WGSL kernel.
///
/// The kernel follows [`PRELUDE`]: it declares its parameters at
/// `@group(0) @binding(2)` as a read-only storage buffer and an entry point
/// `@compute @workgroup_size(8, 8) fn main(@builtin(global_invocation_id) id: vec3<u32>)`
/// that writes each pixel with `store`.
pub trait Stage: 'static {
    /// The kernel, without the prelude.
    const WGSL: &'static str;

    /// Bytes bound at `@binding(2)`. Must be non-empty and a multiple of 4.
    fn params(&self) -> Vec<u8>;

    /// The CPU path, on packed RGBA8 in place. This is the reference the GPU is compared against.
    fn cpu(&self, rgba: &mut [u8]);
}
