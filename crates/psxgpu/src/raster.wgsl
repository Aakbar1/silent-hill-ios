// SPDX-License-Identifier: GPL-3.0-only
struct Draw {
    command: vec4<u32>, state: vec4<u32>, bounds: vec4<i32>,
    vertices: array<vec4<i32>,3>, colors: array<vec4<i32>,3>,
    planes: array<vec4<i32>,5>, fine: array<vec4<f32>,3>,
}
struct Config { a: vec4<u32>, b: vec4<u32>, c: vec4<u32> }
struct Fragment { color: vec3<i32>, uv: vec2<i32>, valid: bool }
@group(0) @binding(0) var<storage,read_write> native: array<u32>;
@group(0) @binding(1) var<storage,read> source: array<u32>;
@group(0) @binding(2) var<storage,read_write> scaled: array<u32>;
@group(0) @binding(3) var<storage,read> primitives: array<Draw>;
@group(0) @binding(4) var<storage,read> tiles: array<vec2<u32>>;
@group(0) @binding(5) var<storage,read> indices: array<u32>;
@group(0) @binding(6) var<uniform> config: Config;
@group(0) @binding(7) var<storage,read> pixels: array<u32>;
@group(0) @binding(8) var screen: texture_storage_2d<rgba8unorm,write>;

fn div_floor(a:i32,b:i32)->i32 {
    let q=a/b; return q-select(0,1,a-q*b<0);
}
fn edge(a:vec2<i32>,b:vec2<i32>,p:vec2<i32>)->i32 {
    return (b.x-a.x)*(p.y-a.y)-(b.y-a.y)*(p.x-a.x);
}
fn edgef(a:vec2<f32>,b:vec2<f32>,p:vec2<f32>)->f32 {
    return (b.x-a.x)*(p.y-a.y)-(b.y-a.y)*(p.x-a.x);
}
fn top_left(a:vec2<i32>,b:vec2<i32>)->bool { return b.y<a.y || (b.y==a.y && b.x>a.x); }
fn interp_value(plane:vec4<i32>,pos:vec2<i32>,origin:vec2<i32>,scale:i32)->i32 {
    if all(vec2<u32>(pos)%u32(scale)==vec2<u32>(0)) {
        let delta=pos/scale-origin;
        return (plane.x+plane.y*delta.x+plane.z*delta.y)>>12u;
    }
    let delta=vec2<f32>(pos)/f32(scale)-vec2<f32>(origin);
    return i32(floor((f32(plane.x)+f32(plane.y)*delta.x+f32(plane.z)*delta.y)/4096.0));
}
fn fragment(d:Draw,pos:vec2<i32>,scale:i32,fine:bool)->Fragment {
    var f:Fragment; f.valid=false; f.color=vec3<i32>(0); f.uv=vec2<i32>(0);
    let ip=pos/scale;
    if (d.command.z&0x20000u)!=0u && (u32(ip.y)&1u)==((d.command.z>>18u)&1u) {return f;}
    if any(ip<d.bounds.xy) || any(ip>=d.bounds.zw) { return f; }
    let a=d.vertices[0].xy; let b=d.vertices[1].xy; let c=d.vertices[2].xy;
    if d.command.x==1u {
        let delta=ip-a;
        if any(delta<vec2<i32>(0)) || any(delta>=vec2<i32>(d.state.zw)) {return f;}
        f.color=d.colors[0].xyz;
        f.uv=d.vertices[0].zw+delta*vec2<i32>(select(1,-1,(d.command.z&4096u)!=0u),select(1,-1,(d.command.z&8192u)!=0u));
    } else if d.command.x==2u {
        let delta=b-a; let n=max(abs(delta.x),abs(delta.y));
        let i=select((ip.y-a.y)*sign(delta.y),(ip.x-a.x)*sign(delta.x),abs(delta.x)>=abs(delta.y));
        if i<select(0,1,(d.command.y&512u)!=0u) || i>n {return f;}
        let div=max(n,1);
        let point=a+vec2<i32>(div_floor(i*delta.x*2+div-1,div*2),div_floor(i*delta.y*2+div-select(0,1,delta.y<0),div*2));
        if any(point!=ip) {return f;}
        let step=(d.colors[1].xyz-d.colors[0].xyz)*vec3<i32>(4096)/vec3<i32>(div);
        f.color=(d.colors[0].xyz*vec3<i32>(4096)+vec3<i32>(2048)+step*i)>>vec3<u32>(12u);
    } else {
        if fine {
            let aa=d.fine[0].xy; let bb=d.fine[1].xy; let cc=d.fine[2].xy;
            let pp=vec2<f32>(pos)/f32(scale);
            let area=edgef(aa,bb,cc);
            if area<=0.0 {return f;}
            let e0=edgef(bb,cc,pp); let e1=edgef(cc,aa,pp); let e2=edgef(aa,bb,pp);
            if e0<0.0 || (e0==0.0 && !(cc.y<bb.y || (cc.y==bb.y && cc.x>bb.x)))
                || e1<0.0 || (e1==0.0 && !(aa.y<cc.y || (aa.y==cc.y && aa.x>cc.x)))
                || e2<0.0 || (e2==0.0 && !(bb.y<aa.y || (bb.y==aa.y && bb.x>aa.x))) {return f;}
            let weight=vec3<f32>(e0,e1,e2)/area;
            f.color=vec3<i32>(floor(vec3<f32>(d.colors[0].xyz)*weight.x+vec3<f32>(d.colors[1].xyz)*weight.y+vec3<f32>(d.colors[2].xyz)*weight.z));
            f.uv=vec2<i32>(floor(vec2<f32>(d.vertices[0].zw)*weight.x+vec2<f32>(d.vertices[1].zw)*weight.y+vec2<f32>(d.vertices[2].zw)*weight.z));
        } else {
            if edge(a,b,c)<=0 {return f;}
            let aa=a*scale; let bb=b*scale; let cc=c*scale;
            let e0=edge(aa,bb,pos); let e1=edge(bb,cc,pos); let e2=edge(cc,aa,pos);
            if e0<0 || (e0==0 && !top_left(a,b)) || e1<0 || (e1==0 && !top_left(b,c)) || e2<0 || (e2==0 && !top_left(c,a)) {return f;}
            f.color=vec3<i32>(interp_value(d.planes[0],pos,a,scale),interp_value(d.planes[1],pos,a,scale),interp_value(d.planes[2],pos,a,scale));
            f.uv=vec2<i32>(interp_value(d.planes[3],pos,a,scale),interp_value(d.planes[4],pos,a,scale));
        }
    }
    f.valid=true; return f;
}
fn textured(d:Draw)->bool {
    return (d.command.y&4u)!=0u && d.command.x!=2u && (d.command.z&0x10800u)!=0x10800u;
}
fn texture(d:Draw,uv:vec2<i32>)->u32 {
    let w=d.command.w;
    let u=((u32(uv.x)&~((w&31u)<<3u))|(((w>>10u)&(w&31u))<<3u))&255u;
    let v=((u32(uv.y)&~(((w>>5u)&31u)<<3u))|(((w>>15u)&((w>>5u)&31u))<<3u))&255u;
    let page=d.command.z; let depth=(page>>7u)&3u;
    let divisor=select(select(1u,2u,depth==1u),4u,depth==0u);
    let x=((page&15u)*64u+u/divisor)&1023u; let y=((page&16u)*16u+v)&511u;
    let word=source[y*1024u+x];
    if depth>=2u {return word;}
    let index=select((word>>((u&1u)*8u))&255u,(word>>((u&3u)*4u))&15u,depth==0u);
    return source[((d.state.y>>6u)&511u)*1024u+(((d.state.y&63u)*16u+index)&1023u)];
}
fn shade(d:Draw,dst:u32,f:Fragment,pos:vec2<i32>)->u32 {
    if !f.valid || ((d.state.x&2u)!=0u && (dst&0x8000u)!=0u) {return dst;}
    let tex=textured(d); var texel=0u;
    if tex {texel=texture(d,f.uv); if texel==0u {return dst;}}
    let raw=(d.command.y&1u)!=0u;
    let dith=(d.command.z&512u)!=0u && d.command.x!=1u && (d.command.x==2u || (d.command.y&16u)!=0u || (tex && !raw));
    let matrix=array<i32,16>(-4,0,-3,1,2,-2,3,-1,-3,1,-4,0,3,-1,2,-2);
    let offset=select(0,matrix[u32((pos.y&3)*4+(pos.x&3))],dith);
    let blend=(d.command.y&2u)!=0u && (!tex || (texel&0x8000u)!=0u);
    let t=vec3<i32>(i32(texel&31u),i32((texel>>5u)&31u),i32((texel>>10u)&31u));
    var value=t;
    if !(tex && raw) {
        let col=select(f.color,(t*clamp(f.color,vec3<i32>(0),vec3<i32>(255)))>>vec3<u32>(4u),vec3<bool>(tex));
        value=clamp(col+vec3<i32>(offset),vec3<i32>(0),vec3<i32>(255))>>vec3<u32>(3u);
    }
    if blend {
        let bg=vec3<i32>(i32(dst&31u),i32((dst>>5u)&31u),i32((dst>>10u)&31u));
        switch (d.command.z>>5u)&3u {
            case 0u: {value=(bg+value)>>vec3<u32>(1u);}
            case 1u: {value=bg+value;}
            case 2u: {value=bg-value;}
            default: {value=bg+(value>>vec3<u32>(2u));}
        }
        value=clamp(value,vec3<i32>(0),vec3<i32>(31));
    }
    let result=u32(value.x)|(u32(value.y)<<5u)|(u32(value.z)<<10u);
    return result|select(select(0u,texel&0x8000u,tex),0x8000u,(d.state.x&1u)!=0u);
}
@compute @workgroup_size(8,8)
fn draw(@builtin(global_invocation_id) gid:vec3<u32>) {
    if gid.x>=config.a.w || gid.y>=config.b.x {return;}
    let scale=config.a.x; let pos=gid.xy+config.a.yz; let ip=pos/scale;
    let index=pos.y*(1024u*scale)+pos.x;
    let ni=ip.y*1024u+ip.x;
    let anchor=all(pos%scale==vec2<u32>(0));
    var output=scaled[index]; var reference=0u;
    if anchor {reference=native[ni];}
    let tile=tiles[(ip.y/4u)*256u+ip.x/4u];
    for(var j=0u;j<tile.y;j++) {
        let d=primitives[indices[tile.x+j]];
        let f=fragment(d,vec2<i32>(pos),i32(scale),(d.command.y&256u)!=0u);
        output=shade(d,output,f,vec2<i32>(ip));
        if anchor {reference=shade(d,reference,fragment(d,vec2<i32>(ip),1,false),vec2<i32>(ip));}
    }
    scaled[index]=output;
    if anchor {native[ni]=reference;}
}

@compute @workgroup_size(8,8)
fn transfer(@builtin(global_invocation_id) gid:vec3<u32>) {
    // kind,scale,x,y / width,height,source_x,source_y / mask,color,unused,unused
    let scale=config.a.y;
    let size=config.b.xy*scale;
    if any(gid.xy>=size) {return;}
    let delta=gid.xy/scale;
    let dest=(config.a.zw+delta)&vec2<u32>(1023u,511u);
    let local=gid.xy%scale;
    if config.a.x==0u && (config.c.z&1u)!=0u && (dest.y&1u)==(config.c.z>>1u) {return;}
    let ni=dest.y*1024u+dest.x;
    let sp=dest*scale+local;
    let si=sp.y*(1024u*scale)+sp.x;
    var value=config.c.y;
    var nv=value;
    if config.a.x==1u {value=pixels[delta.y*config.b.x+delta.x];nv=value;}
    if config.a.x==3u {value=pixels[gid.y*(config.b.x*scale)+gid.x];}
    if config.a.x==2u {
        let src=(config.b.zw+delta)&vec2<u32>(1023u,511u);
        let srcp=src*scale+local;
        value=scaled[srcp.y*(1024u*scale)+srcp.x];nv=native[src.y*1024u+src.x];
    }
    let mask=config.c.x;
    if config.a.x==0u || (mask&2u)==0u || (scaled[si]&0x8000u)==0u {
        scaled[si]=value|select(0u,0x8000u,(mask&1u)!=0u);
    }
    if config.a.x!=3u && all(local==vec2<u32>(0)) {
        if config.a.x==0u || (mask&2u)==0u || (native[ni]&0x8000u)==0u {
            native[ni]=nv|select(0u,0x8000u,(mask&1u)!=0u);
        }
    }
}

@compute @workgroup_size(8,8)
fn scanout(@builtin(global_invocation_id) gid:vec3<u32>) {
    let s=config.a.x; let width=config.b.x; let height=config.b.y;
    if gid.x>=width*s || gid.y>=height*s {return;}
    let mode=config.a.w;let y=gid.y/s;
    let x=select(gid.x,width*s-1u-gid.x,(mode&128u)!=0u);
    let row=(config.a.z+y)&511u;
    var color=vec3<u32>(0);
    if config.b.z==0u && !((mode&36u)==36u && (y&1u)!=config.b.w) {
        if (mode&16u)!=0u {
            let byte=config.a.y*2u+(x/s)*3u;
            let b0=byte&2047u;let b1=(byte+1u)&2047u;let b2=(byte+2u)&2047u;
            color=vec3<u32>((native[row*1024u+b0/2u]>>((b0&1u)*8u))&255u,
                (native[row*1024u+b1/2u]>>((b1&1u)*8u))&255u,
                (native[row*1024u+b2/2u]>>((b2&1u)*8u))&255u);
        } else {
            let xx=(config.a.y*s+x)%(1024u*s);
            let word=scaled[(row*s+gid.y%s)*(1024u*s)+xx];
            let c=vec3<u32>(word&31u,(word>>5u)&31u,(word>>10u)&31u);
            color=(c<<vec3<u32>(3u))|(c>>vec3<u32>(2u));
        }
    }
    textureStore(screen,vec2<i32>(gid.xy),vec4<f32>(vec3<f32>(color)/255.0,1.0));
}
