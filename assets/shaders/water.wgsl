// WGSL шейдер для воды (прозрачный, без обводки или с простой обводкой)
// Vertex и Fragment shader в одном файле

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) depth: f32, // Для сортировки прозрачности
};

@group(0) @binding(0)
var<uniform> camera_view_proj: mat4x4<f32>;

@group(1) @binding(0)
var diffuse_texture: texture_2d<f32>;

@group(1) @binding(1)
var diffuse_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera_view_proj * vec4<f32>(input.position, 1.0);
    output.uv = input.uv;
    output.depth = input.position.y; // Используем Y как глубину для сортировки
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(diffuse_texture, diffuse_sampler, input.uv);
    
    // Вода прозрачная, можно добавить легкую анимацию или искажение
    // Пока просто возвращаем цвет с альфа-каналом
    let alpha = 0.6; // Прозрачность воды
    
    return vec4<f32>(base_color.rgb * 0.8, alpha); // Немного затемняем воду
}
