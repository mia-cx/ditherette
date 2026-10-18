//! Iterative traversal of the CPU-prepared balanced tree. Enumeration order,
//! rather than traversal order, resolves score ties.
pub(super) const WGSL: &str = r#"
@group(0) @binding(2) var<storage, read_write> work: array<u32>;
@group(0) @binding(7) var mixtures: texture_2d<u32>;
fn node_part(n: u32, part: u32) -> vec4<u32> {
    let texel=4u*n+part;
    return textureLoad(mixtures,vec2<i32>(i32(texel%4096u),i32(texel/4096u)),0);
}
fn is_polar(policy: u32) -> bool {return policy==7u || policy==8u || policy==12u || policy==13u;}
fn matching_space(policy: u32) -> u32 {
    switch policy {
        case 4u: {return 1u;} case 5u: {return 2u;}
        case 6u,7u,8u: {return 3u;} case 9u,10u: {return 4u;}
        case 11u,12u,13u: {return 5u;} case 14u: {return 6u;}
        default: {return 0u;}
    }
}
fn period_gap(value: f32, low: f32, high: f32) -> f32 {
    var result=TAU;
    for(var shift=-1;shift<=1;shift++) {
        let v=value+f32(shift)*TAU;
        result=min(result,max(max(low-v,v-high),0.0));
    }
    return result;
}
fn subtree_gap(n: u32, color: vec3<f32>, policy: u32) -> f32 {
    let raw_low=node_part(n,2u);
    let low=bitcast<vec3<f32>>(raw_low.xyz);
    let high=bitcast<vec3<f32>>(node_part(n,3u).xyz);
    let d=max(max(low-color,color-high),vec3(0.0));
    if policy==10u {
        let scale=1.0+0.045*0.75*(length(color.yz)+bitcast<f32>(raw_low.w));
        return 0.9801*(d.x*d.x/(1.75*1.75)+0.13*dot(d.yz,d.yz)/(scale*scale));
    }
    let angle=period_gap(color.z,low.z,high.z);
    var hue=0.0;
    if policy==7u || policy==12u {hue=4.0*max(0.0,color.y*low.y)*pow(sin(angle*0.5),2.0);}
    else {hue=pow(min(color.y,low.y)*angle,2.0);}
    return 0.99*(d.x*d.x+d.y*d.y+hue);
}
fn mix_index(color: vec3<f32>, x: u32, y: u32) -> u32 {
    let policy=settings[0];
    var indexed_color=color;
    if is_polar(policy) {indexed_color=vec3(color.x,color.y*cos(color.z),color.y*sin(color.z));}
    let first=16u;
    let initial=vec3(bitcast<f32>(settings[first+2u]),bitcast<f32>(settings[first+3u]),bitcast<f32>(settings[first+4u]));
    var score=metric(color,initial,policy);
    var best_order=0u;
    var best_mix=settings[first];
    var stack: array<vec2<u32>,32>;
    stack[0]=vec2(0u,settings[15]);
    var depth=1u;
    loop {
        if depth==0u {break;}
        depth--;
        let range=stack[depth];
        let middle=range.x+range.y/2u;
        var ceiling=score+32.0*1.1920928955078125e-7*(1.0+score);
        if policy==10u {ceiling*=ceiling;}
        if (is_polar(policy) || policy==10u) && subtree_gap(middle,color,policy)>ceiling {continue;}
        let raw=node_part(middle,0u);
        let metadata=node_part(middle,1u);
        let candidate=bitcast<vec3<f32>>(raw.xyz);
        let order=metadata.w>>2u;
        let next=metric(color,candidate,policy);
        if next<score || (next==score && order<best_order) {
            score=next;best_order=order;best_mix=raw.w;
        }
        let axis=metadata.w&3u;
        let delta=indexed_color[axis]-bitcast<vec3<f32>>(metadata.xyz)[axis];
        let lower=vec2(range.x,range.y/2u);
        let upper=vec2(middle+1u,range.y-range.y/2u-1u);
        let near=select(upper,lower,delta<0.0);
        let far=select(lower,upper,delta<0.0);
        var weights=vec3(1.0);
        if policy==1u {weights=vec3(2.0,4.0,2.0);}
        if policy==2u {weights=vec3(0.299,0.587,0.114);}
        if policy==3u {weights=vec3(0.2126,0.7152,0.0722);}
        if policy==8u || policy==13u {weights=vec3(1.0/(1.0+PI*0.5));}
        // CIEDE uses subtree colour bounds rather than a Euclidean split plane.
        if far.y>0u && (policy==10u || 0.99*weights[axis]*delta*delta<=score+32.0*1.1920928955078125e-7*(1.0+score)) {
            stack[depth]=far;depth++;
        }
        if near.y>0u {stack[depth]=near;depth++;}
    }
    let size=settings[3];
    var rank=0u;
    for(var bit=1u;bit<size;bit<<=1u) {
        let rx=select(0u,1u,(x&bit)!=0u);
        let ry=select(0u,1u,(y&bit)!=0u);
        rank=(rank<<2u)|((rx^ry)<<1u)|ry;
    }
    return select(best_mix&255u,(best_mix>>8u)&255u,rank<(best_mix>>16u));
}
fn yliluoma_index(i: u32) -> u32 {
    let alpha=source[i]>>24u;
    if settings[7]==0u || (settings[9]==0u && alpha<=settings[10]) {return settings[8];}
    if settings[6]==2u && (coverage[i/32u]&(1u<<(i%32u)))==0u {return palette_index(i);}
    var color=rgb(i);
    if settings[9]!=0u {
        var bytes=vec3<u32>(floor(color*255.0+0.5))*alpha;
        if settings[9]==2u {
            let matte=settings[11];
            bytes+=vec3(matte&255u,(matte>>8u)&255u,(matte>>16u)&255u)*(255u-alpha);
        }
        color=vec3<f32>((bytes+vec3(127u))/255u)/255.0;
    }
    return mix_index(coordinates(color,matching_space(settings[0])),i%p.width,i/p.width);
}
@compute @workgroup_size(64)
fn yliluoma(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x>=p.minimum {return;}
    let word=p.diagonal+id.x;
    var packed=0u;
    for(var byte=0u;byte<4u;byte++) {
        let i=word*4u+byte;
        if i<count() {packed|=yliluoma_index(i)<<(byte*8u);}
    }
    work[word]=packed;
}
"#;
