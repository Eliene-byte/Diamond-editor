//! Scopes widget - waveform, parade, vectorscope, histogram
use crate::core::math::{Rect, Point2};

use crate::ui::widgets::*;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::gpu::scopes::{ScopesRenderer, ScopeMode};
use crate::gpu::texture::GpuTexture;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct ScopesWidget {
    device: Option<Arc<wgpu::Device>>,
    scopes_renderer: Arc<parking_lot::RwLock<Option<Arc<ScopesRenderer>>>>,
    current_mode: ScopeMode,
    modes: Vec<ScopeMode>,
    input_texture: Option<GpuTexture>,
    output_textures: std::collections::HashMap<ScopeMode, GpuTexture>,
    gain: f32,
    rect: Rect,
    layout: ScopesLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopesLayout { Single, Dual, Quad, Horizontal, Vertical }

impl ScopesWidget {
    pub fn new() -> Self {
        Self {
            device: None,
            scopes_renderer: Arc::new(parking_lot::RwLock::new(None)),
            current_mode: ScopeMode::Waveform,
            modes: vec![ScopeMode::Waveform, ScopeMode::Parade, ScopeMode::Vectorscope, ScopeMode::Histogram],
            input_texture: None,
            output_textures: std::collections::HashMap::new(),
            gain: 1.0,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            layout: ScopesLayout::Horizontal,
        }
    }

    /// Attach the GPU device used to build the scope renderer and its output textures.
    /// Until this is called the widget stays inert (no GPU work, nothing drawn).
    pub fn set_device(&mut self, device: &Arc<wgpu::Device>, queue: &Arc<wgpu::Queue>) {
        self.device = Some(device.clone());
        *self.scopes_renderer.write() =
            Some(Arc::new(ScopesRenderer::new(device.clone(), queue.clone())));
        // Output textures belong to the previous device, drop them.
        self.output_textures.clear();
        self.render_scopes();
    }

    pub fn set_input(&mut self, texture: GpuTexture) {
        self.input_texture = Some(texture);
        self.render_scopes();
    }

    pub fn set_mode(&mut self, mode: ScopeMode) {
        self.current_mode = mode;
        self.render_scopes();
    }

    pub fn set_layout(&mut self, layout: ScopesLayout) {
        self.layout = layout;
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.1, 10.0);
        self.render_scopes();
    }

    fn render_scopes(&mut self) {
        let input = match &self.input_texture { Some(t) => t.clone(), None => return };
        let device = match &self.device { Some(d) => d.clone(), None => return };
        let renderer = match self.scopes_renderer.read().clone() { Some(r) => r, None => return };
        let gain = self.gain;
        let modes = self.modes.clone();

        for mode in &modes {
            let output = self.output_textures.entry(*mode).or_insert_with(|| {
                GpuTexture::new(
                    &device,
                    256, 256,
                    wgpu::TextureFormat::Rgba8Unorm,
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    Some(&format!("Scope {:?}", mode)),
                )
            }).clone();

            // `ScopesRenderer` mutates its cached scope images, so drive it through
            // the unique borrow held by the local `Arc` clone.
            let mut renderer = Arc::clone(&renderer);
            Arc::get_mut(&mut renderer).map(|r| {
                match mode {
                    ScopeMode::Waveform => { r.render_waveform(&input, &output, gain).ok(); }
                    ScopeMode::Parade => { r.render_parade(&input, &output, gain).ok(); }
                    ScopeMode::Vectorscope => { r.render_vectorscope(&input, &output, gain).ok(); }
                    ScopeMode::Histogram => { r.render_histogram(&input, &output, gain).ok(); }
                }
            });
        }
    }
}

impl Widget for ScopesWidget {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        match self.layout {
            ScopesLayout::Single => (256.0, 256.0),
            ScopesLayout::Dual => (512.0, 256.0),
            ScopesLayout::Quad => (512.0, 512.0),
            ScopesLayout::Horizontal => (256.0 * self.modes.len() as f32, 256.0),
            ScopesLayout::Vertical => (256.0, 256.0 * self.modes.len() as f32),
        }
    }

    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) {
        self.rect = rect;
    }

    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        // Background
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
            color: ctx.theme.bg_primary,
            rect: [rect.x, rect.y, rect.w, rect.h],
        });

        // Mode tabs
        let tab_height = 30.0;
        let tab_rect = Rect::new(rect.x, rect.y, rect.w, tab_height);
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
            color: ctx.theme.bg_secondary,
            rect: [tab_rect.x, tab_rect.y, tab_rect.w, tab_rect.h],
        });

        let tab_w = rect.w / self.modes.len() as f32;
        for (i, mode) in self.modes.iter().enumerate() {
            let active = *mode == self.current_mode;
            let x = rect.x + i as f32 * tab_w;
            let m_rect = Rect::new(x, rect.y, tab_w, tab_height);
            
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: if active { ctx.theme.accent } else { ctx.theme.bg_secondary },
                rect: [m_rect.x, m_rect.y, m_rect.w, m_rect.h],
            });
            
            let label = match mode {
                ScopeMode::Waveform => "Waveform",
                ScopeMode::Parade => "Parade",
                ScopeMode::Vectorscope => "Vectorscope",
                ScopeMode::Histogram => "Histogram",
            };
            ctx.text_renderer.draw_text(label, Point2::new(x + tab_w * 0.5, rect.y + 20.0), ctx.theme.font_size_sm, ctx.theme.text_primary);
        }

        // Scope display
        let scope_rect = Rect::new(rect.x, rect.y + tab_height, rect.w, rect.h - tab_height);
        
        match self.layout {
            ScopesLayout::Single => {
                if let Some(tex) = self.output_textures.get(&self.current_mode) {
                    ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawTexture {
                        texture: tex.clone(),
                        dest_rect: [scope_rect.x, scope_rect.y, scope_rect.w, scope_rect.h],
                        src_rect: [0.0, 0.0, 1.0, 1.0],
                        opacity: 1.0,
                        blend_mode: crate::gpu::renderer::BlendMode::Normal,
                    });
                }
            }
            ScopesLayout::Horizontal => {
                let w = scope_rect.w / self.modes.len() as f32;
                for (i, mode) in self.modes.iter().enumerate() {
                    if let Some(tex) = self.output_textures.get(mode) {
                        let x = scope_rect.x + i as f32 * w;
                        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawTexture {
                            texture: tex.clone(),
                            dest_rect: [x, scope_rect.y, w, scope_rect.h],
                            src_rect: [0.0, 0.0, 1.0, 1.0],
                            opacity: 1.0,
                            blend_mode: crate::gpu::renderer::BlendMode::Normal,
                        });
                    }
                }
            }
            ScopesLayout::Vertical => {
                let h = scope_rect.h / self.modes.len() as f32;
                for (i, mode) in self.modes.iter().enumerate() {
                    if let Some(tex) = self.output_textures.get(mode) {
                        let y = scope_rect.y + i as f32 * h;
                        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawTexture {
                            texture: tex.clone(),
                            dest_rect: [scope_rect.x, y, scope_rect.w, h],
                            src_rect: [0.0, 0.0, 1.0, 1.0],
                            opacity: 1.0,
                            blend_mode: crate::gpu::renderer::BlendMode::Normal,
                        });
                    }
                }
            }
            ScopesLayout::Quad => {
                let w = scope_rect.w / 2.0;
                let h = scope_rect.h / 2.0;
                for (i, mode) in self.modes.iter().enumerate() {
                    if let Some(tex) = self.output_textures.get(mode) {
                        let col = i % 2;
                        let row = i / 2;
                        let x = scope_rect.x + col as f32 * w;
                        let y = scope_rect.y + row as f32 * h;
                        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawTexture {
                            texture: tex.clone(),
                            dest_rect: [x, y, w, h],
                            src_rect: [0.0, 0.0, 1.0, 1.0],
                            opacity: 1.0,
                            blend_mode: crate::gpu::renderer::BlendMode::Normal,
                        });
                    }
                }
            }
            ScopesLayout::Dual => {
                // Two panes: the selected mode on top, the next mode below it.
                let h = scope_rect.h / 2.0;
                for (row, mode) in self.modes.iter().take(2).enumerate() {
                    if let Some(tex) = self.output_textures.get(mode) {
                        let y = scope_rect.y + row as f32 * h;
                        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawTexture {
                            texture: tex.clone(),
                            dest_rect: [scope_rect.x, y, scope_rect.w, h],
                            src_rect: [0.0, 0.0, 1.0, 1.0],
                            opacity: 1.0,
                            blend_mode: crate::gpu::renderer::BlendMode::Normal,
                        });
                    }
                }
            }
        }

        // Gain control
        let gain_rect = Rect::new(rect.x + 10.0, rect.y + rect.h - 40.0, 200.0, 30.0);
        ctx.text_renderer.draw_text(&format!("Gain: {:.1}x", self.gain), Point2::new(gain_rect.x, gain_rect.y + 20.0), ctx.theme.font_size_sm, ctx.theme.text_secondary);
    }

    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        match event {
            InputEvent::Click(pos) => {
                // Check mode tabs
                let tab_height = 30.0;
                let tab_w = self.rect.w / self.modes.len() as f32;
                if pos.y >= self.rect.y && pos.y < self.rect.y + tab_height {
                    let index = ((pos.x - self.rect.x) / tab_w) as usize;
                    if index < self.modes.len() {
                        self.set_mode(self.modes[index]);
                        return true;
                    }
                }
                false
            }
            InputEvent::Scroll(dx, dy) => {
                if dx != 0.0 {
                    self.set_gain(self.gain * (1.0 + dx * 0.1));
                    true
                } else { false }
            }
            _ => false
        }
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}