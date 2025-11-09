struct ConverterParams {
    format: u32,
    width: u32,
    height: u32,
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> params: ConverterParams;

// Простая функция конвертации YUV420P в RGB (исправленная)
fn bt709_yuv_to_rgb(y_val: f32, u_val: f32, v_val: f32) -> vec3<f32> {
    let y_normalized = y_val - 16.0/255.0;
    let u_normalized = u_val - 128.0/255.0;
    let v_normalized = v_val - 128.0/255.0;
    
    let r = 1.164 * y_normalized + 1.793 * v_normalized;
    let g = 1.164 * y_normalized - 0.213 * u_normalized - 0.533 * v_normalized;
    let b = 1.164 * y_normalized + 2.112 * u_normalized;
    return vec3<f32>(r, g, b);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coord = vec2<i32>(global_id.xy);
    
    // Проверяем границы
    if coord.x >= i32(params.width) || coord.y >= i32(params.height) {
        return;
    }
    
    var color: vec4<f32>;
    
    // Пока работаем только с YUV420P
    if params.format == 0 {
        // YUV420P: Y-plane наверху, U и V planes ниже
        let y_coord = coord;
        
        // Читаем Y компоненту
        let y_value = textureLoad(input_texture, y_coord, 0).r;
        
        // Для теста: просто делаем grayscale из Y компоненты
        color = vec4<f32>(y_value, y_value, y_value, 1.0);
        
        // TODO: позже добавим U и V плоскости
        // let uv_coord = coord / 2;
        // let u_value = textureLoad(input_texture, 
        //     vec2<i32>(uv_coord.x, uv_coord.y + i32(params.height)), 0).r;
        // let v_value = textureLoad(input_texture, 
        //     vec2<i32>(uv_coord.x, uv_coord.y + i32(params.height * 3/2)), 0).r;
        // color = vec4<f32>(bt709_yuv_to_rgb(y_value, u_value, v_value), 1.0);
    } else {
        // Для других форматов - magenta (ошибка)
        color = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    
    textureStore(output_texture, coord, color);
}
