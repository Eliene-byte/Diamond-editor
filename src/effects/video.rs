//! Video effects - GPU accelerated

use crate::gpu::renderer::{GpuRenderer, RenderCommand, ComputeParams, ComputePipelineType};
use crate::gpu::texture::{TextureManager, GpuTexture};
use crate::core::types::{EffectType, EffectParams, EffectValue, Interpolation, Easing, Keyframe};
use crate::core::time::Frame;
use crate::core::id::EffectId;
use std::collections::HashMap;

/// Video effect instance
#[derive(Debug, Clone)]
pub struct VideoEffect {
    pub id: EffectId,
    pub effect_type: EffectType,
    pub enabled: bool,
    pub params: EffectParams,
    pub keyframes: HashMap<String, Vec<Keyframe>>,
    pub mix: f32, // 0.0 - 1.0
}

impl VideoEffect {
    pub fn new(effect_type: EffectType) -> Self {
        Self {
            id: EffectId::generate(),
            effect_type,
            enabled: true,
            params: Self::default_params(effect_type),
            keyframes: HashMap::new(),
            mix: 1.0,
        }
    }

    fn default_params(effect_type: EffectType) -> EffectParams {
        let mut params = HashMap::new();
        
        match effect_type {
            EffectType::Transform => {
                params.insert("position".into(), EffectValue::Vec2(0.0, 0.0));
                params.insert("scale".into(), EffectValue::Vec2(1.0, 1.0));
                params.insert("rotation".into(), EffectValue::Float(0.0));
                params.insert("anchor".into(), EffectValue::Vec2(0.5, 0.5));
                params.insert("flip_h".into(), EffectValue::Bool(false));
                params.insert("flip_v".into(), EffectValue::Bool(false));
            }
            EffectType::Crop => {
                params.insert("left".into(), EffectValue::Float(0.0));
                params.insert("right".into(), EffectValue::Float(0.0));
                params.insert("top".into(), EffectValue::Float(0.0));
                params.insert("bottom".into(), EffectValue::Float(0.0));
                params.insert("feather".into(), EffectValue::Float(0.0));
            }
            EffectType::Opacity => {
                params.insert("opacity".into(), EffectValue::Float(1.0));
            }
            EffectType::BlendMode => {
                params.insert("mode".into(), EffectValue::Int(0)); // Normal
            }
            EffectType::Speed => {
                params.insert("speed".into(), EffectValue::Float(1.0));
                params.insert("reverse".into(), EffectValue::Bool(false));
                params.insert("ripple".into(), EffectValue::Bool(false));
                params.insert("frame_blending".into(), EffectValue::Bool(false));
                params.insert("optical_flow".into(), EffectValue::Bool(false));
            }
            EffectType::Stabilize => {
                params.insert("smoothness".into(), EffectValue::Float(0.5));
                params.insert("method".into(), EffectValue::Int(0)); // Position, Position+Scale, Perspective
                params.insert("borders".into(), EffectValue::Int(0)); // Stabilize, Stabilize+Crop, Stabilize+Crop+Auto-scale
                params.insert("crop_less".into(), EffectValue::Float(0.0));
            }
            EffectType::LiftGammaGain => {
                params.insert("lift".into(), EffectValue::Vec3(0.0, 0.0, 0.0));
                params.insert("gamma".into(), EffectValue::Vec3(1.0, 1.0, 1.0));
                params.insert("gain".into(), EffectValue::Vec3(1.0, 1.0, 1.0));
            }
            EffectType::ContrastSaturation => {
                params.insert("contrast".into(), EffectValue::Float(1.0));
                params.insert("saturation".into(), EffectValue::Float(1.0));
                params.insert("pivot".into(), EffectValue::Float(0.5));
            }
            EffectType::ColorWheels => {
                params.insert("shadows".into(), EffectValue::Vec3(0.0, 0.0, 0.0));
                params.insert("midtones".into(), EffectValue::Vec3(0.0, 0.0, 0.0));
                params.insert("highlights".into(), EffectValue::Vec3(0.0, 0.0, 0.0));
                params.insert("offset".into(), EffectValue::Vec3(0.0, 0.0, 0.0));
                params.insert("shadows_range".into(), EffectValue::Float(0.33));
                params.insert("highlights_range".into(), EffectValue::Float(0.66));
            }
            EffectType::Curves => {
                params.insert("master".into(), EffectValue::Keyframes(vec![]));
                params.insert("red".into(), EffectValue::Keyframes(vec![]));
                params.insert("green".into(), EffectValue::Keyframes(vec![]));
                params.insert("blue".into(), EffectValue::Keyframes(vec![]));
            }
            EffectType::Lut3d => {
                params.insert("lut".into(), EffectValue::String(String::new()));
                params.insert("mix".into(), EffectValue::Float(1.0));
            }
            EffectType::CrossDissolve | EffectType::DipToColor | EffectType::Push | EffectType::Slide | EffectType::Wipe => {
                params.insert("duration".into(), EffectValue::Float(1.0));
                params.insert("ease".into(), EffectValue::Float(0.5));
                params.insert("alignment".into(), EffectValue::Int(0)); // Center at cut
            }
            _ => {}
        }
        
        EffectParams { effect_type, parameters: params }
    }

    pub fn get_param(&self, name: &str) -> Option<&EffectValue> {
        self.params.parameters.get(name)
    }

    pub fn set_param(&mut self, name: &str, value: EffectValue) {
        self.params.parameters.insert(name.to_string(), value);
    }

    pub fn get_keyframe_value(&self, param: &str, frame: Frame) -> Option<EffectValue> {
        self.keyframes.get(param).and_then(|kfs| {
            kfs.iter().find(|kf| kf.frame == frame).map(|kf| kf.value.clone())
        })
    }

    pub fn interpolated_value(&self, param: &str, frame: Frame) -> Option<EffectValue> {
        let kfs = self.keyframes.get(param)?;
        if kfs.is_empty() { return None; }
        
        let mut prev = None;
        let mut next = None;
        
        for kf in kfs {
            if kf.frame <= frame {
                prev = Some(kf);
            } else {
                next = Some(kf);
                break;
            }
        }
        
        match (prev, next) {
            (Some(p), Some(n)) if p.frame != n.frame => {
                let t = (frame.0 - p.frame.0) as f32 / (n.frame.0 - p.frame.0) as f32;
                Self::interpolate_values(&p.value, &n.value, t, p.interpolation, p.easing)
            }
            (Some(p), _) => Some(p.value.clone()),
            (_, Some(n)) => Some(n.value.clone()),
            _ => None,
        }
    }

    fn interpolate_values(a: &EffectValue, b: &EffectValue, t: f32, interp: Interpolation, easing: Easing) -> Option<EffectValue> {
        let t = Self::apply_easing(t, easing);
        match (a, b) {
            (EffectValue::Float(a), EffectValue::Float(b)) => Some(EffectValue::Float(Self::lerp(*a, *b, t))),
            (EffectValue::Vec2(ax, ay), EffectValue::Vec2(bx, by)) => Some(EffectValue::Vec2(Self::lerp(*ax, *bx, t), Self::lerp(*ay, *by, t))),
            (EffectValue::Vec3(ax, ay, az), EffectValue::Vec3(bx, by, bz)) => Some(EffectValue::Vec3(Self::lerp(*ax, *bx, t), Self::lerp(*ay, *by, t), Self::lerp(*az, *bz, t))),
            (EffectValue::Color(ar, ag, ab, aa), EffectValue::Color(br, bg, bb, ba)) => Some(EffectValue::Color(Self::lerp(*ar, *br, t), Self::lerp(*ag, *bg, t), Self::lerp(*ab, *bb, t), Self::lerp(*aa, *ba, t))),
            (EffectValue::Int(a), EffectValue::Int(b)) => Some(EffectValue::Int((*a as f32 + (*b - *a) as f32 * t) as i64)),
            (EffectValue::Bool(a), EffectValue::Bool(b)) => Some(EffectValue::Bool(if t < 0.5 { *a } else { *b })),
            _ => None,
        }
    }

    fn apply_easing(t: f32, easing: Easing) -> f32 {
        match easing {
            Easing::None => t,
            Easing::EaseIn => t * t * t,
            Easing::EaseOut => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOut => if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 },
            Easing::Custom(x1, y1, x2, y2) => Self::cubic_bezier(t, x1, y1, x2, y2),
        }
    }

    fn cubic_bezier(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        let u = 1.0 - t;
        u * u * u * 0.0 + 3.0 * u * u * t * y1 + 3.0 * u * t * t * y2 + t * t * t * 1.0
    }

    fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
}

/// Video effect renderer
pub struct VideoEffectRenderer {
    renderer: GpuRenderer,
    texture_manager: TextureManager,
    pipelines: HashMap<EffectType, EffectPipeline>,
}

struct EffectPipeline {
    // Would contain pipeline, bind group layout, etc.
}

impl VideoEffectRenderer {
    pub fn new(renderer: GpuRenderer, texture_manager: TextureManager) -> Self {
        Self { renderer, texture_manager, pipelines: HashMap::new() }
    }

    /// Allocate a scratch texture for an intermediate effect pass.
    pub fn alloc_texture(&mut self, width: u32, height: u32) -> GpuTexture {
        self.texture_manager.create_render_target(width, height)
    }

    pub fn apply(&mut self, input: &GpuTexture, effect: &VideoEffect, output: &GpuTexture) -> Result<(), Box<dyn std::error::Error>> {
        if !effect.enabled || effect.mix <= 0.0 {
            return Ok(());
        }
        
        match effect.effect_type {
            EffectType::Transform => self.apply_transform(input, effect, output)?,
            EffectType::Crop => self.apply_crop(input, effect, output)?,
            EffectType::Opacity => self.apply_opacity(input, effect, output)?,
            EffectType::LiftGammaGain => self.apply_lift_gamma_gain(input, effect, output)?,
            EffectType::ContrastSaturation => self.apply_contrast_saturation(input, effect, output)?,
            EffectType::ColorWheels => self.apply_color_wheels(input, effect, output)?,
            EffectType::Lut3d => self.apply_lut3d(input, effect, output)?,
            EffectType::Stabilize => self.apply_stabilize(input, effect, output)?,
            EffectType::Speed => self.apply_speed(input, effect, output)?,
            _ => {}
        }
        
        Ok(())
    }

    fn apply_transform(&mut self, input: &GpuTexture, effect: &VideoEffect, output: &GpuTexture) -> Result<(), Box<dyn std::error::Error>> {
        // Would use compute shader for transform
        Ok(())
    }

    fn apply_crop(&mut self, input: &GpuTexture, effect: &VideoEffect, output: &GpuTexture) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn apply_opacity(&mut self, input: &GpuTexture, effect: &VideoEffect, output: &GpuTexture) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn apply_lift_gamma_gain(&mut self, input: &GpuTexture, effect: &VideoEffect, output: &GpuTexture) -> Result<(), Box<dyn std::error::Error>> {
        // Use color pipeline
        Ok(())
    }

    fn apply_contrast_saturation(&mut self, input: &GpuTexture, effect: &VideoEffect, output: &GpuTexture) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn apply_color_wheels(&mut self, input: &GpuTexture, effect: &VideoEffect, output: &GpuTexture) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn apply_lut3d(&mut self, input: &GpuTexture, effect: &VideoEffect, output: &GpuTexture) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn apply_stabilize(&mut self, input: &GpuTexture, effect: &VideoEffect, output: &GpuTexture) -> Result<(), Box<dyn std::error::Error>> {
        // Use compute pipeline
        self.renderer.submit(RenderCommand::Compute { 
            pipeline: ComputePipelineType::Stabilize, 
            params: ComputeParams { input: (*input).clone(), output: Some((*output).clone()), params: [0.0; 16] } 
        });
        Ok(())
    }

    fn apply_speed(&mut self, input: &GpuTexture, effect: &VideoEffect, output: &GpuTexture) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

/// Effect stack - manages multiple effects on a clip
pub struct EffectStack {
    effects: Vec<VideoEffect>,
    cache: Option<GpuTexture>,
}

impl EffectStack {
    pub fn new() -> Self {
        Self { effects: Vec::new(), cache: None }
    }

    pub fn add(&mut self, effect: VideoEffect) {
        self.effects.push(effect);
        self.cache = None;
    }

    pub fn remove(&mut self, index: usize) -> Option<VideoEffect> {
        if index < self.effects.len() {
            self.cache = None;
            Some(self.effects.remove(index))
        } else { None }
    }

    pub fn reorder(&mut self, from: usize, to: usize) {
        if from < self.effects.len() && to < self.effects.len() {
            let effect = self.effects.remove(from);
            self.effects.insert(to, effect);
            self.cache = None;
        }
    }

    pub fn get(&self, index: usize) -> Option<&VideoEffect> {
        self.effects.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut VideoEffect> {
        self.cache = None;
        self.effects.get_mut(index)
    }

    pub fn len(&self) -> usize { self.effects.len() }
    pub fn is_empty(&self) -> bool { self.effects.is_empty() }

    pub fn iter(&self) -> impl Iterator<Item = &VideoEffect> { self.effects.iter() }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut VideoEffect> { self.effects.iter_mut() }

    pub fn render(&mut self, input: &GpuTexture, output: &GpuTexture, renderer: &mut VideoEffectRenderer) -> Result<(), Box<dyn std::error::Error>> {
        if self.effects.is_empty() { return Ok(()); }

        let mut current_input = input.clone();

        for (i, effect) in self.effects.iter().enumerate() {
            let is_last = i == self.effects.len() - 1;
            if is_last {
                renderer.apply(&current_input, effect, output)?;
                break;
            }

            // Intermediate stages render into a scratch texture. The stack has no
            // texture manager of its own, so it allocates through the GPU renderer
            // the caller already owns instead of growing an unbounded pool.
            let scratch = renderer.alloc_texture(current_input.width, current_input.height);
            renderer.apply(&current_input, effect, &scratch)?;
            current_input = scratch;
        }

        Ok(())
    }
}