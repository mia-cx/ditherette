//! Embedded WGSL keeps GPU programs within the audited source tree.

pub(super) const WGSL: &str = r#"@group(0) @binding(2) var<storage, read_write> work: array<atomic<u32>>;

// Parents exist before union begins. Every update strictly lowers a parent;
// no invocation waits for another workgroup to be scheduled.
fn root(start: u32) -> u32 {
    var node = start;
    loop {
        let parent = atomicLoad(&work[node]);
        if parent == node { break; }
        let grandparent = atomicLoad(&work[parent]);
        atomicMin(&work[node], grandparent);
        node = grandparent;
    }
    return node;
}
fn join(a: u32, b: u32) {
    loop {
        let ra = root(a);
        let rb = root(b);
        if ra == rb { return; }
        let high = max(ra, rb);
        let result = atomicCompareExchangeWeak(&work[high], high, min(ra, rb));
        if result.exchanged { return; }
    }
}
@compute @workgroup_size(64)
fn initialize(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let i = id.y*p.width + id.x;
    atomicStore(&work[i], i);
    atomicStore(&work[count() + i], 0u);
}
@compute @workgroup_size(8, 8)
fn connect(@builtin(global_invocation_id) id: vec3<u32>) {
    let xy = vec2<i32>(id.xy);
    if !inside(xy) { return; }
    let i = index(xy);
    if cells[3u*i] != 1.0 { return; }
    // These four edges enumerate the undirected eight-connected graph once.
    let neighbours = array(vec2(-1, 0), vec2(-1, -1), vec2(0, -1), vec2(1, -1));
    for (var n = 0u; n < 4u; n++) {
        let other = xy + neighbours[n];
        if inside(other) && cells[3u*index(other)] == 1.0 { join(i, index(other)); }
    }
}
@compute @workgroup_size(64)
fn tally(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let i = id.y*p.width + id.x;
    if cells[3u*i] != 1.0 { return; }
    atomicAdd(&work[count() + root(i)], 1u);
}
@compute @workgroup_size(64)
fn retain(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.width || id.y >= p.height { return; }
    let i = id.y*p.width + id.x;
    var distance = p.reach * 2.0;
    if cells[3u*i + 1u] < 0.0 {
        distance = -1.0;
    } else if cells[3u*i] == 1.0 && atomicLoad(&work[count() + root(i)]) >= p.minimum {
        distance = 0.0;
    }
    cells[3u*i + 2u] = distance;
}
"#;
