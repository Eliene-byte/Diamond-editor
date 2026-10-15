//! Viewer widget - source, timeline, color, fusion, fairlight, deliver viewers
use crate::core::math::{Point2, Rect};

use crate::core::time::{Frame, FrameRate, Timecode};
use crate::gpu::texture::GpuTexture;
use crate::timeline::clip::Timeline;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::ui::widgets::*;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerMode {
    Source,
    Timeline,
    Color,
    Fusion,
    Fairlight,
    Deliver,
}

#[derive(Clone)]
pub struct ViewerWidget {
    mode: ViewerMode,
    timeline: Option<Arc<RwLock<Timeline>>>,
    current_frame: Frame,
    current_timecode: Timecode,
    frame_rate: FrameRate,
    zoom: f32,
    pan: (f32, f32),
    show_overlays: bool,
    show_safe_areas: bool,
    show_grid: bool,
    rect: Rect,
    texture: Option<GpuTexture>,
    playback_state: crate::core::types::PlaybackState,
}

impl ViewerWidget {
    pub fn new(mode: ViewerMode) -> Self {
        Self {
            mode,
            timeline: None,
            current_frame: Frame::ZERO,
            current_timecode: Timecode::zero(FrameRate::Fps24),
            frame_rate: FrameRate::Fps24,
            zoom: 1.0,
            pan: (0.0, 0.0),
            show_overlays: true,
            show_safe_areas: true,
            show_grid: false,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            texture: None,
            playback_state: crate::core::types::PlaybackState::Stopped,
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline.clone());
        if let Ok(tl) = timeline.read() {
            self.frame_rate = tl.settings.frame_rate;
            self.current_timecode = tl.settings.start_timecode;
        }
    }

    pub fn set_frame(&mut self, frame: Frame) {
        self.current_frame = frame;
        self.current_timecode = Timecode::from_frame(frame, self.frame_rate);
    }

    pub fn set_texture(&mut self, texture: GpuTexture) {
        self.texture = Some(texture);
    }

    pub fn set_playback_state(&mut self, state: crate::core::types::PlaybackState) {
        self.playback_state = state;
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.1, 10.0);
    }

    pub fn set_pan(&mut self, x: f32, y: f32) {
        self.pan = (x, y);
    }

    pub fn toggle_overlays(&mut self) {
        self.show_overlays = !self.show_overlays;
    }

    pub fn toggle_safe_areas(&mut self) {
        self.show_safe_areas = !self.show_safe_areas;
    }
}

impl ViewerWidget {
    fn draw_overlays(&self, ctx: &DrawContext, rect: Rect) {
        if self.show_safe_areas {
            // Action safe (90%)
            let safe_w = rect.w * 0.9;
            let safe_h = rect.h * 0.9;
            let safe_x = rect.x + (rect.w - safe_w) * 0.5;
            let safe_y = rect.y + (rect.h - safe_h) * 0.5;
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: [1.0, 1.0, 1.0, 0.3],
                    rect: [safe_x, safe_y, safe_w, 1.0],
                });
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: [1.0, 1.0, 1.0, 0.3],
                    rect: [safe_x, safe_y + safe_h - 1.0, safe_w, 1.0],
                });
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: [1.0, 1.0, 1.0, 0.3],
                    rect: [safe_x, safe_y, 1.0, safe_h],
                });
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: [1.0, 1.0, 1.0, 0.3],
                    rect: [safe_x + safe_w - 1.0, safe_y, 1.0, safe_h],
                });

            // Title safe (80%)
            let title_w = rect.w * 0.8;
            let title_h = rect.h * 0.8;
            let title_x = rect.x + (rect.w - title_w) * 0.5;
            let title_y = rect.y + (rect.h - title_h) * 0.5;
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: [0.0, 1.0, 0.0, 0.3],
                    rect: [title_x, title_y, title_w, 1.0],
                });
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: [0.0, 1.0, 0.0, 0.3],
                    rect: [title_x, title_y + title_h - 1.0, title_w, 1.0],
                });
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: [0.0, 1.0, 0.0, 0.3],
                    rect: [title_x, title_y, 1.0, title_h],
                });
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: [0.0, 1.0, 0.0, 0.3],
                    rect: [title_x + title_w - 1.0, title_y, 1.0, title_h],
                });
        }
    }

    fn draw_timecode(&self, ctx: &DrawContext, rect: Rect) {
        let tc_text = self.current_timecode.format();
        ctx.text_renderer.draw_text(
            &tc_text,
            Point2::new(rect.x + 10.0, rect.y + 20.0),
            ctx.theme.font_size_md,
            ctx.theme.text_primary,
        );

        // Frame number
        let frame_text = format!("{}f", self.current_frame.0);
        ctx.text_renderer.draw_text(
            &frame_text,
            Point2::new(rect.x + rect.w - 100.0, rect.y + 20.0),
            ctx.theme.font_size_sm,
            ctx.theme.text_secondary,
        );
    }
}

impl Widget for ViewerWidget {
    fn measure(&self, ctx: &LayoutContext, constraints: Constraints) -> (f32, f32) {
        let w = constraints.max_width.min(1920.0).max(320.0);
        let h = w * 9.0 / 16.0;
        (w, h)
    }

    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) {
        self.rect = rect;
    }

    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        // Background
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_primary,
                rect: [rect.x, rect.y, rect.w, rect.h],
            });

        // Draw video texture if available
        if let Some(texture) = &self.texture {
            let (tw, th) = (texture.width as f32, texture.height as f32);
            let scale = (rect.w / tw).min(rect.h / th) * self.zoom;
            let dw = tw * scale;
            let dh = th * scale;
            let dx = rect.x + (rect.w - dw) * 0.5 + self.pan.0;
            let dy = rect.y + (rect.h - dh) * 0.5 + self.pan.1;
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawTexture {
                    texture: texture.clone(),
                    dest_rect: [dx, dy, dw, dh],
                    src_rect: [0.0, 0.0, 1.0, 1.0],
                    opacity: 1.0,
                    blend_mode: crate::gpu::renderer::BlendMode::Normal,
                });
        } else {
            ctx.text_renderer.draw_text(
                "No media loaded",
                Point2::new(rect.x + 10.0, rect.y + 10.0),
                ctx.theme.font_size_sm,
                ctx.theme.text_secondary,
            );
        }

        // Overlays
        if self.show_overlays {
            self.draw_overlays(ctx, rect);
        }

        // Timecode display
        self.draw_timecode(ctx, rect);
    }

    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        match event {
            InputEvent::MouseMove(pos) => {
                if pos.x >= self.rect.x
                    && pos.x < self.rect.x + self.rect.w
                    && pos.y >= self.rect.y
                    && pos.y < self.rect.y + self.rect.h
                {
                    // Hover - show transport controls
                }
                false
            }
            InputEvent::MouseDown(pos, btn) => {
                if pos.x >= self.rect.x
                    && pos.x < self.rect.x + self.rect.w
                    && pos.y >= self.rect.y
                    && pos.y < self.rect.y + self.rect.h
                {
                    match btn {
                        MouseButton::Left => {
                            // Set playhead position. Read the duration out of the
                            // guard before mutating self, so the two borrows do
                            // not overlap.
                            let duration = self
                                .timeline
                                .as_ref()
                                .and_then(|t| t.read().ok())
                                .map(|tl| tl.duration);
                            if let Some(duration) = duration {
                                let rel_x = (pos.x - self.rect.x) / self.rect.w;
                                let frame = Frame((rel_x * duration.0 as f32) as i64);
                                self.set_frame(frame);
                            }
                            true
                        }
                        MouseButton::Right => {
                            // Context menu
                            true
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            InputEvent::Scroll(dx, dy) => {
                if dy != 0.0 {
                    self.set_zoom(self.zoom * (1.0 + dy * 0.1));
                    true
                } else {
                    false
                }
            }
            InputEvent::KeyDown(key) => match key {
                Key::Space => {
                    self.playback_state = match self.playback_state {
                        crate::core::types::PlaybackState::Playing => {
                            crate::core::types::PlaybackState::Paused
                        }
                        _ => crate::core::types::PlaybackState::Playing,
                    };
                    true
                }
                Key::Left => {
                    self.step_frame(-1);
                    true
                }
                Key::Right => {
                    self.step_frame(1);
                    true
                }
                Key::Up => {
                    self.step_frame(-10);
                    true
                }
                Key::Down => {
                    self.step_frame(10);
                    true
                }
                Key::Home => {
                    self.set_frame(Frame::ZERO);
                    true
                }
                Key::End => {
                    let duration = self
                        .timeline
                        .as_ref()
                        .and_then(|t| t.read().ok())
                        .map(|tl| tl.duration);
                    if let Some(duration) = duration {
                        self.set_frame(duration);
                    }
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ViewerWidget {
    fn step_frame(&mut self, delta: i64) {
        let new_frame = Frame((self.current_frame.0 + delta).max(0));
        self.set_frame(new_frame);
    }
}
