//! Color grading pipeline on GPU

use crate::core::color::PixelFormat;
use crate::core::color::{Cdl, ColorSpace, Lut1D, Lut3D};
use crate::gpu::buffer::{ColorGradeUniforms, UniformBuffer};
use crate::gpu::pipeline::{PipelineKey, PipelineManager};
use crate::gpu::texture::{GpuTexture, TextureManager};
use std::sync::Arc;
use wgpu::{
    BindGroup, BindGroupLayout, Device, PipelineLayout, Queue, RenderPipeline, Texture,
    TextureFormat, TextureView,
};

/// Color grading pipeline
pub struct ColorPipeline {
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: RenderPipeline,
    bind_group_layout: Arc<BindGroupLayout>,
    lut3d_bind_group_layout: Arc<BindGroupLayout>,
    texture_manager: TextureManager,
    current_grade: ColorGradeUniforms,
    lut3d_texture: Option<GpuTexture>,
    lut1d_texture: Option<GpuTexture>,
}

impl ColorPipeline {
    /// `queue` must be passed in explicitly: `wgpu::Device` has no `queue()`
    /// accessor, so the queue cannot be recovered from the device.
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let texture_manager = TextureManager::new(device.clone(), queue.clone());

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Color Grade Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D3,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D1,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
        });

        let lut3d_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("LUT 3D Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D3,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Color Grade Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout, &lut3d_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Color Grade Shader"),
            source: wgpu::ShaderSource::Wgsl(COLOR_GRADE_SHADER.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Color Grade Pipeline"),
            layout: Some(&pipeline_layout),
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
                    format: TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            device: device.clone(),
            queue,
            pipeline,
            bind_group_layout: Arc::new(bind_group_layout),
            lut3d_bind_group_layout: Arc::new(lut3d_bind_group_layout),
            texture_manager,
            current_grade: ColorGradeUniforms::default(),
            lut3d_texture: None,
            lut1d_texture: None,
        }
    }

    pub fn apply_grade(
        &mut self,
        device: &Device,
        queue: &Queue,
        input: GpuTexture,
        output: GpuTexture,
        grade: ColorGradeUniforms,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.current_grade = grade;

        let uniform_buffer =
            UniformBuffer::new_uniform(device, &grade, Some("Color Grade Uniforms"));

        // Materialise the identity LUTs once. They are cached on the pipeline, so
        // this only allocates on the first grade that has no LUT applied yet.
        if self.lut3d_texture.is_none() {
            self.lut3d_texture = Some(self.create_identity_lut3d());
        }
        if self.lut1d_texture.is_none() {
            self.lut1d_texture = Some(self.create_identity_lut1d());
        }

        let lut3d_view = &self
            .lut3d_texture
            .as_ref()
            .expect("lut3d initialised above")
            .view;
        let lut1d_view = &self
            .lut1d_texture
            .as_ref()
            .expect("lut1d initialised above")
            .view;

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Color Grade Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&input.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(lut3d_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(lut1d_view),
                },
            ],
        });

        let lut3d_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LUT 3D Bind Group"),
            layout: &self.lut3d_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(lut3d_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(
                        &self.texture_manager.get_sampler(Default::default()),
                    ),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Color Grade Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Color Grade Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_bind_group(1, &lut3d_bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }

        queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub fn set_lut3d(&mut self, lut: &Lut3D) -> Result<(), Box<dyn std::error::Error>> {
        let size = lut.size as u32;
        let data: Vec<u8> = lut
            .data
            .iter()
            .flat_map(|c| {
                [
                    (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                    255,
                ]
            })
            .collect();
        self.lut3d_texture = Some(
            self.texture_manager
                .create_3d_texture(size, size, size, data),
        );
        Ok(())
    }

    pub fn set_lut1d(&mut self, lut: &Lut1D) -> Result<(), Box<dyn std::error::Error>> {
        let size = lut.size as u32;
        let data: Vec<u8> = lut
            .data
            .iter()
            .flat_map(|c| {
                [
                    (c[0].clamp(0.0, 1.0) * 65535.0) as u16,
                    (c[1].clamp(0.0, 1.0) * 65535.0) as u16,
                    (c[2].clamp(0.0, 1.0) * 65535.0) as u16,
                ]
            })
            .flat_map(|v| v.to_le_bytes())
            .collect();
        self.lut1d_texture = Some(self.texture_manager.create_1d_texture(size, data));
        Ok(())
    }

    pub fn apply_cdl(&mut self, cdl: &Cdl) {
        self.current_grade.lift = [cdl.offset[0], cdl.offset[1], cdl.offset[2], 0.0];
        self.current_grade.gain = [cdl.slope[0], cdl.slope[1], cdl.slope[2], 0.0];
        self.current_grade.gamma = [cdl.power[0], cdl.power[1], cdl.power[2], 0.0];
        self.current_grade.saturation = cdl.sat;
    }

    fn create_identity_lut3d(&mut self) -> GpuTexture {
        let size = 32u32;
        let mut data = Vec::with_capacity((size * size * size * 4) as usize);
        for z in 0..size {
            for y in 0..size {
                for x in 0..size {
                    let r = x as f32 / (size - 1) as f32;
                    let g = y as f32 / (size - 1) as f32;
                    let b = z as f32 / (size - 1) as f32;
                    data.push((r * 255.0) as u8);
                    data.push((g * 255.0) as u8);
                    data.push((b * 255.0) as u8);
                    data.push(255);
                }
            }
        }
        self.texture_manager
            .create_3d_texture(size, size, size, data)
    }

    fn create_identity_lut1d(&mut self) -> GpuTexture {
        let size = 256u32;
        let mut data = Vec::with_capacity((size * 3 * 2) as usize);
        for i in 0..size {
            let v = i as f32 / (size - 1) as f32;
            let val = (v * 65535.0) as u16;
            data.extend_from_slice(&val.to_le_bytes());
            data.extend_from_slice(&val.to_le_bytes());
            data.extend_from_slice(&val.to_le_bytes());
        }
        self.texture_manager.create_1d_texture(size, data)
    }
}

const COLOR_GRADE_SHADER: &str = r#"
struct ColorGradeUniforms {
    lift: vec4<f32>, gamma: vec4<f32>, gain: vec4<f32>, offset: vec4<f32>,
    contrast: f32, pivot: f32, saturation: f32, temperature: f32, tint: f32,
    shadows: vec4<f32>, midtones: vec4<f32>, highlights: vec4<f32>,
}

@group(0) @binding(0) var inputTexture: texture_2d<f32>;
@group(0) @binding(1) var<uniform> grade: ColorGradeUniforms;
@group(0) @binding(2) var lut3d: texture_3d<f32>;
@group(0) @binding(3) var lut1d: texture_1d<f32>;

@vertex fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> @builtin(position) vec4<f32> {
    var pos = array(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(pos[vertexIndex], 0.0, 1.0);
}

@fragment fn fs_main(@builtin(position) fragCoord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = fragCoord.xy / vec2<f32>(textureDimensions(inputTexture));
    var color = textureLoad(inputTexture, vec2<u32>(uv * vec2<f32>(textureDimensions(inputTexture))), 0);
    
    // Lift/Gamma/Gain
    color.rgb = (color.rgb + grade.lift.rgb) * grade.gain.rgb;
    color.rgb = pow(color.rgb, vec3<f32>(1.0 / grade.gamma.rgb));
    
    // Offset
    color.rgb += grade.offset.rgb;
    
    // Contrast
    color.rgb = (color.rgb - grade.pivot) * grade.contrast + grade.pivot;
    
    // Saturation
    let lum = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    color.rgb = mix(vec3<f32>(lum), color.rgb, grade.saturation);
    
    // Temperature/Tint
    color.rgb *= vec3<f32>(1.0 + grade.temperature, 1.0, 1.0 - grade.temperature);
    color.rgb *= vec3<f32>(1.0 - grade.tint, 1.0 + grade.tint, 1.0 - grade.tint);
    
    // 3D LUT
    color.rgb = textureSampleLevel(lut3d, sampler, color.rgb, 0.0).rgb;
    
    // 1D LUT
    color.r = textureSampleLevel(lut1d, sampler, color.r, 0.0).r;
    color.g = textureSampleLevel(lut1d, sampler, color.g, 0.0).g;
    color.b = textureSampleLevel(lut1d, sampler, color.b, 0.0).b;
    
    return color;
}
"#;
