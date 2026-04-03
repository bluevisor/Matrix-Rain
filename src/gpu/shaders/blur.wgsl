// Gaussian blur - configurable horizontal/vertical via uniforms

struct BlurUniforms {
    direction: vec2<f32>,   // (1,0) for horizontal, (0,1) for vertical
    texel_size: vec2<f32>,  // 1.0 / texture_dimensions
};

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> blur_uniforms: BlurUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );

    var out: VertexOutput;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 9-tap Gaussian kernel
    let weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

    let offset = blur_uniforms.direction * blur_uniforms.texel_size;
    var result = textureSample(input_texture, input_sampler, in.uv) * weights[0];

    for (var i = 1; i < 5; i++) {
        let off = offset * f32(i);
        result += textureSample(input_texture, input_sampler, in.uv + off) * weights[i];
        result += textureSample(input_texture, input_sampler, in.uv - off) * weights[i];
    }

    return result;
}
