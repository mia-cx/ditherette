//! Embedded WGSL keeps GPU programs within the audited source tree.

pub(super) const WGSL: &str = r#"struct Parameters {
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
    unused: u32,
}
@group(0) @binding(0) var<storage, read> source: array<u32>;
@group(0) @binding(1) var<storage, read_write> cells: array<f32>;
@group(0) @binding(3) var<uniform> p: Parameters;
@group(0) @binding(6) var<storage, read_write> lab_values: array<f32>;

fn count() -> u32 { return p.width * p.height; }
fn inside(xy: vec2<i32>) -> bool {
    return all(xy >= vec2(0)) && all(xy < vec2<i32>(i32(p.width), i32(p.height)));
}
fn index(xy: vec2<i32>) -> u32 { return u32(xy.y) * p.width + u32(xy.x); }
fn rgb(i: u32) -> vec3<f32> {
    let value = source[i];
    return vec3<f32>(f32(value & 255u), f32((value >> 8u) & 255u), f32((value >> 16u) & 255u)) / 255.0;
}
fn linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
}
fn oklab_rgb(encoded: vec3<f32>) -> vec3<f32> {
    let c = linear(encoded);
    let l = pow(0.41222146*c.r + 0.53633255*c.g + 0.051445995*c.b, 1.0/3.0);
    let m = pow(0.2119035*c.r + 0.6806995*c.g + 0.10739696*c.b, 1.0/3.0);
    let s = pow(0.08830246*c.r + 0.28171885*c.g + 0.6299787*c.b, 1.0/3.0);
    return vec3(0.21045426*l + 0.7936178*m - 0.004072047*s,
                1.9779985*l - 2.4285922*m + 0.4505937*s,
                0.025904037*l + 0.78277177*m - 0.80867577*s);
}
fn oklab(i: u32) -> vec3<f32> {
    return vec3(lab_values[3u*i], lab_values[3u*i+1u], lab_values[3u*i+2u]);
}
fn jump(a: u32, b: u32) -> f32 {
    let delta = oklab(a) - oklab(b);
    return min(sqrt(dot(delta, delta)), 1.0);
}
"#;
