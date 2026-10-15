//! Video scopes rendering on GPU

use wgpu::{Device, Queue, Texture, TextureView, RenderPipeline, BindGroup, BindGroupLayout, TextureFormat};
use crate::gpu::buffer::{UniformBuffer, ScopesUniforms};
use crate::gpu::texture::{TextureManager, GpuTexture};
use crate::core::time::Frame;
use std::sync::Arc;

pub struct ScopesRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipelines: std::collections::HashMap<ScopeMode, RenderPipeline>,
    bind_group_layout: Arc<BindGroupLayout>,
    texture_manager: TextureManager,
    waveform_texture: Option<GpuTexture>,
    parade_texture: Option<GpuTexture>,
    vectorscope_texture: Option<GpuTexture>,
    histogram_texture: Option<GpuTexture>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeMode {
    Waveform,
    Parade,
    Vectorscope,
    Histogram,
}

impl ScopesRenderer {
    /// `queue` is passed in explicitly: `wgpu::Device` has no `queue()` accessor.
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let texture_manager = TextureManager::new(device.clone(), queue);
        
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scopes Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Scopes Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Scopes Shader"),
            source: wgpu::ShaderSource::Wgsl(SCOPES_SHADER.into()),
        });
        
        let mut pipelines = std::collections::HashMap::new();
        for mode in [ScopeMode::Waveform, ScopeMode::Parade, ScopeMode::Vectorscope, ScopeMode::Histogram] {
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&format!("{:?} Pipeline", mode)),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
                fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some(mode.fs_entry()), targets: &[Some(wgpu::ColorTargetState { format: TextureFormat::Rgba8Unorm, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
            pipelines.insert(mode, pipeline);
        }
        
        Self {
            device: device.clone(),
            queue,
            pipelines,
            bind_group_layout: Arc::new(bind_group_layout),
            texture_manager,
            waveform_texture: None,
            parade_texture: None,
            vectorscope_texture: None,
            histogram_texture: None,
        }
    }

    pub fn render_waveform(&mut self, input: &GpuTexture, output: &GpuTexture, gain: f32) -> Result<(), Box<dyn std::error::Error>> {
        self.render_scope(ScopeMode::Waveform, input, output, ScopesUniforms { resolution: [256.0, 256.0], mode: 0, color_space: 0, gain, _padding: [0.0; 3] })
    }

    pub fn render_parade(&mut self, input: &GpuTexture, output: &GpuTexture, gain: f32) -> Result<(), Box<dyn std::error::Error>> {
        self.render_scope(ScopeMode::Parade, input, output, ScopesUniforms { resolution: [256.0, 256.0], mode: 1, color_space: 0, gain, _padding: [0.0; 3] })
    }

    pub fn render_vectorscope(&mut self, input: &GpuTexture, output: &GpuTexture, gain: f32) -> Result<(), Box<dyn std::error::Error>> {
        self.render_scope(ScopeMode::Vectorscope, input, output, ScopesUniforms { resolution: [256.0, 256.0], mode: 2, color_space: 0, gain, _padding: [0.0; 3] })
    }

    pub fn render_histogram(&mut self, input: &GpuTexture, output: &GpuTexture, gain: f32) -> Result<(), Box<dyn std::error::Error>> {
        self.render_scope(ScopeMode::Histogram, input, output, ScopesUniforms { resolution: [256.0, 256.0], mode: 3, color_space: 0, gain, _padding: [0.0; 3] })
    }

    fn render_scope(&mut self, mode: ScopeMode, input: &GpuTexture, output: &GpuTexture, uniforms: ScopesUniforms) -> Result<(), Box<dyn std::error::Error>> {
        let device = &self.device;
        // The queue cannot be recovered from the device, so it is borrowed from the
        // manager we were constructed with.
        let queue = self.queue.as_ref();
        
        let uniform_buffer = UniformBuffer::new_uniform(device, &uniforms, Some("Scopes Uniforms"));
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scopes Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&input.view) },
                wgpu::BindGroupEntry { binding: 1, resource: uniform_buffer.as_entire_binding() },
            ],
        });
        
        let pipeline = self.pipelines.get(&mode).unwrap();
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Scopes Encoder") });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("{:?} Pass", mode)),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &output.view, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }
        
        queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub fn read_waveform_data(&self, input: &GpuTexture) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        // Would use compute shader to read back data
        Ok(vec![0.0; 256 * 3])
    }
}

impl ScopeMode {
    fn fs_entry(&self) -> &'static str {
        match self {
            ScopeMode::Waveform => "fs_waveform",
            ScopeMode::Parade => "fs_parade",
            ScopeMode::Vectorscope => "fs_vectorscope",
            ScopeMode::Histogram => "fs_histogram",
        }
    }
}

const SCOPES_SHADER: &str = r#"
struct ScopesUniforms {
    resolution: vec2<f32>,
    mode: u32,
    color_space: u32,
    gain: f32,
}

@group(0) @binding(0) var inputTexture: texture_2d<f32>;
@group(0) @binding(1) var<uniform> scopes: ScopesUniforms;

@vertex fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> @builtin(position) vec4<f32> {
    var pos = array(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(pos[vertexIndex], 0.0, 1.0);
}

@fragment fn fs_waveform(@builtin(position) fragCoord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = fragCoord.xy / scopes.resolution;
    let x = u32(uv.x * scopes.resolution.x);
    let color = textureLoad(inputTexture, vec2<u32>(x, 0), 0);
    let y = color.r * scopes.gain;
    let intensity = step(uv.y, 1.0 - y) * vec3<f32>(1.0, 0.0, 0.0) + 
                    step(uv.y, 1.0 - color.g * scopes.gain) * vec3<f32>(0.0, 1.0, 0.0) + 
                    step(uv.y, 1.0 - color.b * scopes.gain) * vec3<f32>(0.0, 0.0, 1.0);
    return vec4<f32>(intensity, 1.0);
}

@fragment fn fs_parade(@builtin(position) fragCoord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = fragCoord.xy / scopes.resolution;
    let channel = u32(uv.x * 3.0);
    let x = u32(fract(uv.x * 3.0) * scopes.resolution.x / 3.0);
    let color = textureLoad(inputTexture, vec2<u32>(x, 0), 0);
    let y = color[channel] * scopes.gain;
    let intensity = step(uv.y, 1.0 - y);
    var outColor = vec3<f32>(0.0);
    if (channel == 0u) { outColor = vec3<f32>(intensity, 0.0, 0.0); }
    else if (channel == 1u) { outColor = vec3<f32>(0.0, intensity, 0.0); }
    else { outColor = vec3<f32>(0.0, 0.0, intensity); }
    return vec4<f32>(outColor, 1.0);
}

@fragment fn fs_vectorscope(@builtin(position) fragCoord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = fragCoord.xy / scopes.resolution * 2.0 - 1.0;
    let dist = length(uv);
    if (dist > 1.0) { return vec4<f32>(0.0); }
    // Simplified - would need compute shader for proper vectorscope
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

@fragment fn fs_histogram(@builtin(position) fragCoord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = fragCoord.xy / scopes.resolution;
    let x = u32(uv.x * 256.0);
    let color = textureLoad(inputTexture, vec2<u32>(x, 0), 0);
    let y = color.r * scopes.gain;
    return vec4<f32>(vec3<f32>(step(uv.y, 1.0 - y)), 1.0);
}
"#;