use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurUniforms {
    direction: [f32; 2],
    texel_size: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CompositeUniforms {
    pub bloom_intensity: f32,
    pub dof_strength: f32,
    pub focal_depth: f32,
    pub focal_range: f32,
}

pub struct PostProcess {
    // Bright extract
    bright_pipeline: wgpu::RenderPipeline,
    bright_view: wgpu::TextureView,
    bright_bind_group: wgpu::BindGroup,

    // Bloom blur (ping-pong at half res)
    blur_pipeline: wgpu::RenderPipeline,
    bloom_blur_views: [wgpu::TextureView; 2],
    bloom_blur_bind_groups: Vec<wgpu::BindGroup>,

    // DOF blur (ping-pong at full res) — 3 iterations = very wide blur
    dof_blur_views: [wgpu::TextureView; 2],
    dof_blur_bind_groups: Vec<wgpu::BindGroup>,

    // Composite
    composite_pipeline: wgpu::RenderPipeline,
    composite_bind_group: wgpu::BindGroup,
    composite_uniform_buffer: wgpu::Buffer,
}

fn create_fullscreen_pipeline(
    device: &wgpu::Device,
    label: &str,
    shader_source: &str,
    bind_group_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        })),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn create_rt(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat, label: &str) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    (tex, view)
}

fn create_blur_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    src_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform_buffer: &wgpu::Buffer,
    label: &str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
        ],
    })
}

impl PostProcess {
    pub fn new(
        device: &wgpu::Device,
        scene_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let hdr_format = wgpu::TextureFormat::Rgba16Float;
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let half_w = (width / 2).max(1);
        let half_h = (height / 2).max(1);

        // --- Bright extract ---
        let (_bright_texture, bright_view) = create_rt(device, half_w, half_h, hdr_format, "bright");

        let bright_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bright_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bright_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bright_bg"),
            layout: &bright_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(scene_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        let bright_pipeline = create_fullscreen_pipeline(
            device, "bright_extract",
            include_str!("shaders/bright_extract.wgsl"),
            &bright_bgl, hdr_format,
        );

        // --- Shared blur pipeline & layout ---
        let blur_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blur_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let blur_pipeline = create_fullscreen_pipeline(
            device, "blur",
            include_str!("shaders/blur.wgsl"),
            &blur_bgl, hdr_format,
        );

        // --- Bloom blur (half res, 2 iterations = 4 passes) ---
        let (_bloom_tex_a, bloom_view_a) = create_rt(device, half_w, half_h, hdr_format, "bloom_a");
        let (_bloom_tex_b, bloom_view_b) = create_rt(device, half_w, half_h, hdr_format, "bloom_b");

        let half_texel = [1.0 / half_w as f32, 1.0 / half_h as f32];
        let bloom_passes: [(& wgpu::TextureView, [f32; 2]); 4] = [
            (&bright_view, [1.0, 0.0]),
            (&bloom_view_a, [0.0, 1.0]),
            (&bloom_view_b, [1.0, 0.0]),
            (&bloom_view_a, [0.0, 1.0]),
        ];

        let mut bloom_blur_bind_groups = Vec::new();
        for (i, (src, dir)) in bloom_passes.iter().enumerate() {
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("bloom_blur_u_{i}")),
                contents: bytemuck::bytes_of(&BlurUniforms { direction: *dir, texel_size: half_texel }),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            bloom_blur_bind_groups.push(create_blur_bind_group(device, &blur_bgl, src, &sampler, &buf, &format!("bloom_bg_{i}")));
        }

        // --- DOF blur (full res, 3 iterations = 6 passes for very wide blur) ---
        let (_dof_tex_a, dof_view_a) = create_rt(device, width, height, hdr_format, "dof_a");
        let (_dof_tex_b, dof_view_b) = create_rt(device, width, height, hdr_format, "dof_b");

        let full_texel = [1.0 / width as f32, 1.0 / height as f32];
        // scene→a(H), a→b(V) x5 = 5 full Gaussian passes (~90px effective blur radius)
        let dof_passes: [(&wgpu::TextureView, [f32; 2]); 10] = [
            (scene_view,  [1.0, 0.0]),
            (&dof_view_a, [0.0, 1.0]),
            (&dof_view_b, [1.0, 0.0]),
            (&dof_view_a, [0.0, 1.0]),
            (&dof_view_b, [1.0, 0.0]),
            (&dof_view_a, [0.0, 1.0]),
            (&dof_view_b, [1.0, 0.0]),
            (&dof_view_a, [0.0, 1.0]),
            (&dof_view_b, [1.0, 0.0]),
            (&dof_view_a, [0.0, 1.0]),
        ];

        let mut dof_blur_bind_groups = Vec::new();
        for (i, (src, dir)) in dof_passes.iter().enumerate() {
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("dof_blur_u_{i}")),
                contents: bytemuck::bytes_of(&BlurUniforms { direction: *dir, texel_size: full_texel }),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            dof_blur_bind_groups.push(create_blur_bind_group(device, &blur_bgl, src, &sampler, &buf, &format!("dof_bg_{i}")));
        }

        // --- Composite (now takes scene + bloom + dof_blurred + depth) ---
        let composite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("composite_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let composite_uniforms = CompositeUniforms {
            bloom_intensity: 0.8,
            dof_strength: 1.0,
            focal_depth: 0.3,
            focal_range: 0.1,
        };

        let composite_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("composite_uniforms"),
            contents: bytemuck::bytes_of(&composite_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite_bg"),
            layout: &composite_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(scene_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&bloom_view_b) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&dof_view_b) }, // pre-blurred scene
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(depth_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: composite_uniform_buffer.as_entire_binding() },
            ],
        });

        let composite_pipeline = create_fullscreen_pipeline(
            device, "composite",
            include_str!("shaders/composite.wgsl"),
            &composite_bgl, surface_format,
        );

        PostProcess {
            bright_pipeline,
            bright_view,
            bright_bind_group,
            blur_pipeline,
            bloom_blur_views: [bloom_view_a, bloom_view_b],
            bloom_blur_bind_groups,
            dof_blur_views: [dof_view_a, dof_view_b],
            dof_blur_bind_groups,
            composite_pipeline,
            composite_bind_group,
            composite_uniform_buffer,
        }
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &CompositeUniforms) {
        queue.write_buffer(&self.composite_uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        // Pass 1: Bright extract
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bright_extract_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bright_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.bright_pipeline);
            pass.set_bind_group(0, &self.bright_bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        // Pass 2: Bloom blur (4 passes at half res)
        let bloom_targets = [&self.bloom_blur_views[0], &self.bloom_blur_views[1], &self.bloom_blur_views[0], &self.bloom_blur_views[1]];
        for (i, bg) in self.bloom_blur_bind_groups.iter().enumerate() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("bloom_blur_{i}")),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: bloom_targets[i],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, bg, &[]);
            pass.draw(0..6, 0..1);
        }

        // Pass 3: DOF blur (10 passes at full res — very heavy blur)
        let dof_targets = [
            &self.dof_blur_views[0], &self.dof_blur_views[1],
            &self.dof_blur_views[0], &self.dof_blur_views[1],
            &self.dof_blur_views[0], &self.dof_blur_views[1],
            &self.dof_blur_views[0], &self.dof_blur_views[1],
            &self.dof_blur_views[0], &self.dof_blur_views[1],
        ];
        for (i, bg) in self.dof_blur_bind_groups.iter().enumerate() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("dof_blur_{i}")),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dof_targets[i],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, bg, &[]);
            pass.draw(0..6, 0..1);
        }

        // Pass 4: Composite to screen
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &self.composite_bind_group, &[]);
            pass.draw(0..6, 0..1);
        }
    }
}
