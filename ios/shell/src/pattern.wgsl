// SPDX-License-Identifier: GPL-3.0-only
struct Frame { size: vec4<f32>, safe: vec4<f32> }
@group(0) @binding(0) var<uniform> frame: Frame;
@vertex fn background(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>, 3>(vec2(-1., -1.), vec2(3., -1.), vec2(-1., 3.));
    return vec4(p[id], 0., 1.);
}
@fragment fn pattern(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let start = frame.safe.xy;
    let end = frame.size.xy - frame.safe.zw;
    if (any(p.xy < start) || any(p.xy >= end)) { return vec4(.025, .035, .06, 1.); }
    if (any(p.xy < start + vec2(3.)) || any(p.xy > end - vec2(3.))) {
        return vec4(.25, 1., .8, 1.);
    }
    let uv = (p.xy - start) / (end - start);
    let bars = array<vec3<f32>, 7>(vec3(.85), vec3(.85,.85,.1), vec3(.1,.85,.85),
        vec3(.1,.85,.1), vec3(.85,.1,.85), vec3(.85,.1,.1), vec3(.1,.1,.85));
    var color = bars[min(u32(uv.x * 7.), 6u)];
    if (uv.y > .65) {
        let checker = (u32(p.x / 32.) + u32(p.y / 32.)) % 2u;
        color = vec3(select(.08, .2, checker == 1u));
    }
    return vec4(color, 1.);
}
@vertex fn circle(@location(0) p: vec2<f32>) -> @builtin(position) vec4<f32> { return vec4(p, 0., 1.); }
@fragment fn touch_color() -> @location(0) vec4<f32> { return vec4(1., .45, .05, 1.); }
