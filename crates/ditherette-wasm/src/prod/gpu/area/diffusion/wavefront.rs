//! With u=x+k*y, every predecessor has smaller u and no greater y.
//! Dispatch diagonals order tiles; barriers order the pixels inside each tile.
pub(super) const WGSL: &str = r#"
@group(0) @binding(2) var<storage, read_write> work: array<atomic<u32>>;
var<workgroup> errors: array<vec3<f32>,256>;
@compute @workgroup_size(16,16)
fn wavefront(@builtin(workgroup_id) group:vec3<u32>,@builtin(local_invocation_id) local:vec3<u32>) {
    let tile_y=p.minimum+group.x;
    let tile_u=p.diagonal-tile_y;
    let u=i32(tile_u*16u+local.x);
    let y=i32(tile_y*16u+local.y);
    let skew=i32(settings[1]);
    let xy=vec2(u-skew*y,y);
    let valid=inside(xy);
    let slot=local.y*16u+local.x;
    errors[slot]=vec3(0.0);
    var current=vec3(0.0);
    if valid {current=initial(index(xy));}
    workgroupBarrier();
    for(var diagonal=0u;diagonal<31u;diagonal++) {
        if valid && local.x+local.y==diagonal {
            let i=index(xy);
            if !fixed(i) {
                // Incoming sources are sorted by original raster order, so
                // each rounded addition retains the CPU contribution order.
                for(var n=0u;n<settings[15];n++) {
                    let tap=settings[2]+3u*n;
                    let dx=bitcast<i32>(settings[tap]);
                    let dy=i32(settings[tap+1u]);
                    let other=xy-vec2(dx,dy);
                    if !inside(other) {continue;}
                    let lx=i32(local.x)-dx-skew*dy;
                    let ly=i32(local.y)-dy;
                    var error:vec3<f32>;
                    if lx>=0 && lx<16 && ly>=0 && ly<16 {error=errors[u32(ly)*16u+u32(lx)];}
                    else {
                        let at=3u*index(other);
                        error=vec3(cells[at],cells[at+1u],cells[at+2u]);
                    }
                    let weight=bitcast<f32>(settings[tap+2u])*bitcast<f32>(settings[5]);
                    current+=error*weight;
                }
            }
            let result=match_current(current,i);
            if result.valid {
                errors[slot]=result.error;
                // Adjacent pixels can share a packed word across workgroups.
                atomicOr(&work[i/4u],result.value<<((i%4u)*8u));
            } else {atomicStore(&work[(count()+3u)/4u],1u);}
        }
        workgroupBarrier();
    }
    if valid {
        let at=3u*index(xy);
        let error=errors[slot];
        cells[at]=error.x;cells[at+1u]=error.y;cells[at+2u]=error.z;
    }
}
"#;
