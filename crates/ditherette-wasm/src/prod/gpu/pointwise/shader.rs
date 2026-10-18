//! Continuous encoded-sRGB effect carrier; no clipping/rounding between steps.
pub(super) const WGSL: &str = r#"
@group(0) @binding(2) var<storage, read> code: array<u32>;
const TAU: f32 = 6.283185307179586;
fn f(p:u32)->f32 { return bitcast<f32>(code[p]); }
fn v3(p:u32)->vec3<f32> { return vec3(f(p),f(p+1u),f(p+2u)); }
fn wrap(x:f32)->f32 { return x-floor(x); }
fn decode(c:vec3<f32>)->vec3<f32> {
    return select(pow(max((c+0.055)/1.055,vec3(0.0)),vec3(2.4)),c/12.92,c<=vec3(0.04045));
}
fn encode(c:vec3<f32>)->vec3<f32> {
    return select(1.055*pow(max(c,vec3(0.0)),vec3(1.0/2.4))-0.055,c*12.92,c<=vec3(0.0031308));
}
fn lab(linear:vec3<f32>,cie:bool)->vec3<f32> {
    if cie {
        let xyz=vec3(0.4124564*linear.r+0.3575761*linear.g+0.1804375*linear.b,
            0.2126729*linear.r+0.7151522*linear.g+0.072175*linear.b,
            0.0193339*linear.r+0.119192*linear.g+0.9503041*linear.b)/vec3(0.95047,1.0,1.08883);
        let z=select((24389.0/27.0*xyz+16.0)/116.0,
            pow(max(xyz,vec3(0.0)),vec3(1.0/3.0)),xyz>vec3(216.0/24389.0));
        return vec3(116.0*z.y-16.0,500.0*(z.x-z.y),200.0*(z.y-z.z));
    }
    let lms=vec3(0.41222146*linear.r+0.53633255*linear.g+0.051445995*linear.b,
        0.2119035*linear.r+0.6806995*linear.g+0.10739696*linear.b,
        0.08830246*linear.r+0.28171885*linear.g+0.6299787*linear.b);
    let z=sign(lms)*pow(abs(lms),vec3(1.0/3.0));
    return vec3(0.21045426*z.x+0.7936178*z.y-0.004072047*z.z,
        1.9779985*z.x-2.4285922*z.y+0.4505937*z.z,
        0.025904037*z.x+0.78277177*z.y-0.80867577*z.z);
}
fn inverse_lab(c:vec3<f32>,cie:bool)->vec3<f32> {
    if cie {
        let fy=(c.x+16.0)/116.0;
        let z=vec3(fy+c.y/500.0,fy,fy-c.z/200.0);
        let xyz=select((116.0*z-16.0)/(24389.0/27.0),z*z*z,z>vec3(6.0/29.0))*vec3(0.95047,1.0,1.08883);
        return vec3(3.240455*xyz.x-1.5371388*xyz.y-0.49853155*xyz.z,
            -0.9692664*xyz.x+1.8760109*xyz.y+0.041556083*xyz.z,
            0.05564342*xyz.x-0.20402585*xyz.y+1.0572251*xyz.z);
    }
    let z=vec3(c.x+0.39633778*c.y+0.21580376*c.z,
        c.x-0.105561346*c.y-0.06385417*c.z,c.x-0.08948418*c.y-1.2914855*c.z);
    let lms=z*z*z;
    return vec3(4.0767417*lms.x-3.3077116*lms.y+0.23096994*lms.z,
        -1.268438*lms.x+2.6097574*lms.y-0.34131938*lms.z,
        -0.0041960863*lms.x-0.7034186*lms.y+1.7076147*lms.z);
}
fn luma(c:vec3<f32>,kr:f32,kb:f32)->vec3<f32> {
    let y=kr*c.r+(1.0-kr-kb)*c.g+kb*c.b;
    return vec3(y,(c.b-y)/(2.0-2.0*kb),(c.r-y)/(2.0-2.0*kr));
}
fn inverse_luma(c:vec3<f32>,kr:f32,kb:f32)->vec3<f32> {
    let r=c.x+(2.0-2.0*kr)*c.z; let b=c.x+(2.0-2.0*kb)*c.y;
    return vec3(r,(c.x-kr*r-kb*b)/(1.0-kr-kb),b);
}
fn normalized(input:vec3<f32>,model:u32)->vec3<f32> {
    switch model {
        case 0u: { return input; }
        case 1u: { return decode(input); }
        case 2u,3u: {
            let rgb=clamp(input,vec3(0.0),vec3(1.0));
            let hi=max(rgb.r,max(rgb.g,rgb.b)); let lo=min(rgb.r,min(rgb.g,rgb.b));
            let chroma=hi-lo; var hue=0.0; var saturation=0.0;
            if chroma!=0.0 {
                if hi==rgb.r { hue=wrap((rgb.g-rgb.b)/chroma/6.0); }
                else if hi==rgb.g { hue=((rgb.b-rgb.r)/chroma+2.0)/6.0; }
                else { hue=((rgb.r-rgb.g)/chroma+4.0)/6.0; }
                if model==3u { saturation=chroma/hi; }
                else { saturation=chroma/(1.0-abs(2.0*((hi+lo)/2.0)-1.0)); }
            }
            return vec3(hue,saturation,select((hi+lo)/2.0,hi,model==3u));
        }
        case 4u,5u,6u,7u: {
            let c=lab(decode(input),model>=6u);
            if model==4u { return vec3(c.x,c.y/0.8+0.5,c.z/0.8+0.5); }
            if model==6u { return vec3(c.x/100.0,c.y/250.0+0.5,c.z/250.0+0.5); }
            var hue=0.0;
            if c.y!=0.0 || c.z!=0.0 { hue=wrap(atan2(c.z,c.y)/TAU); }
            if model==5u { return vec3(c.x,length(c.yz)/0.4,hue); }
            return vec3(c.x/100.0,length(c.yz)/150.0,hue);
        }
        default: {
            let y=0.299*input.r+0.587*input.g+0.114*input.b;
            return vec3(y,(input.b-y)/1.772+0.5,(input.r-y)/1.402+0.5);
        }
    }
}
fn from_normalized(c:vec3<f32>,model:u32)->vec3<f32> {
    switch model {
        case 0u: { return c; }
        case 1u: { return encode(c); }
        case 2u,3u: {
            var chroma=c.z*c.y; var minimum=c.z-chroma;
            if model==2u { chroma=(1.0-abs(2.0*c.z-1.0))*c.y; minimum=c.z-chroma/2.0; }
            let sector=wrap(c.x)*6.0; let x=chroma*(1.0-abs((sector-2.0*floor(sector/2.0))-1.0));
            var rgb:vec3<f32>;
            switch u32(sector) {
                case 0u: {rgb=vec3(chroma,x,0.0);} case 1u: {rgb=vec3(x,chroma,0.0);}
                case 2u: {rgb=vec3(0.0,chroma,x);} case 3u: {rgb=vec3(0.0,x,chroma);}
                case 4u: {rgb=vec3(x,0.0,chroma);} default: {rgb=vec3(chroma,0.0,x);}
            }
            return rgb+minimum;
        }
        case 4u: { return encode(inverse_lab(vec3(c.x,(c.y-0.5)*0.8,(c.z-0.5)*0.8),false)); }
        case 6u: { return encode(inverse_lab(vec3(c.x*100.0,(c.y-0.5)*250.0,(c.z-0.5)*250.0),true)); }
        case 5u,7u: {
            let angle=wrap(c.z)*TAU;
            let factor=select(0.4,150.0,model==7u);
            return encode(inverse_lab(vec3(c.x*select(1.0,100.0,model==7u),
                c.y*factor*cos(angle),c.y*factor*sin(angle)),model==7u));
        }
        default: {
            let r=c.x+1.402*(c.z-0.5); let b=c.x+1.772*(c.y-0.5);
            return vec3(r,(c.x-0.299*r-0.114*b)/0.587,b);
        }
    }
}
fn hue_weight(rgb:vec3<f32>,c:vec3<f32>,channel:u32)->f32 {
    if rgb.r==rgb.g && rgb.g==rgb.b { return 0.0; }
    let model=channel&15u;
    if model==2u || model==3u {
        let clamped=clamp(rgb,vec3(0.0),vec3(1.0));
        return clamp((max(clamped.r,max(clamped.g,clamped.b))-min(clamped.r,min(clamped.g,clamped.b)))/0.02,0.0,1.0);
    }
    if model==5u { return clamp(c.y*0.4/0.02,0.0,1.0); }
    if model==7u { return clamp(c.y*150.0/2.0,0.0,1.0); }
    return 1.0;
}
fn tangent(before:f32,after:f32,hb:f32,ha:f32)->f32 {
    if before*after<=0.0 {return 0.0;}
    let wb=2.0*ha+hb; let wa=ha+2.0*hb;
    return (wb+wa)/(wb/before+wa/after);
}
fn hermite(x0:f32,y0:f32,x1:f32,y1:f32,m0:f32,m1:f32,x:f32)->f32 {
    let h=x1-x0; let secant=(y1-y0)/h;
    let c2=(3.0*secant-2.0*m0-m1)/h; let c3=(m0+m1-2.0*secant)/(h*h); let s=x-x0;
    return y0+s*(m0+s*(c2+s*c3));
}
struct Axis { previous:u32,k:u32,next:u32,after:u32,x:f32,x0:f32,x1:f32,hb:f32,ha:f32 }
fn axis(positions:u32,count:u32,cyclic:bool,input:f32)->Axis {
    var x=input; var k=0u;
    if cyclic {
        x=wrap(x); k=count-1u;
        if x<f(positions) {x+=1.0;}
        else { for(var i=0u;i+1u<count;i++) {if x<f(positions+i+1u) {k=i;break;}} }
    } else {
        x=clamp(x,f(positions),f(positions+count-1u)); k=count-2u;
        for(var i=0u;i+1u<count;i++) {if x<f(positions+i+1u) {k=i;break;}}
    }
    let next=(k+1u)%count; let previous=(k+count-1u)%count; let after=(next+1u)%count;
    let x0=f(positions+k); let x1=f(positions+next)+select(0.0,1.0,next==0u);
    let hb=x0-f(positions+previous)+select(0.0,1.0,k==0u);
    let ha=f(positions+after)-f(positions+next)+select(0.0,1.0,after==0u);
    return Axis(previous,k,next,after,x,x0,x1,hb,ha);
}
fn interpolate(a:Axis,y:vec4<f32>,count:u32,cyclic:bool)->f32 {
    let secant=(y.z-y.y)/(a.x1-a.x0); var m0=secant; var m1=secant;
    if cyclic || a.k>0u {m0=tangent((y.y-y.x)/a.hb,secant,a.hb,a.x1-a.x0);}
    if cyclic || a.next+1u<count {m1=tangent(secant,(y.w-y.z)/a.ha,a.x1-a.x0,a.ha);}
    if !cyclic && a.x>=a.x1 && a.next+1u==count {return y.z;}
    return hermite(a.x0,y.y,a.x1,y.z,m0,m1,a.x);
}
fn sequence(pos:u32,values:u32,count:u32,cyclic:bool,x:f32)->f32 {
    let a=axis(pos,count,cyclic,x);
    return interpolate(a,vec4(f(values+a.previous),f(values+a.k),f(values+a.next),f(values+a.after)),count,cyclic);
}
fn curve_value(p:u32,x:f32,y:f32)->f32 {
    let pos=code[p]; let values=code[p+1u]; let count=code[p+2u]; let cyclic=code[p+3u]!=0u;
    if code[p+7u]==0u {return sequence(pos,values,count,cyclic,x);}
    let rows=code[p+5u]; let row_cyclic=code[p+6u]!=0u;
    let a=axis(code[p+4u],rows,row_cyclic,y);
    let four=vec4(sequence(pos,values+a.previous*count,count,cyclic,x),
        sequence(pos,values+a.k*count,count,cyclic,x),sequence(pos,values+a.next*count,count,cyclic,x),
        sequence(pos,values+a.after*count,count,cyclic,x));
    return clamp(interpolate(a,four,rows,row_cyclic),0.0,1.0);
}
fn strength(p:u32,rgb:vec3<f32>)->f32 {
    var result=1.0;
    for(var i=0u;i<code[p+3u];i++) {
        let mask=code[code[p+4u]+i]; let x=code[mask]; let x2=code[mask+1u];
        let c=normalized(rgb,x&15u); var second=vec3(0.0); var weight=1.0;
        if x>>6u==1u {weight=hue_weight(rgb,c,x);}
        if code[mask+3u]!=0u {
            second=normalized(rgb,x2&15u);
            if x2>>6u==1u {weight=min(weight,hue_weight(rgb,second,x2));}
        }
        let value=clamp(curve_value(code[mask+2u],c[(x>>4u)&3u],second[(x2>>4u)&3u]),0.0,1.0);
        if weight==1.0 {result*=value;} else {result*=1.0-weight*(1.0-value);}
    }
    return result;
}
fn toward(neutral:f32,value:f32,s:f32)->f32 {
    if s==1.0 {return value;} return neutral+s*(value-neutral);
}
fn curves(p:u32,source:vec3<f32>,s:f32)->vec3<f32> {
    var current=source;
    for(var i=0u;i<code[p+1u];i++) {
        let c=code[p+2u+i]; let flags=code[c]; let x=code[c+1u]; let y=code[c+2u]; let x2=code[c+3u];
        if (flags&4u)!=0u && all(bitcast<vec3<u32>>(current)==bitcast<vec3<u32>>(source)) {continue;}
        let xc=normalized(source,x&15u); let original=xc[(x>>4u)&3u]; var weight=1.0; var second=vec3(0.0);
        if x>>6u==1u {weight=hue_weight(source,xc,x);}
        if (flags&2u)!=0u {
            second=normalized(source,x2&15u);
            if x2>>6u==1u {weight=min(weight,hue_weight(source,second,x2));}
        }
        if weight==0.0 {continue;}
        let value=curve_value(code[c+4u],original,second[(x2>>4u)&3u]);
        var yc=normalized(current,y&15u); let yi=(y>>4u)&3u; let kind=y>>6u;
        if (flags&1u)==0u {
            if kind==1u {yc[yi]=wrap(original+weight*s*(wrap(value-original+0.5)-0.5));}
            else {yc[yi]=toward(original,value,s);}
        } else {
            if kind==1u {weight=min(weight,hue_weight(current,yc,y)); if weight==0.0 {continue;}}
            let adjusted=toward(0.5,value,s);
            if kind==1u {yc[yi]=wrap(yc[yi]+weight*(adjusted-0.5));}
            else if kind==2u {yc[yi]*=1.0+weight*(2.0*adjusted-1.0);}
            else {yc[yi]+=weight*(adjusted-0.5);}
        }
        if s>1.0 {
            if kind==2u {yc[yi]=max(0.0,yc[yi]);}
            else if kind!=1u {yc[yi]=clamp(yc[yi],0.0,1.0);}
        }
        current=from_normalized(yc,y&15u);
    }
    return current;
}
fn blend(input:vec3<f32>,output:vec3<f32>,s:f32)->vec3<f32> {
    if s==1.0 {return output;} if s==0.0 {return input;} return input+s*(output-input);
}
fn levels_unit(value:f32,p:u32)->f32 {
    let bits=bitcast<u32>(value);
    if (bits&0x80000000u)!=0u || bits<=code[p] {return 0.0;}
    if bits>=code[p+1u] {return 1.0;}
    var v=value;
    if f(p+8u)>1.0 {
        if (bits>>23u)==0u {v=f32(bits&0x7fffffu)*0.00000011920928955078125;}
        else {v*=f(p+8u);}
    }
    return clamp((v-f(p+6u))/(f(p+7u)-f(p+6u)),0.0,1.0);
}
fn recolour(p:u32,rgb:vec3<f32>,s:f32)->vec3<f32> {
    let model=code[p+7u]; var c:vec3<f32>;
    switch model {
        case 0u: {c=luma(rgb,0.299,0.114);}
        case 1u: {c=luma(decode(rgb),0.2126,0.0722);}
        default: {c=lab(decode(rgb),model==6u); if model==6u {c/=100.0;}}
    }
    c.x=curve_value(code[p+4u],c.x,0.0);
    var uv=(c.yz+vec2(f(p+2u),f(p+3u)))*f(p+1u);
    let ramp=min(length(uv)/0.02,1.0);
    if code[p+6u]>0u && ramp!=0.0 {
        let hue=wrap(atan2(uv.y,uv.x)/TAU)*360.0; var turn=0.0; var scale=1.0;
        for(var i=0u;i<code[p+6u];i++) {
            let g=code[p+5u]+4u*i; let distance=abs(wrap((hue-f(g)+180.0)/360.0)*360.0-180.0);
            if distance>=f(g+1u) {continue;}
            let weight=0.5*(1.0+cos((TAU/2.0)*distance/f(g+1u)))*ramp;
            turn+=weight*f(g+2u); scale+=weight*(f(g+3u)-1.0);
        }
        let angle=turn*(TAU/360.0); let sine=sin(angle); let cosine=cos(angle);
        uv=vec2(uv.x*cosine-uv.y*sine,uv.x*sine+uv.y*cosine)*max(scale,0.0);
    }
    c=vec3(c.x,uv); var out:vec3<f32>;
    switch model {
        case 0u: {out=inverse_luma(c,0.299,0.114);}
        case 1u: {out=encode(inverse_luma(c,0.2126,0.0722));}
        default: {if model==6u {c*=100.0;} out=encode(inverse_lab(c,model==6u));}
    }
    return blend(rgb,out,s);
}
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    if !inside(id.xy) {return;}
    let bytes=load(id.xy);
    var rgb=vec3(f(3u+bytes.r),f(259u+bytes.g),f(515u+bytes.b));
    var step=code[1u];
    for(var i=0u;i<code[0u];i++) {
        let tag=code[step+1u]; let p=code[step+2u]; let m=strength(step,rgb); var out=rgb;
        if tag==8u {out=vec3(m);}
        else if m!=0.0 {
            switch tag {
                case 0u: {
                    var t=vec3(levels_unit(rgb.r,p),levels_unit(rgb.g,p),levels_unit(rgb.b,p));
                    if f(p+2u)!=1.0 {t=pow(t,vec3(1.0/f(p+2u)));}
                    let mapped=f(p+3u)+(f(p+4u)-f(p+3u))*t;
                    for(var k=0u;k<3u;k++) {if (code[p+5u]&(1u<<k))!=0u {out[k]=mapped[k];}}
                }
                case 1u: {if f(p)!=0.0 || f(p+1u)!=1.0 {out=(rgb-0.5)*f(p+1u)+0.5+f(p);}}
                case 2u: {if f(p)!=1.0 {out=encode(decode(rgb)*f(p));}}
                case 3u: {
                    let gain=v3(p); let mapped=encode(decode(rgb)*gain);
                    out=select(mapped,rgb,gain==vec3(1.0));
                }
                case 4u: {
                    if f(p)!=0.0 || f(p+1u)!=1.0 || f(p+2u)!=1.0 || f(p+3u)!=0.0 {
                        var linear=decode(rgb); if i==0u && code[2u]!=0u {linear=rgb;}
                        let c=lab(linear,false);
                        var turned=vec3(c.x,(c.y*f(p+1u)-c.z*f(p))*f(p+2u),
                            (c.y*f(p)+c.z*f(p+1u))*f(p+2u));
                        let lightness=f(p+3u);
                        if lightness>=0.0 {turned=vec3(turned.x+(1.0-turned.x)*lightness,turned.yz*(1.0-lightness));}
                        else {turned*=1.0+lightness;}
                        out=encode(inverse_lab(turned,false));
                    }
                }
                case 5u: {let s=m*f(p); if s!=0.0 {out=curves(p,rgb,s);}}
                case 6u: {out=recolour(p,rgb,m*f(p));}
                default: {}
            }
            if tag!=5u && tag!=6u {out=blend(rgb,out,m);}
        }
        rgb=clamp(out,vec3(-64.0),vec3(64.0)); step=code[step];
    }
    store(id.xy,vec4(vec3<u32>(floor(clamp(rgb,vec3(0.0),vec3(1.0))*255.0+0.5)),bytes.a));
}
"#;
