//! Render pipeline management

use wgpu::{Device, RenderPipeline, PipelineLayout, BindGroupLayout, ShaderModule, ShaderStages};
use std::collections::HashMap;
use std::sync::Arc;

/// Pipeline manager - caches and creates pipelines
pub struct PipelineManager {
    device: Arc<Device>,
    pipelines: HashMap<PipelineKey, Arc<RenderPipeline>>,
    layouts: HashMap<LayoutKey, Arc<PipelineLayout>>,
    bind_group_layouts: HashMap<BindGroupKey, Arc<BindGroupLayout>>,
    shaders: HashMap<ShaderKey, Arc<ShaderModule>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PipelineKey {
    TextureQuad,
    TextureQuadBlend,
    ColorGrade,
    Transition(TransitionPipelineType),
    Composite,
    Text,
    Waveform,
    Vectorscope,
    Histogram,
    Custom(String),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum TransitionPipelineType {
    CrossDissolve,
    Wipe,
    Slide,
    Push,
    DipToColor,
    MorphCut,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum LayoutKey {
    TextureQuad,
    ColorGrade,
    Transition,
    Composite,
    Text,
    Scopes,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum BindGroupKey {
    Texture,
    ColorGrade,
    Transition,
    Scopes,
    Compute,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ShaderKey {
    TextureQuad,
    ColorGrade,
    Transition(TransitionPipelineType),
    Scopes,
    Compute(ComputeShaderType),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ComputeShaderType {
    Thumbnail,
    Waveform,
    Histogram,
    Vectorscope,
    OpticalFlow,
    NoiseReduction,
    Stabilize,
    Transcode,
}

impl PipelineManager {
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            pipelines: HashMap::new(),
            layouts: HashMap::new(),
            bind_group_layouts: HashMap::new(),
            shaders: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, key: PipelineKey, format: wgpu::TextureFormat) -> Arc<RenderPipeline> {
        if let Some(pipeline) = self.pipelines.get(&key) {
            return pipeline.clone();
        }
        
        let pipeline = Arc::new(self.create_pipeline(key.clone(), format));
        self.pipelines.insert(key, pipeline.clone());
        pipeline
    }

    fn create_pipeline(&self, key: PipelineKey, format: wgpu::TextureFormat) -> RenderPipeline {
        let layout = self.get_or_create_layout(key.layout_key());
        let shader = self.get_or_create_shader(key.shader_key());
        
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{:?} Layout", key)),
            bind_group_layouts: &[layout.as_ref()],
            push_constant_ranges: &key.push_constant_ranges(),
        });
        
        let (vertex_entry, vertex_buffers) = key.vertex_state();
        let (fragment_entry, targets) = key.fragment_state(format);
        
        self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{:?} Pipeline", key)),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader.as_ref(),
                entry_point: Some(vertex_entry),
                buffers: vertex_buffers,
                compilation_options: Default::default(),
            },
            primitive: key.primitive_state(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader.as_ref(),
                entry_point: Some(fragment_entry),
                targets,
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        })
    }

    fn get_or_create_layout(&mut self, key: LayoutKey) -> Arc<PipelineLayout> {
        if let Some(layout) = self.layouts.get(&key) {
            return layout.clone();
        }
        
        let layout = Arc::new(self.create_layout(key.clone()));
        self.layouts.insert(key, layout.clone());
        layout
    }

    fn create_layout(&self, key: LayoutKey) -> PipelineLayout {
        let bind_group_layout = self.get_or_create_bind_group_layout(key.bind_group_key());
        
        self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{:?} Layout", key)),
            bind_group_layouts: &[bind_group_layout.as_ref()],
            push_constant_ranges: &key.push_constant_ranges(),
        })
    }

    fn get_or_create_bind_group_layout(&mut self, key: BindGroupKey) -> Arc<BindGroupLayout> {
        if let Some(layout) = self.bind_group_layouts.get(&key) {
            return layout.clone();
        }
        
        let layout = Arc::new(self.create_bind_group_layout(key.clone()));
        self.bind_group_layouts.insert(key, layout.clone());
        layout
    }

    fn create_bind_group_layout(&self, key: BindGroupKey) -> BindGroupLayout {
        let entries = key.entries();
        
        self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{:?} Bind Group Layout", key)),
            entries: &entries,
        })
    }

    fn get_or_create_shader(&mut self, key: ShaderKey) -> Arc<ShaderModule> {
        if let Some(shader) = self.shaders.get(&key) {
            return shader.clone();
        }
        
        let shader = Arc::new(self.create_shader(key.clone()));
        self.shaders.insert(key, shader.clone());
        shader
    }

    fn create_shader(&self, key: ShaderKey) -> ShaderModule {
        let source = key.source();
        self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{:?} Shader", key)),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        })
    }
}

impl PipelineKey {
    fn layout_key(&self) -> LayoutKey {
        match self {
            PipelineKey::TextureQuad | PipelineKey::TextureQuadBlend => LayoutKey::TextureQuad,
            PipelineKey::ColorGrade => LayoutKey::ColorGrade,
            PipelineKey::Transition(_) => LayoutKey::Transition,
            PipelineKey::Composite => LayoutKey::Composite,
            PipelineKey::Text => LayoutKey::Text,
            PipelineKey::Waveform | PipelineKey::Vectorscope | PipelineKey::Histogram => LayoutKey::Scopes,
            PipelineKey::Custom(_) => LayoutKey::TextureQuad,
        }
    }

    fn shader_key(&self) -> ShaderKey {
        match self {
            PipelineKey::TextureQuad | PipelineKey::TextureQuadBlend => ShaderKey::TextureQuad,
            PipelineKey::ColorGrade => ShaderKey::ColorGrade,
            PipelineKey::Transition(t) => ShaderKey::Transition(t.clone()),
            PipelineKey::Composite => ShaderKey::TextureQuad,
            PipelineKey::Text => ShaderKey::TextureQuad,
            PipelineKey::Waveform => ShaderKey::Compute(ComputeShaderType::Waveform),
            PipelineKey::Vectorscope => ShaderKey::Compute(ComputeShaderType::Vectorscope),
            PipelineKey::Histogram => ShaderKey::Compute(ComputeShaderType::Histogram),
            PipelineKey::Custom(_) => ShaderKey::TextureQuad,
        }
    }

    fn push_constant_ranges(&self) -> &'static [wgpu::PushConstantRange] {
        match self {
            PipelineKey::Transition(_) => &[wgpu::PushConstantRange {
                stages: ShaderStages::FRAGMENT,
                range: 0..64,
            }],
            _ => &[],
        }
    }

    fn vertex_state(&self) -> (&'static str, &'static [wgpu::VertexBufferLayout]) {
        match self {
            PipelineKey::Text => ("vs_text", &[crate::gpu::text::TEXT_VERTEX_LAYOUT]),
            _ => ("vs_main", &[]),
        }
    }

    fn fragment_state(&self, format: wgpu::TextureFormat) -> (&'static str, &'static [Option<wgpu::ColorTargetState>]) {
        let target = Some(wgpu::ColorTargetState {
            format,
            blend: Some(match self {
                PipelineKey::TextureQuadBlend => wgpu::BlendState::ALPHA_BLENDING,
                _ => wgpu::BlendState::REPLACE,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        });
        ("fs_main", &[target])
    }

    fn primitive_state(&self) -> wgpu::PrimitiveState {
        match self {
            PipelineKey::Text => wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            _ => wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
        }
    }
}

impl LayoutKey {
    fn bind_group_key(&self) -> BindGroupKey {
        match self {
            LayoutKey::TextureQuad => BindGroupKey::Texture,
            LayoutKey::ColorGrade => BindGroupKey::ColorGrade,
            LayoutKey::Transition => BindGroupKey::Transition,
            LayoutKey::Composite => BindGroupKey::Texture,
            LayoutKey::Text => BindGroupKey::Texture,
            LayoutKey::Scopes => BindGroupKey::Scopes,
        }
    }

    fn push_constant_ranges(&self) -> &'static [wgpu::PushConstantRange] {
        match self {
            LayoutKey::Transition => &[wgpu::PushConstantRange {
                stages: ShaderStages::FRAGMENT,
                range: 0..64,
            }],
            _ => &[],
        }
    }
}

impl BindGroupKey {
    fn entries(&self) -> Vec<wgpu::BindGroupLayoutEntry> {
        match self {
            BindGroupKey::Texture => vec![
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            BindGroupKey::ColorGrade => vec![
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D3,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
            BindGroupKey::Transition => vec![
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            BindGroupKey::Scopes => vec![
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            BindGroupKey::Compute => vec![
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        }
    }
}

impl ShaderKey {
    fn source(&self) -> &'static str {
        match self {
            ShaderKey::TextureQuad => include_str!("../assets/shaders/quad.wgsl"),
            ShaderKey::ColorGrade => include_str!("../assets/shaders/color_grade.wgsl"),
            ShaderKey::Transition(t) => match t {
                TransitionPipelineType::CrossDissolve => include_str!("../assets/shaders/transitions/cross_dissolve.wgsl"),
                TransitionPipelineType::Wipe => include_str!("../assets/shaders/transitions/wipe.wgsl"),
                TransitionPipelineType::Slide => include_str!("../assets/shaders/transitions/slide.wgsl"),
                TransitionPipelineType::Push => include_str!("../assets/shaders/transitions/push.wgsl"),
                TransitionPipelineType::DipToColor => include_str!("../assets/shaders/transitions/dip_to_color.wgsl"),
                TransitionPipelineType::MorphCut => include_str!("../assets/shaders/transitions/morph_cut.wgsl"),
            },
            ShaderKey::Scopes => include_str!("../assets/shaders/scopes.wgsl"),
            ShaderKey::Compute(c) => match c {
                ComputeShaderType::Thumbnail => include_str!("../assets/shaders/compute/thumbnail.wgsl"),
                ComputeShaderType::Waveform => include_str!("../assets/shaders/compute/waveform.wgsl"),
                ComputeShaderType::Histogram => include_str!("../assets/shaders/compute/histogram.wgsl"),
                ComputeShaderType::Vectorscope => include_str!("../assets/shaders/compute/vectorscope.wgsl"),
                ComputeShaderType::OpticalFlow => include_str!("../assets/shaders/compute/optical_flow.wgsl"),
                ComputeShaderType::NoiseReduction => include_str!("../assets/shaders/compute/noise_reduction.wgsl"),
                ComputeShaderType::Stabilize => include_str!("../assets/shaders/compute/stabilize.wgsl"),
                ComputeShaderType::Transcode => include_str!("../assets/shaders/compute/transcode.wgsl"),
            },
            ShaderKey::Custom(_) => include_str!("../assets/shaders/quad.wgsl"),
        }
    }
}

/// Render pipeline wrapper
pub struct RenderPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub layout: Arc<PipelineLayout>,
    pub bind_group_layout: Arc<BindGroupLayout>,
}