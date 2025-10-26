// @group(0) layout:
// 0: sampler
// 1: texY
// 2: texU
// 3: texV
// @group(1) layout:

@group(0) @binding(0) var samp : sampler;
@group(0) @binding(1) var texY : texture_2d<f32>;
@group(0) @binding(2) var texU : texture_2d<f32>;
@group(0) @binding(3) var texV : texture_2d<f32>;

struct VSOut {
    @builtin(position) pos : vec4<f32>,
    @location(0) uv : vec2<f32>,
};

@vertex
fn vs_main(@location(0) in_pos: vec2<f32>,
           @location(1) in_uv : vec2<f32>) -> VSOut {
    var out: VSOut;
    out.pos = vec4<f32>(in_pos, 0.0, 1.0);
    out.uv  = in_uv;
    return out;
}

fn yuv_to_rgb709(yuv: vec3<f32>) -> vec3<f32> {
    let y  = (yuv.x - 16.0/255.0) * 1.1643836;
    let cb = (yuv.y - 128.0/255.0);
    let cr = (yuv.z - 128.0/255.0);

    let r = y + 1.79274107 * cr;
    let g = y - 0.21324861 * cb - 0.53290933 * cr;
    let b = y + 2.11240179 * cb;
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    var Y = textureSample(texY, samp, in.uv).r;
    var U = textureSample(texU, samp, in.uv).r;
    var V = textureSample(texV, samp, in.uv).r;

    let rgb = yuv_to_rgb709(vec3<f32>(Y, U, V));
    return vec4<f32>(rgb, 1.0);
}

