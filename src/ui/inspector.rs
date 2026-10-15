//! Inspector widget - clip, track, effect properties
use crate::core::math::{Rect, Point2};
use crate::core::types::EffectValue;

use crate::ui::widgets::*;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::timeline::clip::Timeline;
use crate::timeline::track::TrackItem;
use crate::effects::video::{VideoEffect, EffectStack};
use crate::effects::audio::AudioEffect;
use crate::core::id::{TrackItemId, ClipId, TrackId};
use crate::core::time::Frame;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct InspectorWidget {
    timeline: Option<Arc<RwLock<Timeline>>>,
    pool: Option<Arc<RwLock<crate::media::pool::MediaPool>>>,
    selected_item: Option<TrackItemId>,
    selected_clip: Option<ClipId>,
    selected_track: Option<TrackId>,
    rect: Rect,
}

impl InspectorWidget {
    pub fn new() -> Self {
        Self {
            timeline: None,
            pool: None,
            selected_item: None,
            selected_clip: None,
            selected_track: None,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline);
    }

    pub fn set_pool(&mut self, pool: Arc<RwLock<crate::media::pool::MediaPool>>) {
        self.pool = Some(pool);
    }

    pub fn select_item(&mut self, item_id: TrackItemId) {
        self.selected_item = Some(item_id);
        self.selected_clip = None;
        self.selected_track = None;
    }

    pub fn select_clip(&mut self, clip_id: ClipId) {
        self.selected_clip = Some(clip_id);
        self.selected_item = None;
        self.selected_track = None;
    }

    pub fn select_track(&mut self, track_id: TrackId) {
        self.selected_track = Some(track_id);
        self.selected_item = None;
        self.selected_clip = None;
    }
}

impl InspectorWidget {
    fn draw_item_inspector(&self, ctx: &DrawContext, rect: Rect, y: &mut f32, item_id: TrackItemId) {
        if let Some(timeline) = &self.timeline {
            if let Ok(tl) = timeline.read() {
                if let Some(item) = tl.get_item(item_id) {
                    // Header
                    self.draw_section_header(ctx, rect, y, "CLIP");
                    
                    // Name
                    self.draw_property(ctx, rect, y, "Name", &item.metadata.name);
                    
                    // Source timecode
                    let src_in_tc = crate::core::time::Timecode::from_frame(item.source_in(), crate::core::time::FrameRate::Fps24);
                    let src_out_tc = crate::core::time::Timecode::from_frame(item.source_out(), crate::core::time::FrameRate::Fps24);
                    self.draw_property(ctx, rect, y, "Src In", &src_in_tc.format());
                    self.draw_property(ctx, rect, y, "Src Out", &src_out_tc.format());
                    
                    // Timeline timecode
                    let tl_in_tc = crate::core::time::Timecode::from_frame(item.timeline_in(), crate::core::time::FrameRate::Fps24);
                    let tl_out_tc = crate::core::time::Timecode::from_frame(item.timeline_out(), crate::core::time::FrameRate::Fps24);
                    self.draw_property(ctx, rect, y, "TL In", &tl_in_tc.format());
                    self.draw_property(ctx, rect, y, "TL Out", &tl_out_tc.format());
                    
                    // Duration
                    let dur_tc = crate::core::time::Timecode::from_frame(item.timeline_duration(), crate::core::time::FrameRate::Fps24);
                    self.draw_property(ctx, rect, y, "Duration", &dur_tc.format());
                    
                    // Transform
                    *y += 10.0;
                    self.draw_section_header(ctx, rect, y, "TRANSFORM");
                    
                    if let Some(transform_effect) = item.effects.iter().find(|e| e.effect_type == crate::core::types::EffectType::Transform) {
                        if let Some(EffectValue::Vec2(x, y_pos)) = transform_effect.parameters.get("position") {
                            self.draw_property(ctx, rect, y, "Position", &format!("{:.1}, {:.1}", x, y_pos));
                        }
                        if let Some(EffectValue::Vec2(sx, sy)) = transform_effect.parameters.get("scale") {
                            self.draw_property(ctx, rect, y, "Scale", &format!("{:.2}, {:.2}", sx, sy));
                        }
                        if let Some(EffectValue::Float(rot)) = transform_effect.parameters.get("rotation") {
                            self.draw_property(ctx, rect, y, "Rotation", &format!("{:.1}°", rot));
                        }
                        if let Some(EffectValue::Vec2(ax, ay)) = transform_effect.parameters.get("anchor") {
                            self.draw_property(ctx, rect, y, "Anchor", &format!("{:.2}, {:.2}", ax, ay));
                        }
                    }
                    
                    // Opacity
                    self.draw_property(ctx, rect, y, "Opacity", &format!("{:.0}%", item.opacity * 100.0));
                    
                    // Blend mode
                    self.draw_property(ctx, rect, y, "Blend", &format!("{:?}", item.blend_mode));
                    
                    // Speed
                    self.draw_property(ctx, rect, y, "Speed", &format!("{:.2}x", item.speed));
                    
                    // Audio
                    *y += 10.0;
                    self.draw_section_header(ctx, rect, y, "AUDIO");
                    self.draw_property(ctx, rect, y, "Volume", &format!("{:.1}", item.volume));
                    self.draw_property(ctx, rect, y, "Pan", &format!("{:.1}", item.pan));
                    
                    // Effects
                    if !item.effects.is_empty() {
                        *y += 10.0;
                        self.draw_section_header(ctx, rect, y, "EFFECTS");
                        for (i, effect) in item.effects.iter().enumerate() {
                            let enabled_str = if effect.enabled { "✓" } else { "✗" };
                            self.draw_property(ctx, rect, y, &format!("{} {}", enabled_str, format!("{:?}", effect.effect_type)), "");
                        }
                    }
                }
            }
        }
    }

    fn draw_clip_inspector(&self, ctx: &DrawContext, rect: Rect, y: &mut f32, clip_id: ClipId) {
        if let Some(pool) = &self.pool {
            if let Ok(p) = pool.read() {
                if let Some(clip) = p.get_clip(clip_id) {
                    self.draw_section_header(ctx, rect, y, "CLIP PROPERTIES");
                    
                    self.draw_property(ctx, rect, y, "Name", &clip.metadata.tags.name);
                    self.draw_property(ctx, rect, y, "Path", &clip.metadata.file_path);
                    self.draw_property(ctx, rect, y, "Type", &format!("{:?}", clip.metadata.media_type));
                    
                    let tc = crate::core::time::Timecode::from_frame(clip.metadata.duration, crate::core::time::FrameRate::Fps24);
                    self.draw_property(ctx, rect, y, "Duration", &tc.format());
                    
                    if let Some(res) = clip.metadata.resolution {
                        self.draw_property(ctx, rect, y, "Resolution", &format!("{:?}", res.dimensions()));
                    }
                    
                    if let Some(fps) = clip.metadata.frame_rate {
                        self.draw_property(ctx, rect, y, "Frame Rate", &format!("{:.2}", fps.fps_f32()));
                    }
                    
                    if let Some(fmt) = clip.metadata.pixel_format {
                        self.draw_property(ctx, rect, y, "Pixel Format", &format!("{:?}", fmt));
                    }
                    
                    if let Some(cs) = clip.metadata.color_space {
                        self.draw_property(ctx, rect, y, "Color Space", &format!("{:?}", cs));
                    }
                    
                    if let Some(codec) = clip.metadata.codec_name {
                        self.draw_property(ctx, rect, y, "Codec", &codec);
                    }
                    
                    if clip.metadata.audio_channels.is_some() {
                        self.draw_property(ctx, rect, y, "Audio Channels", &clip.metadata.audio_channels.unwrap().to_string());
                    }
                    
                    if clip.metadata.audio_sample_rate.is_some() {
                        self.draw_property(ctx, rect, y, "Sample Rate", &format!("{} Hz", clip.metadata.audio_sample_rate.unwrap()));
                    }
                    
                    // Metadata
                    *y += 10.0;
                    self.draw_section_header(ctx, rect, y, "METADATA");
                    for (k, v) in &clip.metadata.tags.custom {
                        self.draw_property(ctx, rect, y, k, v);
                    }
                }
            }
        }
    }

    fn draw_track_inspector(&self, ctx: &DrawContext, rect: Rect, y: &mut f32, track_id: TrackId) {
        if let Some(timeline) = &self.timeline {
            if let Ok(tl) = timeline.read() {
                // The three track kinds are distinct types, so resolve them one at a
                // time instead of chaining or_else across different Option<&T>.
                let track_name = tl
                    .tracks
                    .get_video_track(track_id)
                    .map(|t| (t.name.clone(), t.enabled, t.locked, "Video"))
                    .or_else(|| {
                        tl.tracks
                            .get_audio_track(track_id)
                            .map(|t| (t.name.clone(), t.enabled, t.locked, "Audio"))
                    })
                    .or_else(|| {
                        tl.tracks
                            .get_subtitle_track(track_id)
                            .map(|t| (t.name.clone(), t.enabled, t.locked, "Subtitle"))
                    });
                if let Some((name, enabled, locked, kind)) = track_name {
                    self.draw_section_header(ctx, rect, y, "TRACK");
                    
                    self.draw_property(ctx, rect, y, "Name", &name);
                    self.draw_property(ctx, rect, y, "Type", kind);
                    self.draw_property(ctx, rect, y, "Enabled", &enabled.to_string());
                    self.draw_property(ctx, rect, y, "Locked", &locked.to_string());

                    if let Some(audio_track) = tl.tracks.get_audio_track(track_id) {
                        self.draw_property(ctx, rect, y, "Solo", &audio_track.solo.to_string());
                        self.draw_property(ctx, rect, y, "Volume", &audio_track.volume.to_string());
                        self.draw_property(ctx, rect, y, "Pan", &audio_track.pan.to_string());
                        self.draw_property(ctx, rect, y, "Muted", &audio_track.muted.to_string());
                    }
                }
            }
        }
    }

    fn draw_section_header(&self, ctx: &DrawContext, rect: Rect, y: &mut f32, title: &str) {
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
            color: ctx.theme.bg_tertiary,
            rect: [rect.x, *y, rect.w, 24.0],
        });
        ctx.text_renderer.draw_text(title, Point2::new(rect.x + 10.0, *y + 16.0), ctx.theme.font_size_sm, ctx.theme.accent);
        *y += 28.0;
    }

    fn draw_property(&self, ctx: &DrawContext, rect: Rect, y: &mut f32, label: &str, value: &str) {
        let label_w = 80.0;
        let row_rect = Rect::new(rect.x, *y, rect.w, 22.0);
        
        ctx.text_renderer.draw_text(label, Point2::new(rect.x + 10.0, *y + 16.0), ctx.theme.font_size_sm, ctx.theme.text_secondary);
        ctx.text_renderer.draw_text(value, Point2::new(rect.x + label_w + 10.0, *y + 16.0), ctx.theme.font_size_sm, ctx.theme.text_primary);
        
        *y += 22.0;
    }
}

impl Widget for InspectorWidget {

    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (300.0, f32::INFINITY)
    }

    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) {
        self.rect = rect;
    }

    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
            color: ctx.theme.bg_secondary,
            rect: [rect.x, rect.y, rect.w, rect.h],
        });

        let mut y = rect.y + 10.0;
        let label_w = 80.0;
        let value_w = rect.w - label_w - 20.0;

        if let Some(item_id) = self.selected_item {
            self.draw_item_inspector(ctx, rect, &mut y, item_id);
        } else if let Some(clip_id) = self.selected_clip {
            self.draw_clip_inspector(ctx, rect, &mut y, clip_id);
        } else if let Some(track_id) = self.selected_track {
            self.draw_track_inspector(ctx, rect, &mut y, track_id);
        } else {
            ctx.text_renderer.draw_text("No selection", Point2::new(rect.x + 10.0, y + 20.0), ctx.theme.font_size_md, ctx.theme.text_secondary);
        }

    }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        false
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Color page inspector
#[derive(Clone)]
pub struct ColorInspector {
    pub selected_node: Option<crate::core::id::GradeNodeId>,
    pub node_graph: Option<Arc<RwLock<crate::ui::node_graph::NodeGraph>>>,
    rect: Rect,
}

impl ColorInspector {
    pub fn new() -> Self {
        Self { selected_node: None, node_graph: None, rect: Rect::new(0.0, 0.0, 0.0, 0.0) }
    }
}

impl Widget for ColorInspector {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) { (300.0, f32::INFINITY) }
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) { self.rect = rect; }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.bg_secondary, rect: [rect.x, rect.y, rect.w, rect.h] });
        ctx.text_renderer.draw_text("Color Inspector", Point2::new(rect.x + 10.0, rect.y + 20.0), ctx.theme.font_size_md, ctx.theme.text_primary);
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Fusion page inspector
#[derive(Clone)]
pub struct FusionInspector {
    pub selected_tool: Option<String>,
    rect: Rect,
}

impl FusionInspector {
    pub fn new() -> Self { Self { selected_tool: None, rect: Rect::new(0.0, 0.0, 0.0, 0.0) } }
}

impl Widget for FusionInspector {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) { (300.0, f32::INFINITY) }
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) { self.rect = rect; }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.bg_secondary, rect: [rect.x, rect.y, rect.w, rect.h] });
        ctx.text_renderer.draw_text("Fusion Inspector", Point2::new(rect.x + 10.0, rect.y + 20.0), ctx.theme.font_size_md, ctx.theme.text_primary);
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Fairlight page inspector
#[derive(Clone)]
pub struct FairlightInspector {
    pub selected_track: Option<crate::core::id::TrackId>,
    rect: Rect,
}

impl FairlightInspector {
    pub fn new() -> Self { Self { selected_track: None, rect: Rect::new(0.0, 0.0, 0.0, 0.0) } }
}

impl Widget for FairlightInspector {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) { (350.0, f32::INFINITY) }
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) { self.rect = rect; }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.bg_secondary, rect: [rect.x, rect.y, rect.w, rect.h] });
        ctx.text_renderer.draw_text("Fairlight Inspector", Point2::new(rect.x + 10.0, rect.y + 20.0), ctx.theme.font_size_md, ctx.theme.text_primary);
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}