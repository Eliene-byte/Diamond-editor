//! Main GPU renderer

use crate::core::types::{Resolution, PixelFormat, ColorSpace, FrameRate};
use crate::core::time::Frame;
use crate::gpu::pipeline::{PipelineManager, RenderPipeline};
use crate::gpu::texture::{TextureManager, GpuTexture};
use crate::gpu::buffer::{BufferManager, UniformBuffer};
use crate::gpu::color::ColorPipeline;
use crate::gpu::scopes::ScopesRenderer;
use crate::gpu::compute::ComputePipeline;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration, TextureFormat, PresentMode, CompositeAlphaMode};
use std::sync::Arc;
use parking_lot::RwLock;
use crossbeam::channel::{Sender, Receiver, unbounded};

/// Render command
#[derive(Debug)]
pub enum RenderCommand {
    Clear { color: [f32; 4] },
    DrawTexture { texture: GpuTexture, dest_rect: [f32; 4], src_rect: [f32; 4], opacity: f32, blend_mode: BlendMode },
    DrawQuad { color: [f32; 4], rect: [f32; 4] },
    ApplyLut { input: GpuTexture, output: GpuTexture, lut: GpuTexture },
    ApplyColorGrade { input: GpuTexture, output: GpuTexture, grade: ColorGradeUniforms },
    Composite { layers: Vec<CompositeLayer>, output: GpuTexture },
    Compute { pipeline: ComputePipelineType, params: ComputeParams },
    Present,
}

#[derive(Debug, Clone, Copy)]
pub enum BlendMode {
    Normal, Add, Screen, Multiply, Overlay, Darken, Lighten,
    ColorDodge, ColorBurn, SoftLight, HardLight, Difference, Exclusion,
}

#[derive(Debug, Clone)]
pub struct CompositeLayer {
    pub texture: GpuTexture,
    pub rect: [f32; 4],
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub transform: [f32; 16], // 4x4 matrix
}

#[derive(Debug, Clone)]
pub struct ColorGradeUniforms {
    pub lift: [f32; 3],
    pub gamma: [f32; 3],
    pub gain: [f32; 3],
    pub offset: [f32; 3],
    pub contrast: f32,
    pub pivot: f32,
    pub saturation: f32,
    pub temperature: f32,
    pub tint: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum ComputePipelineType {
    Thumbnail,
    Waveform,
    Histogram,
    Vectorscope,
    Transcode,
    OpticalFlow,
    NoiseReduction,
    Stabilize,
}

#[derive(Debug, Clone)]
pub struct ComputeParams {
    pub input: GpuTexture,
    pub output: Option<GpuTexture>,
    pub params: [f32; 16],
}

/// GPU Renderer
pub struct GpuRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Option<Surface<'static>>,
    config: RwLock<SurfaceConfiguration>,
    
    pipeline_manager: PipelineManager,
    texture_manager: TextureManager,
    buffer_manager: BufferManager,
    color_pipeline: ColorPipeline,
    scopes_renderer: ScopesRenderer,
    compute_pipeline: ComputePipeline,
    
    render_tx: Sender<RenderCommand>,
    render_rx: Receiver<RenderCommand>,
    
    frame_count: u64,
    stats: RenderStats,
}

#[derive(Debug, Default, Clone)]
pub struct RenderStats {
    pub frame_time_ms: f32,
    pub gpu_time_ms: f32,
    pub draw_calls: u32,
    pub triangles: u32,
    pub texture_memory_mb: f32,
    pub buffer_memory_mb: f32,
}

impl GpuRenderer {
    pub async fn new(
        width: u32,
        height: u32,
        present_mode: PresentMode,
        vsync: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        
        // Create surface (would need window handle)
        // For now, create headless
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }).await.ok_or("No GPU adapter found")?;
        
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("DaVinci Clone Device"),
                required_features: wgpu::Features::TEXTURE_COMPRESSION_BC
                    | wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::PUSH_CONSTANTS
                    | wgpu::Features::STORAGE_RESOURCE_BINDING_ARRAY,
                required_limits: wgpu::Limits {
                    max_push_constant_size: 256,
                    max_storage_buffers_per_shader_stage: 16,
                    max_storage_textures_per_shader_stage: 16,
                    ..Default::default()
                },
                memory_hints: Default::default(),
            },
            None,
        ).await?;
        
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        
        let (render_tx, render_rx) = unbounded();
        
        let pipeline_manager = PipelineManager::new(device.clone());
        let texture_manager = TextureManager::new(device.clone(), queue.clone());
        let buffer_manager = BufferManager::new(device.clone(), queue.clone());
        let color_pipeline = ColorPipeline::new(device.clone());
        let scopes_renderer = ScopesRenderer::new(device.clone());
        let compute_pipeline = ComputePipeline::new(device.clone());
        
        Ok(Self {
            device,
            queue,
            surface: None,
            config: RwLock::new(SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: TextureFormat::Bgra8UnormSrgb,
                width,
                height,
                present_mode,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            }),
            pipeline_manager,
            texture_manager,
            buffer_manager,
            color_pipeline,
            scopes_renderer,
            compute_pipeline,
            render_tx,
            render_rx,
            frame_count: 0,
            stats: RenderStats::default(),
        })
    }

    pub fn configure_surface(&self, surface: Surface<'static>, width: u32, height: u32) {
        let mut config = self.config.write();
        config.width = width;
        config.height = height;
        surface.configure(&self.device, &config);
        *self.surface.write() = Some(surface);
    }

    pub fn resize(&self, width: u32, height: u32) {
        let mut config = self.config.write();
        config.width = width;
        config.height = height;
        if let Some(surface) = &*self.surface.read() {
            surface.configure(&self.device, &config);
        }
    }

    pub fn submit(&self, command: RenderCommand) {
        self.render_tx.send(command).ok();
    }

    pub fn render_frame(&mut self, timeline_frame: Frame) -> Result<Option<wgpu::TextureView>, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();
        
        // Process commands
        while let Ok(cmd) = self.render_rx.try_recv() {
            self.execute_command(cmd)?;
        }
        
        // Get surface texture
        let surface_texture = if let Some(surface) = &*self.surface.read() {
            surface.get_current_texture().ok()
        } else {
            None
        };
        
        let output_view = surface_texture.as_ref().map(|t| t.texture.create_view(&Default::default()));
        
        // Present
        if let Some(surface_texture) = surface_texture {
            surface_texture.present();
        }
        
        self.frame_count += 1;
        self.stats.frame_time_ms = start.elapsed().as_secs_f32() * 1000.0;
        
        Ok(output_view)
    }

    fn execute_command(&mut self, cmd: RenderCommand) -> Result<(), Box<dyn std::error::Error>> {
        match cmd {
            RenderCommand::Clear { color } => {
                // Would clear current render target
            }
            RenderCommand::DrawTexture { texture, dest_rect, src_rect, opacity, blend_mode } => {
                self.draw_texture(texture, dest_rect, src_rect, opacity, blend_mode)?;
            }
            RenderCommand::ApplyColorGrade { input, output, grade } => {
                self.color_pipeline.apply_grade(&self.device, &self.queue, input, output, grade)?;
            }
            RenderCommand::Composite { layers, output } => {
                self.composite(layers, output)?;
            }
            RenderCommand::Compute { pipeline, params } => {
                self.compute_pipeline.dispatch(&self.device, &self.queue, pipeline, params)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn draw_texture(
        &mut self,
        texture: GpuTexture,
        dest_rect: [f32; 4],
        src_rect: [f32; 4],
        opacity: f32,
        blend_mode: BlendMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Draw textured quad
        Ok(())
    }

    fn composite(
        &mut self,
        layers: Vec<CompositeLayer>,
        output: GpuTexture,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Composite multiple layers
        Ok(())
    }

    pub fn create_timeline_texture(&mut self, width: u32, height: u32) -> GpuTexture {
        self.texture_manager.create_render_target(width, height)
    }

    pub fn upload_frame(&mut self, data: &[u8], width: u32, height: u32, format: PixelFormat) -> GpuTexture {
        self.texture_manager.upload(data, width, height, format)
    }

    pub fn stats(&self) -> RenderStats { self.stats.clone() }
    
    pub fn device(&self) -> &Arc<Device> { &self.device }
    pub fn queue(&self) -> &Arc<Queue> { &self.queue }
}

/// Headless renderer for export/render
pub struct HeadlessRenderer {
    renderer: GpuRenderer,
    export_format: PixelFormat,
}

impl HeadlessRenderer {
    pub async fn new(width: u32, height: u32, format: PixelFormat) -> Result<Self, Box<dyn std::error::Error>> {
        let renderer = GpuRenderer::new(width, height, PresentMode::AutoVsync, false).await?;
        Ok(Self { renderer, export_format: format })
    }

    pub fn render_timeline(&mut self, timeline: &crate::timeline::clip::Timeline, frame: Frame) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Render single frame
        Ok(Vec::new())
    }

    pub fn render_range(&mut self, timeline: &crate::timeline::clip::Timeline, range: crate::core::time::FrameRange) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        // Render frame range
        Ok(Vec::new())
    }
}