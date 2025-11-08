struct ConverterParams {
    format: u32,
    width: u32,
    height: u32,
    stride_y: u32,
    stride_u: u32,
    stride_v: u32,
    bit_depth: u32,
    color_matrix: u32,
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> params: ConverterParams;

// Функции конвертации цветовых пространств
fn bt709_yuv_to_rgb(y: f32, u: f32, v: f32) -> vec3<f32> {
    let y = y - 16.0/255.0;
    let u = u - 128.0/255.0;
    let v = v - 128.0/255.0;
    
    let r = 1.164 * y + 1.793 * v;
    let g = 1.164 * y - 0.213 * u - 0.533 * v;
    let b = 1.164 * y + 2.112 * u;
    return vec3<f32>(r, g, b);
}

fn bt601_yuv_to_rgb(y: f32, u: f32, v: f32) -> vec3<f32> {
    let r = y + 1.402 * (v - 0.5);
    let g = y - 0.344 * (u - 0.5) - 0.714 * (v - 0.5);
    let b = y + 1.772 * (u - 0.5);
    return vec3<f32>(r, g, b);
}

fn read_10bit_texture(tex: texture_2d<f32>, coord: vec2<i32>) -> f32 {
    let raw = textureLoad(tex, coord, 0).r;
    return raw * 1023.0 / 65535.0; // Конвертация 16bit UNORM -> 10bit
}

fn read_12bit_texture(tex: texture_2d<f32>, coord: vec2<i32>) -> f32 {
    let raw = textureLoad(tex, coord, 0).r;
    return raw * 4095.0 / 65535.0; // Конвертация 16bit UNORM -> 12bit
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coord = vec2<i32>(global_id.xy);
    
    if coord.x >= i32(params.width) || coord.y >= i32(params.height) {
        return;
    }
    
    var color: vec4<f32>;
    
    switch params.format {
        case 0: { // YUV420P
            let y_coord = coord;
            let uv_coord = coord / 2;
            
            let y = textureLoad(input_texture, y_coord, 0).r;
            let u = textureLoad(input_texture, 
                vec2<i32>(uv_coord.x, uv_coord.y + i32(params.height)), 0).r;
            let v = textureLoad(input_texture, 
                vec2<i32>(uv_coord.x, uv_coord.y + i32(params.height * 3/2)), 0).r;
            
            color = vec4<f32>(bt709_yuv_to_rgb(y, u, v), 1.0);
        }
        case 1: { // YUV422P
            let y_coord = coord;
            let uv_coord = vec2<i32>(coord.x / 2, coord.y);
            
            let y = textureLoad(input_texture, y_coord, 0).r;
            let u = textureLoad(input_texture, 
                vec2<i32>(uv_coord.x, uv_coord.y + i32(params.height)), 0).r;
            let v = textureLoad(input_texture, 
                vec2<i32>(uv_coord.x, uv_coord.y + i32(params.height * 2)), 0).r;
            
            color = vec4<f32>(bt709_yuv_to_rgb(y, u, v), 1.0);
        }
        case 2: { // NV12
            let y_coord = coord;
            let uv_coord = coord / 2;
            
            let y = textureLoad(input_texture, y_coord, 0).r;
            let uv = textureLoad(input_texture, 
                vec2<i32>(uv_coord.x, uv_coord.y + i32(params.height)), 0).rg;
            
            color = vec4<f32>(bt709_yuv_to_rgb(y, uv.x, uv.y), 1.0);
        }
        case 3: { // RGB
            let rgb = textureLoad(input_texture, coord, 0).rgb;
            color = vec4<f32>(rgb, 1.0);
        }
        case 4: { // YUV422P10LE (обработка битности)
            let y_coord = coord;
            let uv_coord = vec2<i32>(coord.x / 2, coord.y);
            
            let y = read_10bit_texture(input_texture, y_coord);
            let u = read_10bit_texture(input_texture, 
                vec2<i32>(uv_coord.x, uv_coord.y + i32(params.height)));
            let v = read_10bit_texture(input_texture, 
                vec2<i32>(uv_coord.x, uv_coord.y + i32(params.height * 2)));
            
            // Нормализация 10bit -> 0-1
            let y_norm = y / 1023.0;
            let u_norm = u / 1023.0;
            let v_norm = v / 1023.0;
            
            color = vec4<f32>(bt709_yuv_to_rgb(y_norm, u_norm, v_norm), 1.0);
        }
        default: {
            color = vec4<f32>(1.0, 0.0, 1.0, 1.0); // magenta для ошибки
        }
    }
    
    textureStore(output_texture, coord, color);
}
