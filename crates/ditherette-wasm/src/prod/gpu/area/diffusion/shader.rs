//! Shared matching and continuous ring execution for GPU diffusion.

pub(crate) const COMMON: &str = r#"fn fixed(i:u32)->bool {
    return settings[7]==0u || (settings[9]==0u && (source[i]>>24u)<=settings[10]);
}
fn prepared_rgb(i:u32)->vec3<f32> {
    var bytes=vec3<u32>(source[i]&255u,(source[i]>>8u)&255u,(source[i]>>16u)&255u);
    if settings[9]!=0u {
        let alpha=source[i]>>24u;
        bytes*=alpha;
        if settings[9]==2u {
            let matte=settings[11];
            bytes+=vec3(matte&255u,(matte>>8u)&255u,(matte>>16u)&255u)*(255u-alpha);
        }
        bytes=(bytes+vec3(127u))/255u;
    }
    return vec3<f32>(bytes);
}
fn working_space()->u32 {
    switch settings[0] {
        case 4u:{return 1u;} case 5u:{return 2u;}
        case 6u,7u,8u:{return 3u;} case 9u,10u:{return 4u;}
        case 11u,12u,13u:{return 5u;} case 14u:{return 6u;}
        default:{return 0u;}
    }
}
fn finite3(c:vec3<f32>)->bool {return all(abs(c)<=vec3(3.402823466e38));}
fn diffusion_score(a:vec3<f32>,b:vec3<f32>)->f32 {
    let policy=settings[0];
    if policy==8u || policy==13u {
        let angle=abs(a.z-b.z);
        let wrapped=angle-TAU*floor(angle/TAU);
        let arc=min(a.y,b.y)*min(wrapped,TAU-wrapped);
        let d=a.xy-b.xy;
        return dot(d,d)+arc*arc;
    }
    return metric(a,b,policy);
}

fn initial(i:u32)->vec3<f32> {
    if fixed(i) {return vec3(0.0);}
    let c=prepared_rgb(i);
    if settings[3]!=0u {return coordinates(c/255.0,working_space());}
    return c;
}
struct Diffused {value:u32,error:vec3<f32>,valid:bool}
fn match_current(current:vec3<f32>,i:u32)->Diffused {
    if fixed(i) {return Diffused(settings[8],vec3(0.0),true);}
    if !finite3(current) {return Diffused(0u,vec3(0.0),false);}
    var c=current;
    var bytes=vec3(0.0);
    if settings[3]==0u {
        bytes=floor(clamp(c,vec3(0.0),vec3(255.0))+0.5);
        c=coordinates(bytes/255.0,working_space());
    }
    var best=16u;
    var score=3.402823466e38;
    for(var n=0u;n<settings[7];n++) {
        let offset=16u+8u*n;
        let other=vec3(bitcast<f32>(settings[offset+2u]),bitcast<f32>(settings[offset+3u]),bitcast<f32>(settings[offset+4u]));
        let next=diffusion_score(c,other);
        if !(abs(next)<=3.402823466e38) {return Diffused(0u,vec3(0.0),false);}
        if next<score {score=next;best=offset;}
    }
    var error=c-vec3(bitcast<f32>(settings[best+2u]),bitcast<f32>(settings[best+3u]),bitcast<f32>(settings[best+4u]));
    if settings[3]==0u {
        let rgb=settings[best+1u];
        error=bytes-vec3<f32>(f32(rgb&255u),f32((rgb>>8u)&255u),f32((rgb>>16u)&255u));
    }
    var value=settings[best];
    if settings[6]==2u && (coverage[i/32u]&(1u<<(i%32u)))==0u {value=palette_index(i);}
    return Diffused(value,error,true);
}
"#;

pub(super) const WGSL: &str = r#"@group(0) @binding(2) var<storage, read_write> work: array<u32>;
fn write_index(i:u32,value:u32) {
    let shift=(i%4u)*8u;
    work[i/4u]=(work[i/4u]&~(255u<<shift))|(value<<shift);
}
@compute @workgroup_size(64)
fn prepare_row(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=p.width {return;}
    let c=initial(p.diagonal*p.width+id.x);
    let slot=3u*((p.diagonal%3u)*p.width+id.x);
    cells[slot]=c.x;cells[slot+1u]=c.y;cells[slot+2u]=c.z;
    if settings[6]==2u {
        let plain=coordinates(prepared_rgb(p.diagonal*p.width+id.x)/255.0,working_space());
        lab_values[slot]=plain.x;lab_values[slot+1u]=plain.y;lab_values[slot+2u]=plain.z;
    }
}
@compute @workgroup_size(1)
fn diffuse() {
    if work[(count()+3u)/4u]!=0u {return;}
    let y=p.diagonal;
    let reverse=settings[4]!=0u && (y%2u)==1u;
    for(var step=p.reverse_x;step<p.reverse_x+p.minimum;step++) {
        let x=select(step,p.width-1u-step,reverse);
        let i=y*p.width+x;
        if fixed(i) {write_index(i,settings[8]);continue;}
        let slot=3u*((y%3u)*p.width+x);
        let current=vec3(cells[slot],cells[slot+1u],cells[slot+2u]);
        let result=match_current(current,i);
        if !result.valid {work[(count()+3u)/4u]=1u;return;}
        // Output-only placement never depletes the ordinary recurrence.
        write_index(i,result.value);
        for(var n=0u;n<settings[15];n++) {
            let tap=settings[2]+3u*n;
            var dx=bitcast<i32>(settings[tap]);
            if reverse {dx=-dx;}
            let tx=i32(x)+dx;
            let ty=y+settings[tap+1u];
            if tx<0 || tx>=i32(p.width) || ty>=p.height {continue;}
            if fixed(ty*p.width+u32(tx)) {continue;}
            let destination=3u*((ty%3u)*p.width+u32(tx));
            let weight=bitcast<f32>(settings[tap+2u])*bitcast<f32>(settings[5]);
            let old=vec3(cells[destination],cells[destination+1u],cells[destination+2u]);
            let updated=old+result.error*weight;
            if !finite3(updated) {work[(count()+3u)/4u]=1u;return;}
            cells[destination]=updated.x;cells[destination+1u]=updated.y;cells[destination+2u]=updated.z;
        }
    }
}
"#;
