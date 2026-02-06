struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Uniforms {
    rect: vec4<f32>,
}

@group(0) @binding(0)
var y_tex: texture_2d<f32>;

@group(0) @binding(1)
var u_tex: texture_2d<f32>;

@group(0) @binding(2)
var v_tex: texture_2d<f32>;

@group(0) @binding(3)
var samp: sampler;

@group(0) @binding(4)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2<f32>(0.0);
    out.uv.x = select(0.0, 2.0, idx == 1u);
    out.uv.y = select(0.0, 2.0, idx == 2u);
    out.position = vec4<f32>(out.uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 1.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let y = textureSample(y_tex, samp, in.uv).r;
    let u = textureSample(u_tex, samp, in.uv).r;
    let v = textureSample(v_tex, samp, in.uv).r;
    // BT.709 limited range
    let yn = (y - 0.0627) * 1.1644;
    let cb = u - 0.5;
    let cr = v - 0.5;
    let r = yn + 1.7927 * cr;
    let g = yn - 0.2132 * cb - 0.5329 * cr;
    let b = yn + 2.1124 * cb;
    return vec4<f32>(clamp(r, 0.0, 1.0), clamp(g, 0.0, 1.0), clamp(b, 0.0, 1.0), 1.0);
}
