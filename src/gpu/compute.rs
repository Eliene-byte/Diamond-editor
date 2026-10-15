//! Compute pipelines for GPU processing

use wgpu::{Device, Queue, BindGroup, BindGroupLayout, PipelineLayout};
use crate::gpu::texture::{TextureManager, GpuTexture};
use crate::gpu::buffer::{UniformBuffer, StorageBuffer};
use crate::gpu::renderer::ComputeParams;
use crate::gpu::renderer::ComputePipelineType;
use std::sync::Arc;

/// One fully-built compute pipeline plus the bind group layout it was built against.
#[derive(Debug, Clone)]
pub struct ComputePipelineEntry {
    pub pipeline: Arc<wgpu::ComputePipeline>,
    pub layout: Arc<PipelineLayout>,
    pub bind_group_layout: Arc<BindGroupLayout>,
}

/// Registry of every compute pipeline kind the renderer can dispatch.
pub struct ComputePipeline {
    device: Arc<Device>,
    pipelines: std::collections::HashMap<ComputePipelineType, ComputePipelineEntry>,
    bind_group_layouts: std::collections::HashMap<ComputePipelineType, Arc<BindGroupLayout>>,
}

impl ComputePipeline {
    pub fn new(device: Arc<Device>) -> Self {
        let mut pipelines = std::collections::HashMap::new();
        let mut bind_group_layouts = std::collections::HashMap::new();
        
        for pipeline_type in [
            ComputePipelineType::Thumbnail,
            ComputePipelineType::Waveform,
            ComputePipelineType::Histogram,
            ComputePipelineType::Vectorscope,
            ComputePipelineType::OpticalFlow,
            ComputePipelineType::NoiseReduction,
            ComputePipelineType::Stabilize,
            ComputePipelineType::Transcode,
        ] {
            let (pipeline, layout, bind_group_layout) = Self::create_pipeline(&device, pipeline_type);
            pipelines.insert(pipeline_type, ComputePipelineEntry { pipeline: Arc::new(pipeline), layout: Arc::new(layout), bind_group_layout: Arc::new(bind_group_layout.clone()) });
            bind_group_layouts.insert(pipeline_type, Arc::new(bind_group_layout));
        }
        
        Self { device, pipelines, bind_group_layouts }
    }

    fn create_pipeline(
        device: &Device,
        pipeline_type: ComputePipelineType,
    ) -> (wgpu::ComputePipeline, PipelineLayout, BindGroupLayout) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{:?} Compute Shader", pipeline_type)),
            source: wgpu::ShaderSource::Wgsl(Self::shader_source(pipeline_type).into()),
        });
        
        let bind_group_layout = Self::bind_group_layout(device, pipeline_type);
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{:?} Compute Layout", pipeline_type)),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&format!("{:?} Compute Pipeline", pipeline_type)),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        (pipeline, pipeline_layout, bind_group_layout)
    }

    fn bind_group_layout(device: &Device, pipeline_type: ComputePipelineType) -> BindGroupLayout {
        let entries = match pipeline_type {
            ComputePipelineType::Thumbnail | ComputePipelineType::Transcode => vec![
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba8Unorm, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
            ComputePipelineType::Waveform | ComputePipelineType::Histogram | ComputePipelineType::Vectorscope => vec![
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageBuffer { read_only: false, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
            _ => vec![
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba32Float, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        };
        
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{:?} Compute Bind Group Layout", pipeline_type)),
            entries: &entries,
        })
    }

    fn shader_source(pipeline_type: ComputePipelineType) -> &'static str {
        match pipeline_type {
            ComputePipelineType::Thumbnail => THUMBNAIL_SHADER,
            ComputePipelineType::Waveform => WAVEFORM_SHADER,
            ComputePipelineType::Histogram => HISTOGRAM_SHADER,
            ComputePipelineType::Vectorscope => VECTORSCOPE_SHADER,
            ComputePipelineType::OpticalFlow => OPTICAL_FLOW_SHADER,
            ComputePipelineType::NoiseReduction => NOISE_REDUCTION_SHADER,
            ComputePipelineType::Stabilize => STABILIZE_SHADER,
            ComputePipelineType::Transcode => TRANSCODE_SHADER,
        }
    }

    pub fn dispatch(
        &self,
        device: &Device,
        queue: &Queue,
        pipeline_type: ComputePipelineType,
        params: ComputeParams,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pipeline = self.pipelines.get(&pipeline_type).ok_or("Pipeline not found")?;
        
        // Create bind group
        let bind_group_layout = self.bind_group_layouts.get(&pipeline_type).unwrap();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{:?} Compute Bind Group", pipeline_type)),
            layout: bind_group_layout,
            entries: &params.bind_group_entries(),
        });
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Compute Encoder") });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(&format!("{:?} Pass", pipeline_type)), timestamp_writes: None });
            compute_pass.set_pipeline(&pipeline.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let (width, height) = params.input.size();
            let workgroups_x = (width + 15) / 16;
            let workgroups_y = (height + 15) / 16;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }
        
        queue.submit(Some(encoder.finish()));
        Ok(())
    }
}

impl ComputeParams {
    fn bind_group_entries(&self) -> Vec<wgpu::BindGroupEntry> {
        vec![
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.output.as_ref().unwrap_or(&self.input).view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.input.view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(self.create_uniform_buffer().as_entire_binding()) },
        ]
    }
    
    fn create_uniform_buffer(&self) -> UniformBuffer {
        // Would create uniform buffer from params
        unimplemented!()
    }
}

const THUMBNAIL_SHADER: &str = r#"
@group(0) @binding(0) var outputTexture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

@compute @workgroup_size(16, 16) fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    
    let scale_x = f32(dims.x) / f32(textureDimensions(outputTexture).x);
    let scale_y = f32(dims.y) / f32(textureDimensions(outputTexture).y);
    let src = vec2<f32>(f32(id.x) * scale_x, f32(id.y) * scale_y);
    let color = textureSampleLevel(inputTexture, sampler, src / vec2<f32>(dims), 0.0);
    textureStore(outputTexture, id.xy, color);
}
"#;

const WAVEFORM_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> waveformData: array<f32>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

@compute @workgroup_size(256) fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    let x = id.x;
    if (x >= dims.x) { return; }
    
    var r_max = 0.0, g_max = 0.0, b_max = 0.0;
    var r_min = 1.0, g_min = 1.0, b_min = 1.0;
    var r_sum = 0.0, g_sum = 0.0, b_sum = 0.0;
    
    for (var y = 0u; y < dims.y; y++) {
        let color = textureLoad(inputTexture, vec2<u32>(x, y), 0);
        r_max = max(r_max, color.r);
        g_max = max(g_max, color.g);
        b_max = max(b_max, color.b);
        r_min = min(r_min, color.r);
        g_min = min(g_min, color.g);
        b_min = min(b_min, color.b);
        r_sum += color.r;
        g_sum += color.g;
        b_sum += color.b;
    }
    
    let count = f32(dims.y);
    waveformData[x * 3 + 0] = (r_max + r_min) * 0.5;
    waveformData[x * 3 + 1] = (g_max + g_min) * 0.5;
    waveformData[x * 3 + 2] = (b_max + b_min) * 0.5;
}
"#;

const HISTOGRAM_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> histogramData: array<u32>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

@compute @workgroup_size(256) fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    let idx = id.x + id.y * dims.x;
    if (idx >= dims.x * dims.y) { return; }
    
    let x = idx % dims.x;
    let y = idx / dims.x;
    let color = textureLoad(inputTexture, vec2<u32>(x, y), 0);
    
    let r_bin = u32(color.r * 255.0);
    let g_bin = u32(color.g * 255.0);
    let b_bin = u32(color.b * 255.0);
    let l_bin = u32((0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b) * 255.0);
    
    atomicAdd(&histogramData[r_bin], 1u);
    atomicAdd(&histogramData[256 + g_bin], 1u);
    atomicAdd(&histogramData[512 + b_bin], 1u);
    atomicAdd(&histogramData[768 + l_bin], 1u);
}
"#;

const VECTORSCOPE_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> vectorscopeData: array<u32>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>;

@compute @workgroup_size(256) fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    let idx = id.x + id.y * dims.x;
    if (idx >= dims.x * dims.y) { return; }
    
    let x = idx % dims.x;
    let y = idx / dims.x;
    let color = textureLoad(inputTexture, vec2<u32>(x, y), 0);
    
    // Convert RGB to YCbCr
    let cb = 0.5 + 0.564 * (color.b - (0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b));
    let cr = 0.5 + 0.713 * (color.r - (0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b));
    
    let ix = u32(clamp(cb * 255.0, 0.0, 255.0));
    let iy = u32(clamp(cr * 255.0, 0.0, 255.0));
    
    atomicAdd(&vectorscopeData[iy * 256 + ix], 1u);
}
"#;

const OPTICAL_FLOW_SHADER: &str = r#"
// Simplified optical flow - would use pyramid Lucas-Kanade
@group(0) @binding(0) var outputFlow: texture_storage_2d<rg32float, write>;
@group(0) @binding(1) var prevFrame: texture_2d<f32>;
@group(0) @binding(2) var currFrame: texture_2d<f32>;
@group(0) @binding(3) var<uniform> params: vec4<f32>;

@compute @workgroup_size(16, 16) fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    // Placeholder - real implementation needs multi-scale pyramid
    let dims = textureDimensions(prevFrame);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    textureStore(outputFlow, id.xy, vec2<f32>(0.0, 0.0));
}
"#;

const NOISE_REDUCTION_SHADER: &str = r#"
@group(0) @binding(0) var outputTexture: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>; // spatial, temporal, strength, _

@compute @workgroup_size(16, 16) fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    
    let uv = vec2<f32>(id.xy) / vec2<f32>(dims);
    let spatial = params.x;
    let strength = params.z;
    
    var sum = vec4<f32>(0.0);
    var weight_sum = 0.0;
    let radius = u32(spatial * 3.0);
    
    for (var dy = -radius; dy <= radius; dy++) {
        for (var dx = -radius; dx <= radius; dx++) {
            let sample_x = clamp(i32(id.x) + i32(dx), 0, i32(dims.x) - 1);
            let sample_y = clamp(i32(id.y) + i32(dy), 0, i32(dims.y) - 1);
            let color = textureLoad(inputTexture, vec2<u32>(sample_x, sample_y), 0);
            
            let dist = length(vec2<f32>(dx, dy));
            let spatial_weight = exp(-dist * dist / (2.0 * spatial * spatial));
            let color_diff = length(color.rgb - textureLoad(inputTexture, id.xy, 0).rgb);
            let range_weight = exp(-color_diff * color_diff / (2.0 * 0.1 * 0.1));
            
            let weight = spatial_weight * range_weight;
            sum += color * weight;
            weight_sum += weight;
        }
    }
    
    let filtered = sum / weight_sum;
    let original = textureLoad(inputTexture, id.xy, 0);
    textureStore(outputTexture, id.xy, mix(original, filtered, strength));
}
"#;

const STABILIZE_SHADER: &str = r#"
// Video stabilization - placeholder
@group(0) @binding(0) var outputTexture: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>; // transform matrix

@compute @workgroup_size(16, 16) fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let uv = vec2<f32>(id.xy) / vec2<f32>(dims);
    // Apply transform
    textureStore(outputTexture, id.xy, textureSample(inputTexture, sampler, uv));
}
"#;

const TRANSCODE_SHADER: &str = r#"
@group(0) @binding(0) var outputTexture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: vec4<f32>; // color space, bit depth, _

@compute @workgroup_size(16, 16) fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(inputTexture);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let color = textureLoad(inputTexture, id.xy, 0);
    // Color space conversion would happen here
    textureStore(outputTexture, id.xy, vec4<u32>(vec3<u32>(color.rgb * 255.0), 255u));
}
"#;