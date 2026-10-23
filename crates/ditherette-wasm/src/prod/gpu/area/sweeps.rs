//! Embedded WGSL keeps GPU programs within the audited source tree.

pub(super) const WGSL: &str = r#"@group(0) @binding(2) var<storage, read_write> work: array<u32>;
var<workgroup> tile: array<f32, 256>;

fn coordinate(u: i32, y: i32) -> vec2<i32> {
    var xy = vec2(u - y, y);
    if p.reverse_x != 0u { xy.x = i32(p.width) - 1 - xy.x; }
    if p.reverse_y != 0u { xy.y = i32(p.height) - 1 - xy.y; }
    return xy;
}

// In skew coordinates u=x+y the frozen predecessors become (-1,0),
// (-2,-1), (-1,-1), (0,-1). A tile depends only on earlier u+y tiles.
// Dispatch boundaries order tiles; workgroup barriers order its local diagonals.
@compute @workgroup_size(16, 16)
fn sweep(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_id) local: vec3<u32>) {
    // The host skips skew tiles outside the image; p.minimum is the first row.
    let ty = i32(p.minimum + group.x);
    let tu = i32(p.diagonal) - ty;
    let u = tu*16 + i32(local.x);
    let y = ty*16 + i32(local.y);
    let xy = coordinate(u, y);
    let valid = y >= 0 && y < i32(p.height) && inside(xy);
    let slot = local.y*16u + local.x;
    var distance = 0.0;
    if valid { distance = cells[3u*index(xy) + 2u]; }
    tile[slot] = distance;
    let offsets = array(vec2(-1, 0), vec2(-2, -1), vec2(-1, -1), vec2(0, -1));
    let steps = array(1.0, 1.4142135623730951, 1.0, 1.4142135623730951);
    // Gates and penalties depend only on the source, and earlier dispatches
    // have finished every out-of-tile neighbour. All lanes prepare them at
    // once so each serial diagonal step does no colour work.
    let pending = valid && distance > 0.0;
    var allowed = array(false, false, false, false);
    var penalties: array<f32, 4>;
    var slots = array(256u, 256u, 256u, 256u);
    var outside: array<f32, 4>;
    if pending {
        let i = index(xy);
        for (var n = 0u; n < 4u; n++) {
            let offset = offsets[n];
            let other = coordinate(u + offset.x, y + offset.y);
            if !inside(other) { continue; }
            let lx = i32(local.x) + offset.x;
            let ly = i32(local.y) + offset.y;
            if lx >= 0 && lx < 16 && ly >= 0 && ly < 16 {
                slots[n] = u32(ly)*16u + u32(lx);
            } else {
                outside[n] = cells[3u*index(other) + 2u];
            }
            if p.mode == 0u {
                allowed[n] = true;
                continue;
            }
            let difference = jump(i, index(other));
            if difference > p.tolerance { continue; }
            if n == 1u || n == 3u {
                if jump(i, index(vec2(other.x, xy.y))) > p.tolerance ||
                   jump(i, index(vec2(xy.x, other.y))) > p.tolerance { continue; }
            }
            let step = steps[n];
            var variation = 0.0;
            if p.tolerance != 0.0 { variation = difference / (step*p.tolerance); }
            penalties[n] = 1.0 + p.reach*variation*variation;
            allowed[n] = true;
        }
    }
    workgroupBarrier();
    for (var diagonal = 0u; diagonal < 31u; diagonal++) {
        if pending && local.x + local.y == diagonal {
            for (var n = 0u; n < 4u; n++) {
                if !allowed[n] { continue; }
                var neighbour = outside[n];
                if slots[n] < 256u { neighbour = tile[slots[n]]; }
                let step = steps[n];
                if p.mode == 0u {
                    distance = min(distance, neighbour + step);
                    continue;
                }
                if neighbour < 0.0 || neighbour + step >= distance { continue; }
                // Keep step*penalty beside the add. Drivers may fuse it into
                // one fma; a precomputed cost rounds the product separately.
                distance = min(distance, neighbour + step*penalties[n]);
            }
            tile[slot] = distance;
        }
        workgroupBarrier();
    }
    if valid { cells[3u*index(xy) + 2u] = distance; }
}
"#;
