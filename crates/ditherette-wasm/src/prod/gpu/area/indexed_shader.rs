//! Embedded WGSL keeps GPU programs within the audited source tree.

pub(super) const WGSL: &str = r#"@group(0) @binding(4) var<storage, read> settings: array<u32>;
@group(0) @binding(5) var<storage, read> coverage: array<u32>;
const PI: f32 = 3.141592653589793;
const TAU: f32 = 6.283185307179586;

fn lab_f(t: vec3<f32>) -> vec3<f32> {
    return select((24389.0/27.0*t+16.0)/116.0, pow(max(t,vec3(0.0)),vec3(1.0/3.0)), t > vec3(216.0/24389.0));
}
fn cylindrical(c: vec3<f32>, encoded: vec3<f32>) -> vec3<f32> {
    if encoded.r == encoded.g && encoded.g == encoded.b { return vec3(c.x,0.0,0.0); }
    var hue = atan2(c.z,c.y);
    if hue < 0.0 { hue += TAU; }
    if hue == TAU { hue = 0.0; }
    return vec3(c.x, length(c.yz), hue);
}
fn coordinates(encoded: vec3<f32>, space: u32) -> vec3<f32> {
    switch space {
        case 0u: { return encoded; }
        case 1u: { return linear(encoded); }
        case 2u: { return oklab_rgb(encoded); }
        case 3u: { return cylindrical(oklab_rgb(encoded),encoded); }
        case 4u,5u: {
            let c = linear(encoded);
            let xyz = vec3(0.4124564*c.r+0.3575761*c.g+0.1804375*c.b,
                           0.2126729*c.r+0.7151522*c.g+0.072175*c.b,
                           0.0193339*c.r+0.119192*c.g+0.9503041*c.b);
            let f = lab_f(xyz/vec3(0.95047,1.0,1.08883));
            let lab = vec3(116.0*f.y-16.0,500.0*(f.x-f.y),200.0*(f.y-f.z));
            if space == 5u { return cylindrical(lab,encoded); }
            return lab;
        }
        default: {
            let y = dot(encoded,vec3(0.299,0.587,0.114));
            return vec3(y,0.5+(encoded.b-y)/1.772,0.5+(encoded.r-y)/1.402);
        }
    }
}
fn encode(c: vec3<f32>) -> vec3<f32> {
    return select(1.055*pow(max(c,vec3(0.0)),vec3(1.0/2.4))-0.055,12.92*c,c <= vec3(0.0031308));
}
fn lab_inverse(c: vec3<f32>) -> vec3<f32> {
    return select((116.0*c-16.0)/(24389.0/27.0), c*c*c, c > vec3(6.0/29.0));
}
fn reconstruct(input: vec3<f32>, space: u32) -> vec3<f32> {
    var c = input;
    if space == 3u || space == 5u {
        if c.y <= 0.0 { c = vec3(c.x,0.0,0.0); }
        else { c = vec3(c.x,c.y*cos(c.z),c.y*sin(c.z)); }
    }
    var result: vec3<f32>;
    switch space {
        case 0u: { result = c; }
        case 1u: { result = encode(c); }
        case 2u,3u: {
            let roots = vec3(c.x+0.39633778*c.y+0.21580376*c.z,
                             c.x-0.105561346*c.y-0.06385417*c.z,
                             c.x-0.08948418*c.y-1.2914855*c.z);
            let lms = roots*roots*roots;
            result = encode(vec3(4.0767417*lms.x-3.3077116*lms.y+0.23096994*lms.z,
                                  -1.268438*lms.x+2.6097574*lms.y-0.34131938*lms.z,
                                  -0.0041960863*lms.x-0.7034186*lms.y+1.7076147*lms.z));
        }
        case 4u,5u: {
            let fy = (c.x+16.0)/116.0;
            let xyz = lab_inverse(vec3(fy+c.y/500.0,fy,fy-c.z/200.0))*vec3(0.95047,1.0,1.08883);
            result = encode(vec3(3.240455*xyz.x-1.5371388*xyz.y-0.49853155*xyz.z,
                                  -0.9692664*xyz.x+1.8760109*xyz.y+0.041556083*xyz.z,
                                  0.05564342*xyz.x-0.20402585*xyz.y+1.0572251*xyz.z));
        }
        default: { result = vec3(c.x+1.402*(c.z-0.5),
            c.x-(0.114*1.772*(c.y-0.5)+0.299*1.402*(c.z-0.5))/0.587,
            c.x+1.772*(c.y-0.5)); }
    }
    return floor(clamp(result,vec3(0.0),vec3(1.0))*255.0+0.5)/255.0;
}
fn pow7(x: f32) -> f32 { let x2=x*x; return x2*x2*x2*x; }
fn hue_degrees(a: f32,b: f32) -> f32 {
    if a == 0.0 && b == 0.0 { return 0.0; }
    var h = atan2(b,a)*180.0/PI;
    if h < 0.0 { h += 360.0; }
    return h;
}
fn ciede2000(a: vec3<f32>,b: vec3<f32>) -> f32 {
    let cbar = (length(a.yz)+length(b.yz))*0.5;
    let c7 = pow7(cbar);
    let g = 0.5*(1.0-sqrt(c7/(c7+6103515625.0)));
    let a1 = (1.0+g)*a.y;
    let a2 = (1.0+g)*b.y;
    let c1 = length(vec2(a1,a.z));
    let c2 = length(vec2(a2,b.z));
    let h1 = hue_degrees(a1,a.z);
    let h2 = hue_degrees(a2,b.z);
    var dh = h2-h1;
    if c1*c2 == 0.0 { dh = 0.0; }
    else if dh > 180.0 { dh -= 360.0; }
    else if dh < -180.0 { dh += 360.0; }
    let delta_h = 2.0*sqrt(c1*c2)*sin(dh*0.5*PI/180.0);
    var hbar = (h1+h2)*0.5;
    if c1*c2 == 0.0 { hbar = h1+h2; }
    else if abs(h1-h2) > 180.0 {
        hbar = (h1+h2+select(-360.0,360.0,h1+h2<360.0))*0.5;
    }
    let cb = (c1+c2)*0.5;
    let light = (a.x+b.x)*0.5-50.0;
    let sl = 1.0+0.015*light*light/sqrt(20.0+light*light);
    let sc = 1.0+0.045*cb;
    let t = 1.0-0.17*cos((hbar-30.0)*PI/180.0)+0.24*cos(2.0*hbar*PI/180.0)
        +0.32*cos((3.0*hbar+6.0)*PI/180.0)-0.20*cos((4.0*hbar-63.0)*PI/180.0);
    let sh = 1.0+0.015*cb*t;
    let theta = 30.0*exp(-((hbar-275.0)/25.0)*((hbar-275.0)/25.0));
    let cb7=pow7(cb);
    let rt = -2.0*sqrt(cb7/(cb7+6103515625.0))*sin(2.0*theta*PI/180.0);
    let dl=(b.x-a.x)/sl;
    let dc=(c2-c1)/sc;
    let hh=delta_h/sh;
    return sqrt(max(0.0,dl*dl+dc*dc+hh*hh+rt*dc*hh));
}
fn metric(a: vec3<f32>,b: vec3<f32>, policy: u32) -> f32 {
    let d=a-b;
    switch policy {
        case 1u: {
            let mean=(a.x+b.x)*0.5*255.0;
            return (2.0+mean/256.0)*d.x*d.x+4.0*d.y*d.y+(2.0+(255.0-mean)/256.0)*d.z*d.z;
        }
        case 2u: { return dot(d*d,vec3(0.299,0.587,0.114)); }
        case 3u: { return dot(d*d,vec3(0.2126,0.7152,0.0722)); }
        case 7u,12u: {
            let angle = d.z-TAU*floor((d.z+PI)/TAU);
            let chord = 2.0*sqrt(max(0.0,a.y*b.y))*sin(angle*0.5);
            return d.x*d.x+d.y*d.y+chord*chord;
        }
        case 8u,13u: {
            let angle=abs(d.z);
            let arc=min(a.y,b.y)*min(angle,TAU-angle);
            return d.x*d.x+d.y*d.y+arc*arc;
        }
        case 10u: { return ciede2000(a,b); }
        default: { return dot(d,d); }
    }
}
fn field(x: u32,y: u32,i: u32) -> f32 {
    let kind=settings[2];
    if kind == 1u {
        let size=settings[3];
        var rank=0u;
        for (var bit=1u; bit<size; bit<<=1u) {
            let rx=select(0u,1u,(x&bit)!=0u);
            let ry=select(0u,1u,(y&bit)!=0u);
            rank=(rank<<2u)|((rx^ry)<<1u)|ry;
        }
        return f32(2*i32(rank)+1-i32(size*size))/f32(2u*size*size);
    }
    if kind == 2u {
        let shape=settings[3];
        var rank=0u;
        var cells=3u;
        if shape == 0u { let ranks=array(0u,2u,1u); rank=ranks[x%3u]; }
        if shape == 1u { let ranks=array(0u,2u,1u,3u); rank=ranks[x%4u]; cells=4u; }
        if shape == 2u { let ranks=array(0u,4u,2u,6u,3u,7u,1u,5u); rank=ranks[(y%2u)*4u+x%4u]; cells=8u; }
        if shape == 3u { let ranks=array(0u,12u,7u,3u,9u,14u,8u,1u,5u,11u,6u,4u,10u,13u,2u); rank=ranks[(y%3u)*5u+x%5u]; cells=15u; }
        return (f32(rank)+0.5)/f32(cells)-0.5;
    }
    if kind == 3u { return (f32(BLUE_NOISE[(y%32u)*32u+x%32u])+0.5)/1024.0-0.5; }
    if kind == 4u {
        var v=settings[4]+(i+1u)*0x6d2b79f5u;
        v=(v^(v>>15u))*(v|1u);
        v^=v+((v^(v>>7u))*(v|61u));
        v^=v>>14u;
        // Subtract in integer space before conversion to avoid losing the
        // low bits to rounding near 0.5 (CPU subtracts before its f32 cast).
        return f32(i32(v^0x80000000u))/4294967296.0;
    }
    return 0.0;
}
fn perturbed_color(i: u32) -> vec3<f32> {
    var color=rgb(i);
    let x=i%p.width;
    let y=i/p.width;
    var selected=true;
    if settings[6] == 2u {
        selected = (coverage[i/32u] & (1u << (i%32u))) != 0u;
    } else if settings[6] == 1u {
        let rank=BLUE_NOISE[((x+17u)%32u)*32u+(y+11u)%32u];
        selected=cells[3u*i+2u] > (f32(rank)+0.5)/1024.0;
    }
    let amount=field(x,y,i)*bitcast<f32>(settings[5])*0.25;
    if selected && amount != 0.0 {
        let space=settings[1];
        let range=vec3(bitcast<f32>(settings[12]),bitcast<f32>(settings[13]),bitcast<f32>(settings[14]));
        color=reconstruct(coordinates(color,space)+amount*range,space);
    }
    return color;
}
fn palette_index(i: u32) -> u32 {
    let packed=source[i];
    let alpha=packed>>24u;
    if settings[7] == 0u || (settings[9] == 0u && alpha<=settings[10]) { return settings[8]; }
    var color=perturbed_color(i);
    // The CPU perturbs RGBA first, then applies indexed alpha policy.
    if settings[9] != 0u {
        var bytes=vec3<u32>(floor(color*255.0+0.5))*alpha;
        if settings[9] == 2u {
            let matte=settings[11];
            bytes+=vec3(matte&255u,(matte>>8u)&255u,(matte>>16u)&255u)*(255u-alpha);
        }
        color=vec3<f32>((bytes+vec3(127u))/255u)/255.0;
    }
    let policy=settings[0];
    var match_space=0u;
    switch policy {
        case 4u: { match_space=1u; }
        case 5u: { match_space=2u; }
        case 6u,7u,8u: { match_space=3u; }
        case 9u,10u: { match_space=4u; }
        case 11u,12u,13u: { match_space=5u; }
        case 14u: { match_space=6u; }
        default: {}
    }
    let c=coordinates(color,match_space);
    var best=0u;
    var score=3.402823466e38;
    for (var n=0u;n<settings[7];n++) {
        let offset=16u+8u*n;
        let candidate=vec3(bitcast<f32>(settings[offset+2u]),bitcast<f32>(settings[offset+3u]),bitcast<f32>(settings[offset+4u]));
        let next=metric(c,candidate,policy);
        if next<score { score=next; best=settings[offset]; }
    }
    return best;
}
"#;

pub(super) const INDEXED: &str = r#"@group(0) @binding(2) var<storage, read_write> work: array<u32>;
@compute @workgroup_size(64)
fn indexed(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= p.minimum { return; }
    let word=p.diagonal+id.x;
    if word >= (count()+3u)/4u { return; }
    var packed=0u;
    for (var byte=0u;byte<4u;byte++) {
        let i=word*4u+byte;
        if i<count() { packed|=palette_index(i)<<(byte*8u); }
    }
    work[word]=packed;
}
"#;

pub(super) const PERTURB: &str = r#"@group(0) @binding(2) var<storage, read_write> work: array<u32>;
@compute @workgroup_size(64)
fn perturb(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=p.minimum {return;}
    let i=p.diagonal+id.x;
    if i>=count() {return;}
    let c=vec3<u32>(floor(clamp(perturbed_color(i),vec3(0.0),vec3(1.0))*255.0+0.5));
    work[i]=c.r|(c.g<<8u)|(c.b<<16u)|(source[i]&0xff000000u);
}
"#;
