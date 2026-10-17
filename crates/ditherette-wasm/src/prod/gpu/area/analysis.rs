//! Embedded WGSL keeps GPU programs within the audited source tree.

pub(super) const WGSL: &str = r#"@group(0) @binding(2) var<storage, read_write> work: array<f32>;

fn cell_color(i: u32) -> vec3<f32> {
    return vec3(cells[3u*i], cells[3u*i+1u], cells[3u*i+2u]);
}
fn coarse_color(i: u32) -> vec3<f32> {
    return vec3(abs(work[3u*i]), work[3u*i+1u], work[3u*i+2u]);
}
fn square_offset(n: u32, radius: i32) -> vec2<i32> {
    let offsets = array(vec2(-1,-1), vec2(0,-1), vec2(1,-1), vec2(-1,0),
                        vec2(1,0), vec2(-1,1), vec2(0,1), vec2(1,1));
    return offsets[n]*radius;
}
fn clipped(xy: vec2<i32>) -> vec2<i32> {
    return clamp(xy, vec2(0), vec2(i32(p.width)-1, i32(p.height)-1));
}
fn magnitude(a: vec3<f32>, b: vec3<f32>) -> f32 {
    let delta = a-b;
    return sqrt(dot(delta, delta));
}
fn density(t: f32) -> f32 {
    // Carry the multiplication residual rather than introducing the f32
    // cubic's known downward step near one. The CPU evaluates this in f64.
    let square = t*t;
    let residual = fma(t, t, -square);
    let factor = 3.0-2.0*t;
    return fma(square, factor, residual*factor);
}
@compute @workgroup_size(8, 8)
fn colors(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let i = id.y*p.width + id.x;
    let c = oklab(i);
    cells[3u*i] = c.x;
    cells[3u*i+1u] = c.y;
    cells[3u*i+2u] = c.z;
}
@compute @workgroup_size(8, 8)
fn boundaries(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let xy = vec2<i32>(id.xy);
    let i = index(xy);
    let center = cell_color(i);
    var minimum = center;
    var maximum = center;
    var fine = 0.0;
    for (var n = 0u; n < 8u; n++) {
        let c = cell_color(index(clipped(xy + square_offset(n, 1))));
        minimum = min(minimum, c);
        maximum = max(maximum, c);
        fine = max(fine, magnitude(center, c));
    }
    for (var n = 0u; n < 8u; n++) {
        let c = cell_color(index(clipped(xy + square_offset(n, 4))));
        minimum = min(minimum, c);
        maximum = max(maximum, c);
    }
    work[3u*i] = select(0.0, 1.0, fine > 0.0 && magnitude(minimum, maximum) <= 2.0*fine);
}
@compute @workgroup_size(8, 8)
fn mean_horizontal(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let i = id.y*p.width + id.x;
    let radius = min(p.sample_size/2u, min(id.x, p.width-1u-id.x));
    var sum = vec3(0.0);
    var correction = vec3(0.0);
    for (var x = id.x-radius; x <= id.x+radius; x++) {
        let value = cell_color(id.y*p.width+x) - correction;
        let next = sum+value;
        correction = (next-sum)-value;
        sum = next;
    }
    let mean = sum/f32(2u*radius+1u);
    let marked = work[3u*i] == 1.0;
    work[3u*i] = select(mean.x, -mean.x, marked);
    work[3u*i+1u] = mean.y;
    work[3u*i+2u] = mean.z;
}
@compute @workgroup_size(8, 8)
fn contrast(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let xy = vec2<i32>(id.xy);
    let i = index(xy);
    let radius = min(p.sample_size/2u, min(id.y, p.height-1u-id.y));
    var sum = vec3(0.0);
    var correction = vec3(0.0);
    for (var y = id.y-radius; y <= id.y+radius; y++) {
        let value = coarse_color(y*p.width+id.x)-correction;
        let next = sum+value;
        correction = (next-sum)-value;
        sum = next;
    }
    let mean = sum/f32(2u*radius+1u);
    let center = oklab(i);
    var value = min(2.0*magnitude(center, mean), 1.0);
    for (var n = 0u; n < 8u; n++) {
        value = max(value, min(magnitude(center, oklab(index(clipped(xy + square_offset(n, 1))))), 1.0));
    }
    let marked = (bitcast<u32>(work[3u*i]) >> 31u) != 0u;
    cells[3u*i+1u] = select(value, -value-1.0, marked);
    cells[3u*i+2u] = value;
}
@compute @workgroup_size(8, 8)
fn maximum_horizontal(@builtin(global_invocation_id) id: vec3<u32>) {
    let span = min(p.sample_size, p.width);
    if id.x+span > p.width || id.y >= p.height { return; }
    var maximum = 0.0;
    for (var x = id.x; x < id.x+span; x++) {
        maximum = max(maximum, cells[3u*(id.y*p.width+x)+2u]);
    }
    work[3u*(id.y*p.width+id.x)] = maximum;
}
@compute @workgroup_size(8, 8)
fn maximum_vertical(@builtin(global_invocation_id) id: vec3<u32>) {
    let span_x = min(p.sample_size, p.width);
    let span_y = min(p.sample_size, p.height);
    if id.x+span_x > p.width || id.y+span_y > p.height { return; }
    var maximum = 0.0;
    for (var y = id.y; y < id.y+span_y; y++) {
        maximum = max(maximum, work[3u*(y*p.width+id.x)]);
    }
    work[3u*(id.y*p.width+id.x)+1u] = maximum;
}
@compute @workgroup_size(8, 8)
fn seeds(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let i = id.y*p.width+id.x;
    let span = min(vec2(p.sample_size), vec2(p.width,p.height));
    let top = clamp(vec2<i32>(id.xy)-vec2<i32>((span-vec2(1u))/2u), vec2(0), vec2<i32>(vec2(p.width,p.height)-span));
    cells[3u*i] = select(0.0, 1.0, work[3u*index(top)+1u] <= p.tolerance);
    cells[3u*i+2u] = select(p.reach/2.0+1.0, 0.0, cells[3u*i+1u] < 0.0);
}
@compute @workgroup_size(8, 8)
fn edge_shadow(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let i = id.y*p.width+id.x;
    let contrast = select(cells[3u*i+1u], -cells[3u*i+1u]-1.0, cells[3u*i+1u] < 0.0);
    let allowed = contrast <= p.tolerance || cells[3u*i+2u] <= p.reach/2.0;
    cells[3u*i+1u] = select(-1.0, 1.0, allowed);
    cells[3u*i+2u] = select(0.0, p.reach/2.0, allowed);
}
@compute @workgroup_size(8, 8)
fn regional_density(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let i = id.y*p.width+id.x;
    if cells[3u*i+1u] >= 0.0 {
        cells[3u*i+1u] = density(clamp(cells[3u*i+2u]/(p.reach/2.0), 0.0, 1.0));
    }
}
@compute @workgroup_size(8, 8)
fn finish(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let i = id.y*p.width+id.x;
    let t = clamp((2.0*p.reach-cells[3u*i+2u])/p.reach, 0.0, 1.0);
    cells[3u*i+2u] = select(cells[3u*i+1u]*density(t), 0.0, cells[3u*i+1u] < 0.0);
}
"#;
