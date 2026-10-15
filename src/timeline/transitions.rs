//! Transitions between clips

use crate::core::id::{TrackItemId, TransitionId};
use crate::core::time::{Frame, FrameRange};
use crate::core::types::{Easing, EffectParams, EffectType, EffectValue, Interpolation};
use crate::timeline::clip::Timeline;
use std::collections::HashMap;

/// Transition instance on timeline
#[derive(Debug, Clone)]
pub struct Transition {
    pub id: TransitionId,
    pub transition_type: TransitionType,
    pub left_item: TrackItemId,
    pub right_item: TrackItemId,
    pub alignment: TransitionAlignment,
    pub duration: Frame,
    pub offset: Frame, // Offset from edit point
    pub params: EffectParams,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransitionType {
    // Dissolves
    CrossDissolve,
    FilmDissolve,
    AdditiveDissolve,
    // Wipes
    WipeLeft,
    WipeRight,
    WipeUp,
    WipeDown,
    WipeCenter,
    WipeClock,
    WipeCounterClock,
    // Slides
    SlideLeft,
    SlideRight,
    SlideUp,
    SlideDown,
    // Push
    PushLeft,
    PushRight,
    PushUp,
    PushDown,
    // 3D
    CubeSpin,
    FlipOver,
    PagePeel,
    // Special
    DipToColor,
    MorphCut,
    SmoothCut,
    // Audio
    CrossFade,
    ConstantPower,
    ConstantGain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionAlignment {
    CenterAtCut,
    StartAtCut,
    EndAtCut,
}

impl Transition {
    pub fn new(
        transition_type: TransitionType,
        left_item: TrackItemId,
        right_item: TrackItemId,
        duration: Frame,
    ) -> Self {
        Self {
            id: TransitionId::generate(),
            transition_type,
            left_item,
            right_item,
            alignment: TransitionAlignment::CenterAtCut,
            duration,
            offset: Frame::ZERO,
            params: Self::default_params(transition_type),
            enabled: true,
        }
    }

    fn default_params(transition_type: TransitionType) -> EffectParams {
        use std::collections::HashMap;
        let mut params = HashMap::new();

        match transition_type {
            TransitionType::CrossDissolve
            | TransitionType::FilmDissolve
            | TransitionType::AdditiveDissolve => {
                params.insert("ease".into(), EffectValue::Float(0.5));
            }
            TransitionType::WipeLeft
            | TransitionType::WipeRight
            | TransitionType::WipeUp
            | TransitionType::WipeDown => {
                params.insert("border_width".into(), EffectValue::Float(0.0));
                params.insert(
                    "border_color".into(),
                    EffectValue::Color(1.0, 1.0, 1.0, 1.0),
                );
                params.insert("feather".into(), EffectValue::Float(0.0));
            }
            TransitionType::SlideLeft
            | TransitionType::SlideRight
            | TransitionType::SlideUp
            | TransitionType::SlideDown => {
                params.insert("ease".into(), EffectValue::Float(0.5));
            }
            TransitionType::PushLeft
            | TransitionType::PushRight
            | TransitionType::PushUp
            | TransitionType::PushDown => {
                params.insert("ease".into(), EffectValue::Float(0.5));
                params.insert("overlap".into(), EffectValue::Bool(true));
            }
            TransitionType::DipToColor => {
                params.insert("color".into(), EffectValue::Color(0.0, 0.0, 0.0, 1.0));
                params.insert("ease".into(), EffectValue::Float(0.5));
            }
            TransitionType::CrossFade
            | TransitionType::ConstantPower
            | TransitionType::ConstantGain => {
                params.insert("curve".into(), EffectValue::Float(0.5));
            }
            _ => {}
        }

        EffectParams {
            effect_type: EffectType::CrossDissolve,
            parameters: params,
        }
    }

    pub fn range(&self, edit_point: Frame) -> FrameRange {
        match self.alignment {
            TransitionAlignment::CenterAtCut => FrameRange::new(
                edit_point - self.duration / 2,
                edit_point + self.duration / 2,
            ),
            TransitionAlignment::StartAtCut => {
                FrameRange::new(edit_point, edit_point + self.duration)
            }
            TransitionAlignment::EndAtCut => {
                FrameRange::new(edit_point - self.duration, edit_point)
            }
        }
    }

    pub fn progress_at(&self, frame: Frame, edit_point: Frame) -> f32 {
        let range = self.range(edit_point);
        if !range.contains(frame) {
            return 0.0;
        }

        let progress = (frame - range.start).0 as f32 / self.duration.0 as f32;
        progress.clamp(0.0, 1.0)
    }

    pub fn eased_progress(&self, frame: Frame, edit_point: Frame) -> f32 {
        let t = self.progress_at(frame, edit_point);
        self.apply_easing(t)
    }

    fn apply_easing(&self, t: f32) -> f32 {
        let ease = self
            .params
            .parameters
            .get("ease")
            .and_then(|v| match v {
                EffectValue::Float(f) => Some(*f),
                _ => None,
            })
            .unwrap_or(0.5);

        // Smoothstep with variable ease
        if t < ease {
            t * t / (2.0 * ease)
        } else if t > 1.0 - ease {
            1.0 - (1.0 - t) * (1.0 - t) / (2.0 * ease)
        } else {
            (t - ease / 2.0) / (1.0 - ease)
        }
    }
}

/// Transition manager
pub struct TransitionManager {
    transitions: HashMap<TransitionId, Transition>,
}

impl TransitionManager {
    pub fn new() -> Self {
        Self {
            transitions: HashMap::new(),
        }
    }

    pub fn add(&mut self, transition: Transition) -> TransitionId {
        let id = transition.id;
        self.transitions.insert(id, transition);
        id
    }

    pub fn remove(&mut self, id: TransitionId) -> Option<Transition> {
        self.transitions.remove(&id)
    }

    pub fn get(&self, id: TransitionId) -> Option<&Transition> {
        self.transitions.get(&id)
    }

    pub fn get_mut(&mut self, id: TransitionId) -> Option<&mut Transition> {
        self.transitions.get_mut(&id)
    }

    pub fn at_edit_point(&self, left: TrackItemId, right: TrackItemId) -> Option<&Transition> {
        self.transitions
            .values()
            .find(|t| t.left_item == left && t.right_item == right)
    }

    pub fn for_item(&self, item: TrackItemId) -> Vec<&Transition> {
        self.transitions
            .values()
            .filter(|t| t.left_item == item || t.right_item == item)
            .collect()
    }

    pub fn update(&mut self, id: TransitionId, f: impl FnOnce(&mut Transition)) -> bool {
        if let Some(t) = self.transitions.get_mut(&id) {
            f(t);
            true
        } else {
            false
        }
    }

    pub fn set_duration(&mut self, id: TransitionId, duration: Frame) -> bool {
        self.update(id, |t| t.duration = duration)
    }

    pub fn set_alignment(&mut self, id: TransitionId, alignment: TransitionAlignment) -> bool {
        self.update(id, |t| t.alignment = alignment)
    }

    pub fn set_params(&mut self, id: TransitionId, params: EffectParams) -> bool {
        self.update(id, |t| t.params = params)
    }

    pub fn clear(&mut self) {
        self.transitions.clear();
    }

    pub fn all(&self) -> Vec<&Transition> {
        self.transitions.values().collect()
    }
}

/// GPU transition renderer
pub struct TransitionRenderer {
    pipelines: HashMap<TransitionType, wgpu::RenderPipeline>,
    bind_group_layouts: HashMap<TransitionType, wgpu::BindGroupLayout>,
}

impl TransitionRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let mut pipelines = HashMap::new();
        let mut bind_group_layouts = HashMap::new();

        // Create pipelines for each transition type
        for tt in [
            TransitionType::CrossDissolve,
            TransitionType::WipeLeft,
            TransitionType::SlideLeft,
            TransitionType::PushLeft,
            TransitionType::DipToColor,
        ] {
            let (pipeline, layout) = Self::create_pipeline(device, format, tt);
            pipelines.insert(tt, pipeline);
            bind_group_layouts.insert(tt, layout);
        }

        Self {
            pipelines,
            bind_group_layouts,
        }
    }

    fn create_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        transition_type: TransitionType,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
        let shader = Self::shader_for_type(transition_type);
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("Transition {:?}", transition_type)),
            source: wgpu::ShaderSource::Wgsl(shader.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Transition Bind Group Layout"),
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
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Transition Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("Transition {:?} Pipeline", transition_type)),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        (pipeline, bind_group_layout)
    }

    fn shader_for_type(transition_type: TransitionType) -> &'static str {
        match transition_type {
            TransitionType::CrossDissolve => {
                include_str!("../assets/shaders/transitions/cross_dissolve.wgsl")
            }
            TransitionType::WipeLeft => {
                include_str!("../assets/shaders/transitions/wipe_left.wgsl")
            }
            TransitionType::SlideLeft => {
                include_str!("../assets/shaders/transitions/slide_left.wgsl")
            }
            TransitionType::PushLeft => {
                include_str!("../assets/shaders/transitions/push_left.wgsl")
            }
            TransitionType::DipToColor => {
                include_str!("../assets/shaders/transitions/dip_to_color.wgsl")
            }
            _ => include_str!("../assets/shaders/transitions/cross_dissolve.wgsl"),
        }
    }

    pub fn render(
        &self,
        // `CommandEncoder` has no `device()` accessor in wgpu 23, so the device is
        // threaded in by the caller.
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        transition: &Transition,
        left_texture: &wgpu::TextureView,
        right_texture: &wgpu::TextureView,
        params_buffer: &wgpu::Buffer,
        output_view: &wgpu::TextureView,
    ) {
        if let Some(pipeline) = self.pipelines.get(&transition.transition_type) {
            let layout = self
                .bind_group_layouts
                .get(&transition.transition_type)
                .unwrap();

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Transition Bind Group"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(left_texture),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(right_texture),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Transition Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
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

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }
    }
}

/// Transition parameters uniform
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransitionUniforms {
    pub progress: f32,
    pub duration: f32,
    pub params: [f32; 14], // Generic params
}
