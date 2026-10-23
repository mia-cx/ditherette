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
    let first_y = max(0, i32(p.diagonal) - i32(p.tiles_u) + 1);
    let ty = first_y + i32(group.x);
    let tu = i32(p.diagonal) - ty;
    let u = tu*16 + i32(local.x);
    let y = ty*16 + i32(local.y);
    let xy = coordinate(u, y);
    let valid = y >= 0 && y < i32(p.height) && inside(xy);
    let slot = local.y*16u + local.x;
    tile[slot] = 0.0;
    if valid { tile[slot] = cells[3u*index(xy) + 2u]; }
    workgroupBarrier();
    let offsets = array(vec2(-1, 0), vec2(-2, -1), vec2(-1, -1), vec2(0, -1));
    let steps = array(1.0, 1.4142135623730951, 1.0, 1.4142135623730951);
    for (var diagonal = 0u; diagonal < 31u; diagonal++) {
        if valid && local.x + local.y == diagonal {
            var distance = tile[slot];
            if distance > 0.0 {
                for (var n = 0u; n < 4u; n++) {
                    let offset = offsets[n];
                    let other = coordinate(u + offset.x, y + offset.y);
                    if !inside(other) { continue; }
                    let lx = i32(local.x) + offset.x;
                    let ly = i32(local.y) + offset.y;
                    var neighbour: f32;
                    if lx >= 0 && lx < 16 && ly >= 0 && ly < 16 {
                        neighbour = tile[u32(ly)*16u + u32(lx)];
                    } else {
                        neighbour = cells[3u*index(other) + 2u];
                    }
                    let step = steps[n];
                    if p.mode == 0u {
                        distance = min(distance, neighbour + step);
                        continue;
                    }
                    if neighbour < 0.0 || neighbour + step >= distance { continue; }
                    let i = index(xy);
                    let difference = jump(i, index(other));
                    if difference > p.tolerance { continue; }
                    if n == 1u || n == 3u {
                        if jump(i, index(vec2(other.x, xy.y))) > p.tolerance ||
                           jump(i, index(vec2(xy.x, other.y))) > p.tolerance { continue; }
                    }
                    var variation = 0.0;
                    if p.tolerance != 0.0 { variation = difference / (step*p.tolerance); }
                    let cost = step*(1.0 + p.reach*variation*variation);
                    distance = min(distance, neighbour + cost);
                }
                tile[slot] = distance;
            }
        }
        workgroupBarrier();
    }
    if valid { cells[3u*index(xy) + 2u] = tile[slot]; }
}
"#;
