// @group(0) layout:
// 0: sampler
// 1: texY (R8Unorm, WxH)
// 2: texU (R8Unorm, W/2 x H/2)   // для 4:2:0
// 3: texV (R8Unorm, W/2 x H/2)

@group(0) @binding(0) var samp : sampler;
@group(0) @binding(1) var texY : texture_2d<f32>;
@group(0) @binding(2) var texU : texture_2d<f32>;
@group(0) @binding(3) var texV : texture_2d<f32>;

struct VSOut {
    @builtin(position) pos : vec4<f32>,
    @location(0)       uv  : vec2<f32>,
};

@vertex
fn vs_main(@location(0) in_pos: vec2<f32>,
           @location(1) in_uv:  vec2<f32>) -> VSOut {
    var out: VSOut;
    out.pos = vec4<f32>(in_pos, 0.0, 1.0);
    out.uv  = in_uv;
    return out;
}

// BT.709 limited-range (TV range) матрица.
// Y' [16..235], Cb/Cr [16..240].
// Преобразуем к "display-готовому" RGB' (под sRGB backbuffer это ок на старте).
fn yuv420p_to_rgb709(yuv: vec3<f32>) -> vec3<f32> {
    // смещения и коэффициенты по Rec.709 (TV range)
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
    let Y  = textureSample(texY, samp, in.uv).r;        // 0..1
    let U  = textureSample(texU, samp, in.uv).r;        // полурез, сэмплер сам рескейлит
    let V  = textureSample(texV, samp, in.uv).r;
    let rgb = yuv420p_to_rgb709(vec3<f32>(Y, U, V));
    return vec4<f32>(rgb, 1.0);
}

