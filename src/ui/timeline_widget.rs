//! Timeline widget - professional timeline editing
use crate::core::math::{Rect, Point2};

use crate::ui::widgets::*;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::timeline::clip::Timeline;
use crate::timeline::clip::TrackItems;
use crate::timeline::track::{AudioTrack, SubtitleTrack, TrackItem, VideoTrack};
use crate::core::time::{Frame, FrameRange, Timecode, FrameRate};
use crate::core::types::{TrackType, MediaType};
use crate::core::id::{TrackId, TrackItemId};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct TimelineWidget {
    timeline: Option<Arc<RwLock<Timeline>>>,
    playhead: Frame,
    zoom: f32, // pixels per frame
    scroll_x: f32,
    scroll_y: f32,
    track_heights: std::collections::HashMap<TrackId, f32>,
    selected_items: Vec<TrackItemId>,
    hover_item: Option<TrackItemId>,
    drag_state: Option<TimelineDragState>,
    rect: Rect,
    time_ruler_height: f32,
}

#[derive(Debug, Clone)]
struct TimelineDragState {
    item_id: TrackItemId,
    start_frame: Frame,
    start_mouse_x: f32,
    drag_type: DragType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragType {
    Move,
    TrimIn,
    TrimOut,
    Slip,
    Slide,
}

impl TimelineWidget {
    pub fn new() -> Self {
        Self {
            timeline: None,
            playhead: Frame::ZERO,
            zoom: 50.0, // 50 pixels per frame at 24fps = 2px/frame
            scroll_x: 0.0,
            scroll_y: 0.0,
            track_heights: std::collections::HashMap::new(),
            selected_items: Vec::new(),
            hover_item: None,
            drag_state: None,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            time_ruler_height: 30.0,
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline);
    }

    pub fn set_playhead(&mut self, frame: Frame) {
        self.playhead = frame;
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(1.0, 500.0);
    }

    pub fn scroll_to(&mut self, x: f32, y: f32) {
        self.scroll_x = x.max(0.0);
        self.scroll_y = y.max(0.0);
    }

    pub fn select_item(&mut self, item_id: TrackItemId, add: bool) {
        if !add { self.selected_items.clear(); }
        if !self.selected_items.contains(&item_id) {
            self.selected_items.push(item_id);
        }
    }

    pub fn frame_to_x(&self, frame: Frame) -> f32 {
        (frame.0 as f32 * self.zoom) - self.scroll_x
    }

    pub fn x_to_frame(&self, x: f32) -> Frame {
        Frame(((x + self.scroll_x) / self.zoom).round() as i64)
    }

    pub fn track_y(&self, track_index: usize, is_audio: bool) -> f32 {
        self.time_ruler_height + track_index as f32 * (if is_audio { 40.0 } else { 60.0 }) - self.scroll_y
    }
}

impl TimelineWidget {
    fn draw_time_ruler(&self, ctx: &DrawContext, rect: Rect) {
        let ruler_rect = Rect::new(rect.x, rect.y, rect.w, self.time_ruler_height);
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
            color: ctx.theme.bg_secondary,
            rect: [ruler_rect.x, ruler_rect.y, ruler_rect.w, ruler_rect.h],
        });

        // Draw timecode markers
        let major_interval = (self.zoom * 24.0).max(50.0); // Every second at 24fps
        let start_frame = self.x_to_frame(rect.x);
        let end_frame = self.x_to_frame(rect.x + rect.w);
        
        let mut frame = start_frame.0 - (start_frame.0 % (major_interval / self.zoom) as i64);
        
        while frame <= end_frame.0 {
            let x = self.frame_to_x(Frame(frame));
            if x >= rect.x && x <= rect.x + rect.w {
                ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: ctx.theme.border_normal,
                    rect: [x, ruler_rect.y, 1.0, ruler_rect.h],
                });
                
                let tc = Timecode::from_frame(Frame(frame), FrameRate::Fps24);
                ctx.text_renderer.draw_text(&tc.format(), Point2::new(x + 2.0, ruler_rect.y + 2.0), ctx.theme.font_size_xs, ctx.theme.text_secondary);
            }
            frame += (major_interval / self.zoom) as i64;
        }
    }

    fn draw_tracks(&self, ctx: &DrawContext, rect: Rect, timeline: &Timeline) {
        let track_start_y = rect.y + self.time_ruler_height - self.scroll_y;
        
        // Video tracks
        for (i, track) in timeline.tracks.video_tracks.iter().enumerate() {
            let y = track_start_y + i as f32 * 60.0;
            let track_rect = Rect::new(rect.x, y, rect.w, 60.0);
            
            // Track background
            let bg = if track.enabled { ctx.theme.video_track } else { 
                [ctx.theme.video_track[0] * 0.5, ctx.theme.video_track[1] * 0.5, ctx.theme.video_track[2] * 0.5, 0.5]
            };
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: bg,
                rect: [track_rect.x, track_rect.y, track_rect.w, track_rect.h],
            });

            // Track header
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_tertiary,
                rect: [track_rect.x, track_rect.y, 80.0, track_rect.h],
            });
            ctx.text_renderer.draw_text(&track.name, Point2::new(track_rect.x + 5.0, track_rect.y + track_rect.h * 0.5), ctx.theme.font_size_sm, ctx.theme.text_primary);

            // Clips
            if let Some(tl) = self.timeline.as_ref().and_then(|t| t.read().ok()) {
                for (_, key) in track.items() {
                    if let Some(item) = tl.item_by_key(*key) {
                        self.draw_clip(ctx, track_rect, item);
                    }
                }
            }
        }

        // Audio tracks
        let video_count = timeline.tracks.video_tracks.len();
        for (i, track) in timeline.tracks.audio_tracks.iter().enumerate() {
            let y = track_start_y + video_count as f32 * 60.0 + i as f32 * 40.0;
            let track_rect = Rect::new(rect.x, y, rect.w, 40.0);
            
            let bg = if track.enabled { ctx.theme.audio_track } else { 
                [ctx.theme.audio_track[0] * 0.5, ctx.theme.audio_track[1] * 0.5, ctx.theme.audio_track[2] * 0.5, 0.5]
            };
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: bg,
                rect: [track_rect.x, track_rect.y, track_rect.w, track_rect.h],
            });

            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_tertiary,
                rect: [track_rect.x, track_rect.y, 80.0, track_rect.h],
            });
            ctx.text_renderer.draw_text(&track.name, Point2::new(track_rect.x + 5.0, track_rect.y + track_rect.h * 0.5), ctx.theme.font_size_xs, ctx.theme.text_primary);

            // Audio waveforms would be drawn here
        }
    }

    fn draw_clip(&self, ctx: &DrawContext, track_rect: Rect, item: &TrackItem) {
        let x = self.frame_to_x(item.timeline_in()).max(track_rect.x + 80.0);
        let w = (item.timeline_duration().0 as f32 * self.zoom).max(10.0);
        let clip_rect = Rect::new(x, track_rect.y + 2.0, w, track_rect.h - 4.0);
        
        // Clip background
        let color = ctx.theme.clip_color_for_type(item.clip_id.map_or(MediaType::Video, |id| {
            // Would get media type from clip
            MediaType::Video
        }));
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
            color,
            rect: [clip_rect.x, clip_rect.y, clip_rect.w, clip_rect.h],
        });

        // Clip name
        ctx.text_renderer.draw_text(&item.metadata.name, Point2::new(clip_rect.x + 4.0, clip_rect.y + 4.0), ctx.theme.font_size_xs, ctx.theme.text_primary);

        // Selection highlight
        if self.selected_items.contains(&item.id) {
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.accent,
                rect: [clip_rect.x, clip_rect.y, clip_rect.w, 2.0],
            });
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.accent,
                rect: [clip_rect.x, clip_rect.y + clip_rect.h - 2.0, clip_rect.w, 2.0],
            });
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.accent,
                rect: [clip_rect.x, clip_rect.y, 2.0, clip_rect.h],
            });
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.accent,
                rect: [clip_rect.x + clip_rect.w - 2.0, clip_rect.y, 2.0, clip_rect.h],
            });
        }

        // Trim handles
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
            color: ctx.theme.text_secondary,
            rect: [clip_rect.x, clip_rect.y, 4.0, clip_rect.h],
        });
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
            color: ctx.theme.text_secondary,
            rect: [clip_rect.x + clip_rect.w - 4.0, clip_rect.y, 4.0, clip_rect.h],
        });
    }

    fn draw_playhead(&self, ctx: &DrawContext, rect: Rect) {
        let x = self.frame_to_x(self.playhead);
        if x >= rect.x && x <= rect.x + rect.w {
            // Playhead line
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.accent,
                rect: [x, rect.y + self.time_ruler_height, 2.0, rect.h - self.time_ruler_height],
            });
            
            // Playhead triangle
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.accent,
                rect: [x - 6.0, rect.y, 12.0, self.time_ruler_height],
            });
        }
    }
}

impl Widget for TimelineWidget {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (f32::INFINITY, 400.0)
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

        // Time ruler
        self.draw_time_ruler(ctx, rect);

        if let Some(timeline) = &self.timeline {
            if let Ok(tl) = timeline.read() {
                // Draw tracks
                self.draw_tracks(ctx, rect, &tl);

                // Draw playhead
                self.draw_playhead(ctx, rect);
            }
        }
    }

    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        match event {
            InputEvent::MouseMove(pos) => {
                if pos.x >= self.rect.x && pos.x < self.rect.x + self.rect.w && pos.y >= self.rect.y && pos.y < self.rect.y + self.rect.h {
                    // Update hover item
                }
                false
            }
            InputEvent::MouseDown(pos, btn) => {
                if pos.x >= self.rect.x && pos.x < self.rect.x + self.rect.w && pos.y >= self.rect.y && pos.y < self.rect.y + self.rect.h {
                    match btn {
                        MouseButton::Left => {
                            // Check if clicking on playhead
                            let playhead_x = self.frame_to_x(self.playhead);
                            if (pos.x - playhead_x).abs() < 10.0 {
                                self.drag_state = Some(TimelineDragState {
                                    item_id: TrackItemId::nil(),
                                    start_frame: self.playhead,
                                    start_mouse_x: pos.x,
                                    drag_type: DragType::Move,
                                });
                                return true;
                            }
                            
                            // Check for item click
                            if let Some(timeline) = &self.timeline {
                                if let Ok(tl) = timeline.read() {
                                    for item in tl.iter_items() {
                                        let item_x = self.frame_to_x(item.timeline_in());
                                        let item_w = item.timeline_duration().0 as f32 * self.zoom;
                                        if pos.x >= item_x && pos.x <= item_x + item_w {
                                            if pos.x - item_x < 10.0 {
                                                // Trim in
                                                self.drag_state = Some(TimelineDragState {
                                                    item_id: item.id,
                                                    start_frame: item.timeline_in(),
                                                    start_mouse_x: pos.x,
                                                    drag_type: DragType::TrimIn,
                                                });
                                            } else if item_x + item_w - pos.x < 10.0 {
                                                // Trim out
                                                self.drag_state = Some(TimelineDragState {
                                                    item_id: item.id,
                                                    start_frame: item.timeline_out(),
                                                    start_mouse_x: pos.x,
                                                    drag_type: DragType::TrimOut,
                                                });
                                            } else {
                                                // Move
                                                self.drag_state = Some(TimelineDragState {
                                                    item_id: item.id,
                                                    start_frame: item.timeline_in(),
                                                    start_mouse_x: pos.x,
                                                    drag_type: DragType::Move,
                                                });
                                            }
                                            return true;
                                        }
                                    }
                                }
                            }
                            false
                        }
                        _ => false
                    }
                } else { false }
            }
            InputEvent::MouseMove(pos) => {
                if let Some(drag) = &self.drag_state {
                    let delta_x = pos.x - drag.start_mouse_x;
                    let delta_frames = Frame((delta_x / self.zoom).round() as i64);
                    
                    match drag.drag_type {
                        DragType::Move => {
                            // Would move item
                        }
                        DragType::TrimIn | DragType::TrimOut => {
                            // Would trim item
                        }
                        _ => {}
                    }
                    true
                } else { false }
            }
            InputEvent::MouseUp(_, _) => {
                self.drag_state = None;
                true
            }
            InputEvent::Scroll(dx, dy) => {
                if dx != 0.0 { self.scroll_x = (self.scroll_x - dx * 50.0).max(0.0); }
                if dy != 0.0 { self.scroll_y = (self.scroll_y - dy * 50.0).max(0.0); }
                true
            }
            _ => false
        }
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}