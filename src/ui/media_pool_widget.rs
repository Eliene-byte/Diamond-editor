//! Media pool widget - clip browser and bin management

use crate::ui::widgets::*;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::media::pool::{MediaPool, PoolClip, PoolBin, ClipStatus};
use crate::core::id::{ClipId, BinId};
use crate::core::types::MediaType;
use std::sync::{Arc, RwLock};

pub struct MediaPoolWidget {
    pool: Option<Arc<RwLock<MediaPool>>>,
    current_bin: Option<BinId>,
    view_mode: ViewMode,
    sort_by: SortBy,
    sort_ascending: bool,
    search_text: String,
    selected_clips: Vec<ClipId>,
    clips: Vec<ClipListItem>,
    rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode { List, Thumbnail, Detail }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy { Name, Type, Duration, DateAdded, Resolution }

#[derive(Debug, Clone)]
struct ClipListItem {
    id: ClipId,
    name: String,
    media_type: MediaType,
    duration: crate::core::time::Frame,
    resolution: Option<crate::core::types::Resolution>,
    status: ClipStatus,
    thumbnail: Option<crate::gpu::texture::GpuTexture>,
}

impl MediaPoolWidget {
    pub fn new() -> Self {
        Self {
            pool: None,
            current_bin: None,
            view_mode: ViewMode::Thumbnail,
            sort_by: SortBy::Name,
            sort_ascending: true,
            search_text: String::new(),
            selected_clips: Vec::new(),
            clips: Vec::new(),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    pub fn set_pool(&mut self, pool: Arc<RwLock<MediaPool>>) {
        self.pool = Some(pool.clone());
        self.refresh();
    }

    pub fn set_bin(&mut self, bin_id: BinId) {
        self.current_bin = Some(bin_id);
        self.refresh();
    }

    fn refresh(&mut self) {
        self.clips.clear();
        if let Some(pool) = &self.pool {
            if let Ok(p) = pool.read() {
                let bin = self.current_bin.unwrap_or(p.root_bin);
                let clip_ids = p.list_clips(Some(bin));
                
                for clip_id in clip_ids {
                    if let Some(clip) = p.get_clip(clip_id) {
                        let matches_search = self.search_text.is_empty() || 
                            clip.metadata.tags.name.to_lowercase().contains(&self.search_text.to_lowercase());
                        
                        if matches_search {
                            self.clips.push(ClipListItem {
                                id: clip.id,
                                name: clip.metadata.tags.name.clone(),
                                media_type: clip.metadata.media_type,
                                duration: clip.metadata.duration,
                                resolution: clip.metadata.resolution,
                                status: clip.status,
                                // Would create GPU texture from thumbnail once a device is attached.
                                thumbnail: None,
                            });
                        }
                    }
                }
                
                // Sort
                self.clips.sort_by(|a, b| {
                    let ord = match self.sort_by {
                        SortBy::Name => a.name.cmp(&b.name),
                        SortBy::Type => format!("{:?}", a.media_type).cmp(&format!("{:?}", b.media_type)),
                        SortBy::Duration => a.duration.cmp(&b.duration),
                        SortBy::DateAdded => a.id.0.cmp(&b.id.0),
                        SortBy::Resolution => {
                            let ar = a.resolution.map(|r| r.0 * r.1).unwrap_or(0);
                            let br = b.resolution.map(|r| r.0 * r.1).unwrap_or(0);
                            ar.cmp(&br)
                        }
                    };
                    if self.sort_ascending { ord } else { ord.reverse() }
                });
            }
        }
    }
}

impl Widget for MediaPoolWidget {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (320.0, f32::INFINITY)
    }

    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) {
        self.rect = rect;
    }

    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        // Toolbar
        let toolbar_rect = Rect::new(rect.x, rect.y, rect.w, 40.0);
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
            color: ctx.theme.bg_secondary,
            rect: [toolbar_rect.x, toolbar_rect.y, toolbar_rect.w, toolbar_rect.h],
        });

        // Search
        let search_rect = Rect::new(toolbar_rect.x + 10.0, toolbar_rect.y + 5.0, 200.0, 30.0);
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
            color: ctx.theme.bg_tertiary,
            rect: [search_rect.x, search_rect.y, search_rect.w, search_rect.h],
        });
        ctx.text_renderer.draw_text(&format!("Search: {}", self.search_text), Point2::new(search_rect.x + 8.0, search_rect.y + 20.0), ctx.theme.font_size_sm, ctx.theme.text_primary);

        // View mode buttons
        let modes = [("List", ViewMode::List), ("Thumb", ViewMode::Thumbnail), ("Detail", ViewMode::Detail)];
        let mut btn_x = toolbar_rect.x + 230.0;
        for (label, mode) in modes {
            let active = self.view_mode == mode;
            let btn_rect = Rect::new(btn_x, toolbar_rect.y + 5.0, 50.0, 30.0);
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: if active { ctx.theme.accent } else { ctx.theme.bg_tertiary },
                rect: [btn_rect.x, btn_rect.y, btn_rect.w, btn_rect.h],
            });
            ctx.text_renderer.draw_text(label, Point2::new(btn_rect.x + btn_rect.w * 0.5, btn_rect.y + 20.0), ctx.theme.font_size_xs, ctx.theme.text_primary);
            btn_x += 55.0;
        }

        // Clip list
        let list_rect = Rect::new(rect.x, toolbar_rect.y + toolbar_rect.h, rect.w, rect.h - toolbar_rect.h);
        
        match self.view_mode {
            ViewMode::List => self.draw_list(ctx, list_rect),
            ViewMode::Thumbnail => self.draw_thumbnails(ctx, list_rect),
            ViewMode::Detail => self.draw_detail(ctx, list_rect),
        }
    }

    fn draw_list(&self, ctx: &DrawContext, rect: Rect) {
        let row_height = 24.0;
        for (i, clip) in self.clips.iter().enumerate() {
            let y = rect.y + i as f32 * row_height;
            if y > rect.y + rect.h { break; }
            
            let selected = self.selected_clips.contains(&clip.id);
            let bg = if selected { ctx.theme.bg_selected } else if i % 2 == 0 { ctx.theme.bg_secondary } else { ctx.theme.bg_primary };
            
            let row_rect = Rect::new(rect.x, y, rect.w, row_height);
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: bg,
                rect: [row_rect.x, row_rect.y, row_rect.w, row_rect.h],
            });

            // Type icon
            let icon = match clip.media_type {
                MediaType::Video => "🎬",
                MediaType::Audio => "🔊",
                MediaType::Image => "🖼",
                _ => "📄",
            };
            ctx.text_renderer.draw_text(icon, Point2::new(row_rect.x + 5.0, y + 16.0), ctx.theme.font_size_sm, ctx.theme.text_primary);

            // Name
            ctx.text_renderer.draw_text(&clip.name, Point2::new(row_rect.x + 25.0, y + 16.0), ctx.theme.font_size_sm, ctx.theme.text_primary);

            // Duration
            let tc = crate::core::time::Timecode::from_frame(clip.duration, crate::core::time::FrameRate::Fps24);
            ctx.text_renderer.draw_text(&tc.format(), Point2::new(row_rect.x + row_rect.w - 80.0, y + 16.0), ctx.theme.font_size_xs, ctx.theme.text_secondary);
        }
    }

    fn draw_thumbnails(&self, ctx: &DrawContext, rect: Rect) {
        let thumb_size = 120.0;
        let padding = 10.0;
        let cols = ((rect.w + padding) / (thumb_size + padding)).max(1.0) as usize;
        
        for (i, clip) in self.clips.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = rect.x + col as f32 * (thumb_size + padding) + padding;
            let y = rect.y + row as f32 * (thumb_size + padding + 20.0) + padding;
            
            if y > rect.y + rect.h { break; }
            
            let selected = self.selected_clips.contains(&clip.id);
            
            // Thumbnail
            let thumb_rect = Rect::new(x, y, thumb_size, thumb_size);
            if let Some(thumb) = &clip.thumbnail {
                ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawTexture {
                    texture: thumb.clone(),
                    dest_rect: [thumb_rect.x, thumb_rect.y, thumb_rect.w, thumb_rect.h],
                    src_rect: [0.0, 0.0, 1.0, 1.0],
                    opacity: 1.0,
                    blend_mode: crate::gpu::renderer::BlendMode::Normal,
                });
            } else {
                ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: ctx.theme.bg_tertiary,
                    rect: [thumb_rect.x, thumb_rect.y, thumb_rect.w, thumb_rect.h],
                });
            }

            // Selection border
            if selected {
                ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: ctx.theme.accent,
                    rect: [thumb_rect.x, thumb_rect.y, thumb_rect.w, 2.0],
                });
            }

            // Name below
            ctx.text_renderer.draw_text(&clip.name, Point2::new(x, y + thumb_size + 16.0), ctx.theme.font_size_xs, ctx.theme.text_primary);
        }
    }

    fn draw_detail(&self, ctx: &DrawContext, rect: Rect) {
        // Detailed view with metadata
        self.draw_list(ctx, rect);
    }

    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        match event {
            InputEvent::MouseDown(pos, btn) => {
                if pos.x >= self.rect.x && pos.x < self.rect.x + self.rect.w && pos.y >= self.rect.y && pos.y < self.rect.y + self.rect.h {
                    match btn {
                        MouseButton::Left => {
                            // Select clip
                            let row_height = 24.0;
                            let rel_y = pos.y - self.rect.y - 40.0;
                            if rel_y >= 0.0 {
                                let index = (rel_y / row_height) as usize;
                                if index < self.clips.len() {
                                    let clip_id = self.clips[index].id;
                                    if ctx.key_modifiers.shift {
                                        self.selected_clips.push(clip_id);
                                    } else if ctx.key_modifiers.ctrl {
                                        if self.selected_clips.contains(&clip_id) {
                                            self.selected_clips.retain(|&id| id != clip_id);
                                        } else {
                                            self.selected_clips.push(clip_id);
                                        }
                                    } else {
                                        self.selected_clips = vec![clip_id];
                                    }
                                    return true;
                                }
                            }
                            false
                        }
                        MouseButton::Right => {
                            // Context menu
                            true
                        }
                        _ => false
                    }
                } else { false }
            }
            InputEvent::KeyDown(key) => {
                if key == Key::Delete {
                    // Delete selected
                    true
                } else { false }
            }
            _ => false
        }
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}