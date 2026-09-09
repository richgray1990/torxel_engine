// WGSL шейдер для твердых блоков с обводкой
// Vertex и Fragment shader в одном файле

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) neighbor_mask: u32, // 4 бита: 0=N, 1=S, 2=E, 3=W
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) neighbor_mask: u32,
};

struct Material {
    outline_color: vec4<f32>,
    outline_thickness: f32,
    _padding: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera_view_proj: mat4x4<f32>;

@group(1) @binding(0)
var diffuse_texture: texture_2d<f32>;

@group(1) @binding(1)
var diffuse_sampler: sampler;

@group(2) @binding(0)
var<uniform> material_props: Material;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera_view_proj * vec4<f32>(input.position, 1.0);
    output.uv = input.uv;
    output.neighbor_mask = input.neighbor_mask;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(diffuse_texture, diffuse_sampler, input.uv);
    
    var edge_mask: f32 = 0.0;
    let thickness = material_props.outline_thickness;
    
    // Биты neighbor_mask: 0=North(+Z), 1=South(-Z), 2=East(+X), 3=West(-X)
    // UV: x=0..1 (West->East), y=0..1 (South->North)
    
    if ((input.neighbor_mask & 1u) != 0u) {
        // North сосед - пустота, обводка сверху (UV.y ~ 1.0)
        edge_mask = max(edge_mask, step(1.0 - thickness, input.uv.y));
    }
    if ((input.neighbor_mask & 2u) != 0u) {
        // South сосед - пустота, обводка снизу (UV.y ~ 0.0)
        edge_mask = max(edge_mask, step(input.uv.y, thickness));
    }
    if ((input.neighbor_mask & 4u) != 0u) {
        // East сосед - пустота, обводка справа (UV.x ~ 1.0)
        edge_mask = max(edge_mask, step(1.0 - thickness, input.uv.x));
    }
    if ((input.neighbor_mask & 8u) != 0u) {
        // West сосед - пустота, обводка слева (UV.x ~ 0.0)
        edge_mask = max(edge_mask, step(input.uv.x, thickness));
    }
    
    // Смешиваем базовый цвет с черной обводкой
    let final_color = mix(base_color.rgb, material_props.outline_color.rgb, edge_mask * material_props.outline_color.a);
    
    return vec4<f32>(final_color, base_color.a);
}
