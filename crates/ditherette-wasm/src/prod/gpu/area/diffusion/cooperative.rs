//! One workgroup preserves the serial scan while its lanes share palette scores
//! and disjoint tap/channel updates. No subgroup feature or approximation is used.
pub(super) const WGSL: &str = r#"
@group(0) @binding(2) var<storage, read_write> work: array<u32>;
var<workgroup> query: vec3<f32>;
var<workgroup> byte_query: vec3<f32>;
var<workgroup> plain_query: vec3<f32>;
var<workgroup> shared_error: vec3<f32>;
var<workgroup> pixel_enabled: u32;
var<workgroup> pixel_excluded: u32;
var<workgroup> failed: u32;
var<workgroup> scores: array<f32,32>;
var<workgroup> positions: array<u32,32>;
var<workgroup> plain_scores: array<f32,32>;
var<workgroup> plain_positions: array<u32,32>;
var<workgroup> invalid: array<u32,32>;
var<workgroup> scatter_invalid: atomic<u32>;
@compute @workgroup_size(32)
fn cooperative(@builtin(local_invocation_index) lane:u32) {
    if lane==0u {failed=work[(count()+3u)/4u];}
    if workgroupUniformLoad(&failed)!=0u {return;}
    let y=p.diagonal;
    let reverse=settings[4]!=0u && (y%2u)==1u;
    for(var step=p.reverse_x;step<p.reverse_x+p.minimum;step++) {
        let x=select(step,p.width-1u-step,reverse);
        let i=y*p.width+x;
        if lane==0u {
            pixel_enabled=select(0u,1u,!fixed(i)&&failed==0u);
            pixel_excluded=0u;
            if pixel_enabled!=0u {
                let at=3u*((y%3u)*p.width+x);
                query=vec3(cells[at],cells[at+1u],cells[at+2u]);
                if !finite3(query) {failed=1u;work[(count()+3u)/4u]=1u;pixel_enabled=0u;}
                if settings[3]==0u {
                    byte_query=floor(clamp(query,vec3(0.0),vec3(255.0))+0.5);
                    query=coordinates(byte_query/255.0,working_space());
                }
                if settings[6]==2u && (coverage[i/32u]&(1u<<(i%32u)))==0u {
                    pixel_excluded=1u;
                    plain_query=vec3(lab_values[at],lab_values[at+1u],lab_values[at+2u]);
                }
            }
        }
        workgroupBarrier();
        var best_score=3.402823466e38;
        var best=16u;
        var plain_score=3.402823466e38;
        var plain_best=16u;
        invalid[lane]=0u;
        if pixel_enabled!=0u {
            for(var n=lane;n<settings[7];n+=32u) {
                let at=16u+8u*n;
                let c=vec3(bitcast<f32>(settings[at+2u]),bitcast<f32>(settings[at+3u]),bitcast<f32>(settings[at+4u]));
                let next=diffusion_score(query,c);
                if !(abs(next)<=3.402823466e38) {invalid[lane]=1u;}
                if next<best_score {best_score=next;best=at;}
                if pixel_excluded!=0u {
                    let plain=metric(plain_query,c,settings[0]);
                    if plain<plain_score {plain_score=plain;plain_best=at;}
                }
            }
        }
        scores[lane]=best_score;positions[lane]=best;
        plain_scores[lane]=plain_score;plain_positions[lane]=plain_best;
        workgroupBarrier();
        for(var stride=16u;stride>0u;stride/=2u) {
            if lane<stride {
                let other=lane+stride;
                if scores[other]<scores[lane] || (scores[other]==scores[lane]&&positions[other]<positions[lane]) {
                    scores[lane]=scores[other];positions[lane]=positions[other];
                }
                if pixel_excluded!=0u && (plain_scores[other]<plain_scores[lane] || (plain_scores[other]==plain_scores[lane]&&plain_positions[other]<plain_positions[lane])) {
                    plain_scores[lane]=plain_scores[other];plain_positions[lane]=plain_positions[other];
                }
                invalid[lane]|=invalid[other];
            }
            workgroupBarrier();
        }
        if lane==0u {
            let nearest=positions[0];let plain=plain_positions[0];
            if invalid[0]!=0u {failed=1u;work[(count()+3u)/4u]=1u;}
            atomicStore(&scatter_invalid,0u);
            var value=settings[8];
            shared_error=vec3(0.0);
            if pixel_enabled!=0u && failed==0u {
                value=settings[select(nearest,plain,pixel_excluded!=0u)];
                shared_error=query-vec3(bitcast<f32>(settings[nearest+2u]),bitcast<f32>(settings[nearest+3u]),bitcast<f32>(settings[nearest+4u]));
                if settings[3]==0u {
                    let rgb=settings[nearest+1u];
                    shared_error=byte_query-vec3<f32>(f32(rgb&255u),f32((rgb>>8u)&255u),f32((rgb>>16u)&255u));
                }
            }
            let shift=(i%4u)*8u;
            work[i/4u]=(work[i/4u]&~(255u<<shift))|(value<<shift);
        }
        workgroupBarrier();
        if pixel_enabled!=0u && failed==0u {
            for(var channel=lane;channel<settings[15]*3u;channel+=32u) {
                let tap=settings[2]+3u*(channel/3u);
                var dx=bitcast<i32>(settings[tap]);
                if reverse {dx=-dx;}
                let tx=i32(x)+dx;
                let ty=y+settings[tap+1u];
                if tx<0 || tx>=i32(p.width) || ty>=p.height {continue;}
                if fixed(ty*p.width+u32(tx)) {continue;}
                let axis=channel%3u;
                let at=3u*((ty%3u)*p.width+u32(tx))+axis;
                let weight=bitcast<f32>(settings[tap+2u])*bitcast<f32>(settings[5]);
                let updated=cells[at]+shared_error[axis]*weight;
                if !(abs(updated)<=3.402823466e38) {atomicStore(&scatter_invalid,1u);}
                cells[at]=updated;
            }
        }
        workgroupBarrier();
        if lane==0u {
            if atomicLoad(&scatter_invalid)!=0u {failed=1u;work[(count()+3u)/4u]=1u;}
        }
    }
}
"#;
