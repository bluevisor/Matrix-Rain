// Glyph rendering shader - instanced quads with texture atlas sampling

struct Uniforms {
    view_proj: mat4x4<f32>,
    quad_size: vec2<f32>,
    time: f32,
    _padding: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct InstanceInput {
    @location(0) position: vec3<f32>,
    @location(1) uv_rect: vec4<f32>,    // u_min, v_min, u_max, v_max
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) depth: f32,
};

// Quad vertices: two triangles making a quad
// vertex_index 0-5 maps to the 6 vertices of 2 triangles
@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    // Quad corners: bottom-left, bottom-right, top-right, top-left
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), // tri 1
        vec2<f32>( 0.5, -0.5),
        vec2<f32>( 0.5,  0.5),
        vec2<f32>(-0.5, -0.5), // tri 2
        vec2<f32>( 0.5,  0.5),
        vec2<f32>(-0.5,  0.5),
    );

    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );

    let pos = positions[vertex_index];
    let uv_local = uvs[vertex_index];

    // Glitch: random horizontal band displacement
    let band = floor(instance.position.y * 6.0);
    let t_slot = floor(uniforms.time * 2.0);
    let h = fract(sin(dot(vec2<f32>(band, t_slot), vec2<f32>(127.1, 311.7))) * 43758.5);
    let glitch_active = step(0.92, h);
    let glitch_shift = (fract(h * 39.4) - 0.5) * 1.2 * glitch_active;

    let world_pos = vec3<f32>(
        instance.position.x + pos.x * uniforms.quad_size.x + glitch_shift,
        instance.position.y + pos.y * uniforms.quad_size.y,
        instance.position.z,
    );

    let clip_pos = uniforms.view_proj * vec4<f32>(world_pos, 1.0);

    // Map UV from local [0,1] to atlas rect
    let atlas_uv = vec2<f32>(
        mix(instance.uv_rect.x, instance.uv_rect.z, uv_local.x),
        mix(instance.uv_rect.y, instance.uv_rect.w, uv_local.y),
    );

    var out: VertexOutput;
    out.clip_position = clip_pos;
    out.uv = atlas_uv;
    out.color = instance.color;
    // Normalize depth to 0-1 for DOF
    out.depth = clamp((-world_pos.z - 0.1) / 100.0, 0.0, 1.0);
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) depth_out: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let tex_color = textureSample(atlas_texture, atlas_sampler, in.uv);
    let alpha = tex_color.a;

    if alpha < 0.01 {
        discard;
    }

    var out: FragmentOutput;
    out.color = vec4<f32>(in.color.rgb * alpha, alpha);
    out.depth_out = vec4<f32>(in.depth, in.depth, in.depth, 1.0);
    return out;
}
