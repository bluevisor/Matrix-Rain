// Final composite: scene + bloom + DOF with tone mapping

struct CompositeUniforms {
    bloom_intensity: f32,
    dof_strength: f32,
    focal_depth: f32,
    focal_range: f32,
};

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var bloom_texture: texture_2d<f32>;
@group(0) @binding(2) var dof_texture: texture_2d<f32>;    // pre-blurred scene
@group(0) @binding(3) var depth_texture: texture_2d<f32>;
@group(0) @binding(4) var tex_sampler: sampler;
@group(0) @binding(5) var<uniform> params: CompositeUniforms;

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
    let scene = textureSample(scene_texture, tex_sampler, in.uv);
    let bloom = textureSample(bloom_texture, tex_sampler, in.uv);
    let blurred = textureSample(dof_texture, tex_sampler, in.uv);
    let depth = textureSample(depth_texture, tex_sampler, in.uv).r;

    // Circle of confusion based on depth distance from focal plane
    let diff = abs(depth - params.focal_depth);
    let coc = smoothstep(0.0, params.focal_range, diff) * params.dof_strength;

    // Lerp between sharp scene and pre-blurred DOF texture
    let dof_result = mix(scene, blurred, coc);

    // Add bloom
    let hdr_color = dof_result.rgb + bloom.rgb * params.bloom_intensity;

    // Reinhard tone mapping
    let mapped = hdr_color / (hdr_color + vec3<f32>(1.0));

    // Slight vignette
    let center = in.uv - vec2<f32>(0.5);
    let vignette = 1.0 - dot(center, center) * 0.5;

    return vec4<f32>(mapped * vignette, 1.0);
}
