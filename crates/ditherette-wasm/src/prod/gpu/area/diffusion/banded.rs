//! Visually similar parallel diffusion for every kernel and scan.
//!
//! Bands of rows run in order, and each band reads the exact errors of the
//! previous band's last two rows, so no row ever starts blind to the row above.
//! Inside a band, row segments run in parallel. Each segment first diffuses a
//! halo of neighbouring pixels to warm its horizontal error, then discards
//! their indices. Segment edges shift every band, so seams never line up.
//! Indices differ from the serial recurrence; the texture and tone match it.

/// Pixels each invocation owns per row.
pub(super) const SEGMENT: u32 = 16;
/// Pixels diffused on each side of a segment only to warm its error.
pub(super) const HALO: u32 = 4;
/// Rows per dependent dispatch; at least the two-row reach of every kernel.
pub(super) const BAND_ROWS: u32 = 8;
/// One nearest-palette slot per sRGB byte triple, for byte feedback.
pub(super) const LOOKUP_BYTES: u64 = 1 << 24;

/// `SPAN` bounds `SEGMENT + 2 * HALO`; the error ring lives in each invocation.
pub(super) const WGSL: &str = r#"
@group(0) @binding(2) var<storage, read_write> work: array<atomic<u32>>;
@group(0) @binding(8) var<storage, read_write> lookup: array<u32>;
const SPAN: u32 = 24u;
var<private> tap_dx: array<i32, 12>;
var<private> tap_dy: array<u32, 12>;
var<private> tap_weight: array<f32, 12>;

fn status_fail() {atomicStore(&work[(count()+3u)/4u],1u);}
fn nearest_slot(c:vec3<f32>)->u32 {
    var best=0u;
    var score=3.402823466e38;
    for(var n=0u;n<settings[7];n++) {
        let at=16u+8u*n;
        let next=diffusion_score(c,vec3(bitcast<f32>(settings[at+2u]),bitcast<f32>(settings[at+3u]),bitcast<f32>(settings[at+4u])));
        if !(abs(next)<=3.402823466e38) {return 0xffffffffu;}
        if next<score {score=next;best=n;}
    }
    return best;
}
// Byte feedback rounds every query to sRGB bytes, so one table holds the
// exact nearest slot for all queries. Four slots share each word.
@compute @workgroup_size(64)
fn banded_lookup(@builtin(global_invocation_id) id:vec3<u32>) {
    let word=id.y*65536u+id.x;
    if word>=4194304u {return;}
    var packed=0u;
    for(var byte=0u;byte<4u;byte++) {
        let code=word*4u+byte;
        let bytes=vec3(f32(code&255u),f32((code>>8u)&255u),f32(code>>16u));
        let slot=nearest_slot(coordinates(bytes/255.0,working_space()));
        if slot==0xffffffffu {status_fail();return;}
        packed|=slot<<(8u*byte);
    }
    lookup[word]=packed;
}
// Adaptive placement masks output only: uncovered pixels take their plain match.
@compute @workgroup_size(64)
fn banded_plain(@builtin(global_invocation_id) id:vec3<u32>) {
    let i=id.y*65536u+id.x;
    if i>=count() || fixed(i) || (coverage[i/32u]&(1u<<(i%32u)))!=0u {return;}
    atomicOr(&work[i/4u],palette_index(i)<<((i%4u)*8u));
}
// The previous band's last two rows, in one of two band-parity slots.
fn carried(y:u32,x:u32)->u32 {
    let rows=p.reverse_x;
    return 3u*((((y/rows)%2u)*2u+y%rows+2u-rows)*p.width+x);
}
// One workgroup per segment. Lanes gather a row's errors from earlier rows in
// parallel; one lane then runs the row's serial scan.
var<workgroup> ring: array<f32, 270>;
var<workgroup> incoming: array<vec3<f32>, 24>;
var<workgroup> fixed_pixels: array<u32, 24>;
var<workgroup> failed: u32;
// Three rows of errors, padded by three zero cells per side so sources
// outside the segment read zero.
fn ring_at(y:u32,local:i32)->u32 {return 3u*((y%3u)*30u+u32(local+3));}
@compute @workgroup_size(32)
fn banded(@builtin(workgroup_id) group:vec3<u32>,@builtin(local_invocation_index) lane:u32) {
    if group.x>=p.tiles_u {return;}
    let segment=p.sample_size;
    let rows=p.reverse_x;
    let halo=p.reverse_y;
    let y0=p.diagonal*rows;
    let y1=min(y0+rows,p.height);
    // Segment group.x covers [group.x*segment-shift, (group.x+1)*segment-shift).
    let x0=u32(max(i32(group.x*segment)-i32(p.mode),0));
    let x1=u32(clamp(i32((group.x+1u)*segment)-i32(p.mode),0,i32(p.width)));
    if x0>=x1 {return;}
    let rx0=u32(max(i32(x0)-i32(halo),0));
    let rx1=min(x1+halo,p.width);
    let span=rx1-rx0;
    let masked=settings[6]==2u;
    let serpentine=settings[4]!=0u;
    let strength=bitcast<f32>(settings[5]);
    // Same-row taps reach one or two pixels back along the scan.
    var same=vec2(0.0);
    var taps=0u;
    for(var n=0u;n<settings[15];n++) {
        let tap=settings[2]+3u*n;
        let weight=bitcast<f32>(settings[tap+2u])*strength;
        if settings[tap+1u]==0u {
            same[bitcast<i32>(settings[tap])-1]=weight;
        } else {
            tap_dx[taps]=bitcast<i32>(settings[tap]);
            tap_dy[taps]=settings[tap+1u];
            tap_weight[taps]=weight;
            taps++;
        }
    }
    for(var y=y0;y<y1;y++) {
        if lane<span {
            let x=rx0+lane;
            let i=y*p.width+x;
            fixed_pixels[lane]=u32(fixed(i));
            var current=initial(i);
            for(var n=0u;n<taps;n++) {
                let dy=tap_dy[n];
                if y<dy {continue;}
                let sy=y-dy;
                // Each source row diffused in its own direction.
                let dx=select(tap_dx[n],-tap_dx[n],serpentine && (sy%2u)==1u);
                if sy<y0 {
                    let sx=i32(x)-dx;
                    if sx<0 || sx>=i32(p.width) {continue;}
                    let at=carried(sy,u32(sx));
                    current+=vec3(lab_values[at],lab_values[at+1u],lab_values[at+2u])*tap_weight[n];
                } else {
                    let at=ring_at(sy,i32(lane)-dx);
                    current+=vec3(ring[at],ring[at+1u],ring[at+2u])*tap_weight[n];
                }
            }
            incoming[lane]=current;
        }
        workgroupBarrier();
        if lane==0u {
            let reverse=serpentine && (y%2u)==1u;
            let carry=y%rows+2u>=rows;
            var previous=vec3(0.0);
            var before=vec3(0.0);
            for(var step=0u;step<span;step++) {
                let local=select(step,span-1u-step,reverse);
                let x=rx0+local;
                let i=y*p.width+x;
                var error=vec3(0.0);
                var slot=0xffffffffu;
                if fixed_pixels[local]==0u {
                    let current=incoming[local]+same.x*previous+same.y*before;
                    if !finite3(current) {failed=1u;break;}
                    if settings[3]==0u {
                        let bytes=floor(clamp(current,vec3(0.0),vec3(255.0))+0.5);
                        let code=u32(bytes.x)|(u32(bytes.y)<<8u)|(u32(bytes.z)<<16u);
                        slot=(lookup[code/4u]>>(8u*(code%4u)))&255u;
                        let rgb=settings[16u+8u*slot+1u];
                        error=bytes-vec3<f32>(f32(rgb&255u),f32((rgb>>8u)&255u),f32((rgb>>16u)&255u));
                    } else {
                        slot=nearest_slot(current);
                        if slot==0xffffffffu {failed=1u;break;}
                        let at=16u+8u*slot;
                        error=current-vec3(bitcast<f32>(settings[at+2u]),bitcast<f32>(settings[at+3u]),bitcast<f32>(settings[at+4u]));
                    }
                }
                // Fixed pixels neither receive nor emit error.
                let at=ring_at(y,i32(local));
                ring[at]=error.x;ring[at+1u]=error.y;ring[at+2u]=error.z;
                before=previous;
                previous=error;
                if x<x0 || x>=x1 {continue;}
                if slot==0xffffffffu {
                    atomicOr(&work[i/4u],settings[8]<<((i%4u)*8u));
                } else if !masked || (coverage[i/32u]&(1u<<(i%32u)))!=0u {
                    atomicOr(&work[i/4u],settings[16u+8u*slot]<<((i%4u)*8u));
                }
                if carry {let at=carried(y,x);lab_values[at]=error.x;lab_values[at+1u]=error.y;lab_values[at+2u]=error.z;}
            }
        }
        if workgroupUniformLoad(&failed)!=0u {
            if lane==0u {status_fail();}
            return;
        }
    }
}
"#;
