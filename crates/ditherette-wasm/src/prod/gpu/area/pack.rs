//! Embedded WGSL keeps GPU programs within the audited source tree.

pub(super) const WGSL: &str = r#"@group(0) @binding(2) var<storage, read_write> work: array<u32>;
@compute @workgroup_size(64)
fn pack(@builtin(global_invocation_id) id: vec3<u32>) {
    let word = id.x;
    if word >= (count()+31u)/32u { return; }
    var bits = 0u;
    for (var bit = 0u; bit < 32u; bit++) {
        let i = word*32u + bit;
        if i >= count() { break; }
        let x = i%p.width;
        let y = i/p.width;
        let rank = BLUE_NOISE[((x+17u)%32u)*32u + (y+11u)%32u];
        let threshold = (f32(rank)+0.5)/1024.0;
        if cells[3u*i+2u] > threshold { bits |= 1u << bit; }
    }
    // One writer owns the whole word, including the partial final word.
    work[word] = bits;
}
"#;
