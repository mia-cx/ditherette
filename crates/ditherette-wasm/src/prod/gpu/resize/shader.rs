//! Separable filtering keeps halo coordinates global and intermediate channels unrounded.

pub(super) const WGSL: &str = r#"
struct Parameters {
    source: vec2<u32>, output: vec2<u32>,
    origin: vec2<u32>, tile: vec2<u32>,
    first_row: u32, rows: u32, y_plan: u32, flags: u32,
    blend: f32, pad0: u32, pad1: u32, pad2: u32,
}
@group(0) @binding(0) var input: texture_2d<f32>;
@group(0) @binding(1) var horizontal: texture_2d<f32>;
@group(0) @binding(2) var intermediate: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var<storage, read> plan: array<u32>;
@group(0) @binding(4) var<storage, read_write> output: array<u32>;
@group(0) @binding(5) var<uniform> p: Parameters;
@group(0) @binding(6) var result: texture_storage_2d<rgba32float, write>;

fn source_pixel(xy: vec2<u32>) -> vec4<f32> {
    var c=textureLoad(input,vec2<i32>(xy),0);
    if (p.flags & 1u)!=0u {c*=255.0;}
    if (p.flags & 2u)!=0u {c=vec4(c.rgb*(c.a/255.0),c.a);}
    return c;
}
fn rounded(c: vec4<f32>)->vec4<f32> {return floor(clamp(c,vec4(0.0),vec4(255.0))+0.5);}
fn filtered(local: vec2<u32>)->vec4<f32> {
    let base=p.y_plan+4u*(p.origin.y+local.y);
    let start=p.y_plan+plan[base];
    var sum=vec4(0.0);
    for(var i=0u;i<plan[base+1u];i++) {
        let tap=start+2u*i;
        sum+=textureLoad(horizontal,vec2<i32>(i32(local.x),i32(plan[tap]-p.first_row)),0)*bitcast<f32>(plan[tap+1u]);
    }
    return sum;
}
fn alpha_range(global:vec2<u32>)->vec2<f32> {
    let x=4u*global.x;
    let y=p.y_plan+4u*global.y;
    var lo=255.0;var hi=0.0;
    for(var sy=plan[y+2u];sy<=plan[y+3u];sy++) {
        for(var sx=plan[x+2u];sx<=plan[x+3u];sx++) {
            let a=textureLoad(input,vec2<i32>(i32(sx),i32(sy)),0).a*255.0;
            lo=min(lo,a);hi=max(hi,a);
        }
    }
    return vec2(lo,hi);
}
fn packed(c:vec4<f32>, alpha:f32)->u32 {
    var value=c;
    if (p.flags & 8u)!=0u {
        value=vec4(0.0,0.0,0.0,alpha);
        if c.a>0.0 && alpha>=0.5 {value=vec4(c.rgb*(255.0/c.a),alpha);}
    }
    let b=vec4<u32>(rounded(value));
    return b.r|(b.g<<8u)|(b.b<<16u)|(b.a<<24u);
}
@compute @workgroup_size(8,8)
fn horizontal_pass(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=p.tile.x || id.y>=p.rows {return;}
    let base=4u*(p.origin.x+id.x);
    let start=plan[base];
    var sum=vec4(0.0);
    for(var i=0u;i<plan[base+1u];i++) {
        let tap=start+2u*i;
        sum+=source_pixel(vec2(plan[tap],p.first_row+id.y))*bitcast<f32>(plan[tap+1u]);
    }
    textureStore(intermediate,vec2<i32>(id.xy),sum);
}
@compute @workgroup_size(8,8)
fn vertical_float(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=p.tile) {return;}
    var c=filtered(id.xy);
    if (p.flags & 4u)!=0u {c=rounded(c);}
    var xy=id.xy;
    if (p.flags & 32u)!=0u {xy+=p.origin;}
    textureStore(result,vec2<i32>(xy),c);
}
@compute @workgroup_size(8,8)
fn vertical_bytes(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=p.tile) {return;}
    let c=filtered(id.xy);
    var alpha=c.a;
    if (p.flags & 16u)!=0u {
        let range=alpha_range(p.origin+id.xy);
        alpha=clamp(alpha,range.x,range.y);
    }
    output[id.y*p.tile.x+id.x]=packed(c,alpha);
}
@compute @workgroup_size(8,8)
fn blend_bytes(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=p.tile) {return;}
    let a=textureLoad(input,vec2<i32>(id.xy),0);
    let b=textureLoad(horizontal,vec2<i32>(id.xy),0);
    let c=a*(1.0-p.blend)+b*p.blend;
    output[id.y*p.tile.x+id.x]=packed(c,c.a);
}
"#;
