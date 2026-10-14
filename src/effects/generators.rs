//! Built-in generators - solids, text, bars, noise, etc.

use crate::core::types::{EffectType, EffectParams, EffectValue};
use crate::core::time::{Frame, FrameRate};
use crate::gpu::texture::{TextureManager, GpuTexture};
use crate::gpu::renderer::RenderCommand;
use std::collections::HashMap;

/// Generator types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratorType {
    SolidColor,
    Gradient,
    BarsAndTone,
    Timecode,
    Text,
    Noise,
    Plasma,
    Checkerboard,
    ColorBars,
    SMPTEBars,
    Custom,
}

/// Generator instance
#[derive(Debug, Clone)]
pub struct Generator {
    pub generator_type: GeneratorType,
    pub params: EffectParams,
    pub duration: Frame,
    pub frame_rate: FrameRate,
}

impl Generator {
    pub fn new(generator_type: GeneratorType, duration: Frame, frame_rate: FrameRate) -> Self {
        Self {
            generator_type,
            params: Self::default_params(generator_type),
            duration,
            frame_rate,
        }
    }

    fn default_params(generator_type: GeneratorType) -> EffectParams {
        let mut params = HashMap::new();
        
        match generator_type {
            GeneratorType::SolidColor => {
                params.insert("color".into(), EffectValue::Color(0.5, 0.5, 0.5, 1.0));
            }
            GeneratorType::Gradient => {
                params.insert("start_color".into(), EffectValue::Color(0.0, 0.0, 0.0, 1.0));
                params.insert("end_color".into(), EffectValue::Color(1.0, 1.0, 1.0, 1.0));
                params.insert("direction".into(), EffectValue::Int(0)); // Horizontal, Vertical, Diagonal, Radial
            }
            GeneratorType::BarsAndTone => {
                params.insert("frequency".into(), EffectValue::Float(1000.0));
                params.insert("level".into(), EffectValue::Float(-20.0)); // dBFS
                params.insert("bars_type".into(), EffectValue::Int(0)); // SMPTE, EBU, Custom
            }
            GeneratorType::Timecode => {
                params.insert("format".into(), EffectValue::Int(0)); // HH:MM:SS:FF, HH:MM:SS;FF, Frames
                params.insert("font_size".into(), EffectValue::Float(72.0));
                params.insert("color".into(), EffectValue::Color(1.0, 1.0, 1.0, 1.0));
                params.insert("background".into(), EffectValue::Color(0.0, 0.0, 0.0, 0.5));
                params.insert("position".into(), EffectValue::Int(0)); // Top, Bottom, Center
            }
            GeneratorType::Text => {
                params.insert("text".into(), EffectValue::String("Text".into()));
                params.insert("font".into(), EffectValue::String("Inter".into()));
                params.insert("size".into(), EffectValue::Float(72.0));
                params.insert("color".into(), EffectValue::Color(1.0, 1.0, 1.0, 1.0));
                params.insert("alignment".into(), EffectValue::Int(0)); // Left, Center, Right
                params.insert("position".into(), EffectValue::Vec2(0.5, 0.5));
                params.insert("tracking".into(), EffectValue::Float(0.0));
                params.insert("leading".into(), EffectValue::Float(1.2));
            }
            GeneratorType::Noise => {
                params.insert("type".into(), EffectValue::Int(0)); // Uniform, Gaussian, Perlin
                params.insert("seed".into(), EffectValue::Int(0));
                params.insert("scale".into(), EffectValue::Float(1.0));
                params.insert("octaves".into(), EffectValue::Int(4));
                params.insert("persistence".into(), EffectValue::Float(0.5));
            }
            GeneratorType::Plasma => {
                params.insert("speed".into(), EffectValue::Float(1.0));
                params.insert("scale".into(), EffectValue::Float(1.0));
                params.insert("colors".into(), EffectValue::Int(0)); // Rainbow, Fire, Ice, Custom
            }
            GeneratorType::Checkerboard => {
                params.insert("color1".into(), EffectValue::Color(0.0, 0.0, 0.0, 1.0));
                params.insert("color2".into(), EffectValue::Color(1.0, 1.0, 1.0, 1.0));
                params.insert("size".into(), EffectValue::Float(50.0));
            }
            _ => {}
        }
        
        EffectParams { effect_type: EffectType::SolidColor, parameters: params }
    }

    pub fn render_frame(&self, texture_manager: &mut TextureManager, frame: Frame, width: u32, height: u32) -> GpuTexture {
        match self.generator_type {
            GeneratorType::SolidColor => self.render_solid(texture_manager, width, height),
            GeneratorType::Gradient => self.render_gradient(texture_manager, width, height),
            GeneratorType::BarsAndTone => self.render_bars(texture_manager, frame, width, height),
            GeneratorType::Timecode => self.render_timecode(texture_manager, frame, width, height),
            GeneratorType::Text => self.render_text(texture_manager, width, height),
            GeneratorType::Noise => self.render_noise(texture_manager, frame, width, height),
            GeneratorType::Plasma => self.render_plasma(texture_manager, frame, width, height),
            GeneratorType::Checkerboard => self.render_checkerboard(texture_manager, width, height),
            _ => self.render_solid(texture_manager, width, height),
        }
    }

    fn render_solid(&self, texture_manager: &mut TextureManager, width: u32, height: u32) -> GpuTexture {
        let color = self.params.parameters.get("color")
            .and_then(|v| match v { EffectValue::Color(r, g, b, a) => Some((*r, *g, *b, *a)), _ => None })
            .unwrap_or((0.5, 0.5, 0.5, 1.0));
        
        let data = vec![
            (color.0 * 255.0) as u8,
            (color.1 * 255.0) as u8,
            (color.2 * 255.0) as u8,
            (color.3 * 255.0) as u8,
        ].repeat((width * height) as usize);
        
        texture_manager.upload(&data, width, height, crate::core::types::PixelFormat::Rgba8)
    }

    fn render_gradient(&self, texture_manager: &mut TextureManager, width: u32, height: u32) -> GpuTexture {
        let start = self.params.parameters.get("start_color")
            .and_then(|v| match v { EffectValue::Color(r, g, b, a) => Some((*r, *g, *b, *a)), _ => None })
            .unwrap_or((0.0, 0.0, 0.0, 1.0));
        let end = self.params.parameters.get("end_color")
            .and_then(|v| match v { EffectValue::Color(r, g, b, a) => Some((*r, *g, *b, *a)), _ => None })
            .unwrap_or((1.0, 1.0, 1.0, 1.0));
        let direction = self.params.parameters.get("direction")
            .and_then(|v| match v { EffectValue::Int(i) => Some(*i), _ => None })
            .unwrap_or(0);
        
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        
        for y in 0..height {
            for x in 0..width {
                let t = match direction {
                    0 => x as f32 / (width - 1) as f32,           // Horizontal
                    1 => y as f32 / (height - 1) as f32,          // Vertical
                    2 => (x as f32 + y as f32) / (width + height - 2) as f32, // Diagonal
                    3 => {
                        let cx = (width / 2) as f32;
                        let cy = (height / 2) as f32;
                        let dx = x as f32 - cx;
                        let dy = y as f32 - cy;
                        let dist = (dx * dx + dy * dy).sqrt();
                        let max_dist = (cx * cx + cy * cy).sqrt();
                        (dist / max_dist).min(1.0)
                    }, // Radial
                    _ => 0.0,
                };
                
                let r = start.0 + (end.0 - start.0) * t;
                let g = start.1 + (end.1 - start.1) * t;
                let b = start.2 + (end.2 - start.2) * t;
                let a = start.3 + (end.3 - start.3) * t;
                
                data.push((r * 255.0) as u8);
                data.push((g * 255.0) as u8);
                data.push((b * 255.0) as u8);
                data.push((a * 255.0) as u8);
            }
        }
        
        texture_manager.upload(&data, width, height, crate::core::types::PixelFormat::Rgba8)
    }

    fn render_bars(&self, texture_manager: &mut TextureManager, frame: Frame, width: u32, height: u32) -> GpuTexture {
        // Simplified SMPTE color bars
        let mut data = vec![0u8; (width * height * 4) as usize];
        let bar_width = width / 7;
        
        let colors = [
            (1.0, 1.0, 1.0, 1.0), // White
            (1.0, 1.0, 0.0, 1.0), // Yellow
            (0.0, 1.0, 1.0, 1.0), // Cyan
            (0.0, 1.0, 0.0, 1.0), // Green
            (1.0, 0.0, 1.0, 1.0), // Magenta
            (1.0, 0.0, 0.0, 1.0), // Red
            (0.0, 0.0, 1.0, 1.0), // Blue
        ];
        
        for y in 0..height {
            for x in 0..width {
                let bar = (x / bar_width).min(6);
                let color = colors[bar as usize];
                let idx = ((y * width + x) * 4) as usize;
                data[idx] = (color.0 * 255.0) as u8;
                data[idx + 1] = (color.1 * 255.0) as u8;
                data[idx + 2] = (color.2 * 255.0) as u8;
                data[idx + 3] = (color.3 * 255.0) as u8;
            }
        }
        
        texture_manager.upload(&data, width, height, crate::core::types::PixelFormat::Rgba8)
    }

    fn render_timecode(&self, texture_manager: &mut TextureManager, frame: Frame, width: u32, height: u32) -> GpuTexture {
        // Would render timecode text - simplified to solid
        self.render_solid(texture_manager, width, height)
    }

    fn render_text(&self, texture_manager: &mut TextureManager, width: u32, height: u32) -> GpuTexture {
        // Would use GPU text rendering - simplified
        self.render_solid(texture_manager, width, height)
    }

    fn render_noise(&self, texture_manager: &mut TextureManager, frame: Frame, width: u32, height: u32) -> GpuTexture {
        use rand::{Rng, SeedableRng};
        use rand_chacha::ChaCha8Rng;
        
        let seed = self.params.parameters.get("seed")
            .and_then(|v| match v { EffectValue::Int(i) => Some(*i as u64), _ => None })
            .unwrap_or(0) ^ frame.0 as u64;
        
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let scale = self.params.parameters.get("scale")
            .and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None })
            .unwrap_or(1.0);
        
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        
        for _ in 0..(width * height) {
            let v = rng.random::<f32>() * scale;
            let val = (v.clamp(0.0, 1.0) * 255.0) as u8;
            data.push(val);
            data.push(val);
            data.push(val);
            data.push(255);
        }
        
        texture_manager.upload(&data, width, height, crate::core::types::PixelFormat::Rgba8)
    }

    fn render_plasma(&self, texture_manager: &mut TextureManager, frame: Frame, width: u32, height: u32) -> GpuTexture {
        let time = frame.0 as f32 / self.frame_rate.fps_f32();
        let speed = self.params.parameters.get("speed")
            .and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None })
            .unwrap_or(1.0);
        let scale = self.params.parameters.get("scale")
            .and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None })
            .unwrap_or(1.0);
        
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        
        for y in 0..height {
            for x in 0..width {
                let fx = x as f32 / width as f32 * scale;
                let fy = y as f32 / height as f32 * scale;
                
                let v1 = (fx * 10.0 + time * speed).sin();
                let v2 = (fy * 10.0 + time * speed * 0.7).sin();
                let v3 = ((fx + fy) * 10.0 + time * speed * 1.3).sin();
                let v4 = ((fx * fx + fy * fy).sqrt() * 10.0 + time * speed * 0.5).sin();
                
                let v = (v1 + v2 + v3 + v4) * 0.25 * 0.5 + 0.5;
                
                // Colorize
                let hue = v * 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 1.0, v);
                
                data.push((r * 255.0) as u8);
                data.push((g * 255.0) as u8);
                data.push((b * 255.0) as u8);
                data.push(255);
            }
        }
        
        texture_manager.upload(&data, width, height, crate::core::types::PixelFormat::Rgba8)
    }

    fn render_checkerboard(&self, texture_manager: &mut TextureManager, width: u32, height: u32) -> GpuTexture {
        let color1 = self.params.parameters.get("color1")
            .and_then(|v| match v { EffectValue::Color(r, g, b, a) => Some((*r, *g, *b, *a)), _ => None })
            .unwrap_or((0.0, 0.0, 0.0, 1.0));
        let color2 = self.params.parameters.get("color2")
            .and_then(|v| match v { EffectValue::Color(r, g, b, a) => Some((*r, *g, *b, *a)), _ => None })
            .unwrap_or((1.0, 1.0, 1.0, 1.0));
        let size = self.params.parameters.get("size")
            .and_then(|v| match v { EffectValue::Float(f) => Some(*f as u32), _ => None })
            .unwrap_or(50);
        
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        
        for y in 0..height {
            for x in 0..width {
                let square_x = x / size;
                let square_y = y / size;
                let color = if (square_x + square_y) % 2 == 0 { color1 } else { color2 };
                
                data.push((color.0 * 255.0) as u8);
                data.push((color.1 * 255.0) as u8);
                data.push((color.2 * 255.0) as u8);
                data.push((color.3 * 255.0) as u8);
            }
        }
        
        texture_manager.upload(&data, width, height, crate::core::types::PixelFormat::Rgba8)
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    
    let (r, g, b) = if h < 60.0 { (c, x, 0.0) }
    else if h < 120.0 { (x, c, 0.0) }
    else if h < 180.0 { (0.0, c, x) }
    else if h < 240.0 { (0.0, x, c) }
    else if h < 300.0 { (x, 0.0, c) }
    else { (c, 0.0, x) };
    
    (r + m, g + m, b + m)
}

/// Generator registry
pub struct GeneratorRegistry {
    generators: HashMap<GeneratorType, Generator>,
}

impl GeneratorRegistry {
    pub fn new() -> Self {
        let mut registry = Self { generators: HashMap::new() };
        
        for gt in [
            GeneratorType::SolidColor,
            GeneratorType::Gradient,
            GeneratorType::BarsAndTone,
            GeneratorType::Timecode,
            GeneratorType::Text,
            GeneratorType::Noise,
            GeneratorType::Plasma,
            GeneratorType::Checkerboard,
        ] {
            registry.generators.insert(gt, Generator::new(gt, Frame(24), FrameRate::Fps24));
        }
        
        registry
    }

    pub fn get(&self, generator_type: GeneratorType) -> Option<&Generator> {
        self.generators.get(&generator_type)
    }

    pub fn get_mut(&mut self, generator_type: GeneratorType) -> Option<&mut Generator> {
        self.generators.get_mut(&generator_type)
    }

    pub fn list(&self) -> Vec<GeneratorType> {
        self.generators.keys().copied().collect()
    }
}