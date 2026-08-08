struct ConverterParams {
    format: u32,  // 0=YUV420P, 1=YUV422P, 2=YUV444P
    width: u32,
    height: u32,
}

@group(0) @binding(0) var tex_y: texture_2d<f32>;
@group(0) @binding(1) var tex_u: texture_2d<f32>;
@group(0) @binding(2) var tex_v: texture_2d<f32>;
@group(0) @binding(3) var output_texture: texture_storage_2d<rgba16float, write>;
@group(0) @binding(4) var<uniform> params: ConverterParams;

// BT.709 limited-range YCbCr → RGB
fn yuv_to_rgb(y: f32, cb: f32, cr: f32) -> vec3<f32> {
    let y_norm  = (y  - 16.0 / 255.0) * 1.16438356;
    let u_norm  = cb - 128.0 / 255.0;
    let v_norm  = cr - 128.0 / 255.0;

    let r = y_norm + 1.79274107 * v_norm;
    let g = y_norm - 0.21324861 * u_norm - 0.53290933 * v_norm;
    let b = y_norm + 2.11240179 * u_norm;

    return vec3<f32>(r, g, b);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);

    if x >= i32(params.width) || y >= i32(params.height) {
        return;
    }

    let y_val = textureLoad(tex_y, vec2<i32>(x, y), 0).r;

    // Chroma subsampling: compute UV coordinate per format
    var uv_x: i32;
    var uv_y: i32;
    switch params.format {
        case 1u, 3u: {
            uv_x = x / 2;  // YUV422P / YUV422P10LE: half horizontal
            uv_y = y;
        }
        case 2u: {
            uv_x = x;     // YUV444P: full res
            uv_y = y;
        }
        default: {
            uv_x = x / 2;  // YUV420P: half both dims
            uv_y = y / 2;
        }
    }

    let u_val = textureLoad(tex_u, vec2<i32>(uv_x, uv_y), 0).r;
    let v_val = textureLoad(tex_v, vec2<i32>(uv_x, uv_y), 0).r;

    let rgb = yuv_to_rgb(y_val, u_val, v_val);
    textureStore(output_texture, vec2<i32>(x, y), vec4<f32>(rgb, 1.0));
}