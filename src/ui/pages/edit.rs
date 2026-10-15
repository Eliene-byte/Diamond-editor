//! Edit page - main professional editing page
use crate::core::math::{Rect, Point2};

use crate::ui::widgets::*;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::timeline::clip::Timeline;
use crate::media::pool::MediaPool;
use crate::ui::viewer::{ViewerWidget, ViewerMode};
use crate::ui::timeline_widget::TimelineWidget;
use crate::ui::media_pool_widget::MediaPoolWidget;
use crate::ui::inspector::InspectorWidget;
use crate::ui::mixer::AudioMixerWidget;
use std::sync::{Arc, RwLock};

pub struct EditPage {
    timeline: Option<Arc<RwLock<Timeline>>>,
    media_pool: Option<Arc<RwLock<MediaPool>>>,
    
    // Top bar
    top_bar: TopBar,
    
    // Main areas
    media_pool_area: MediaPoolArea,
    source_viewer: ViewerWidget,
    timeline_viewer: ViewerWidget,
    timeline_area: TimelineArea,
    inspector: InspectorWidget,
    mixer: AudioMixerWidget,
    
    // State
    active_panel: EditPanel,
    layout_mode: LayoutMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditPanel {
    MediaPool,
    Inspector,
    Mixer,
    Effects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Default,
    DualViewer,
    SingleViewer,
    FullTimeline,
}

impl EditPage {
    pub fn new() -> Self {
        Self {
            timeline: None,
            media_pool: None,
            top_bar: TopBar::new(),
            media_pool_area: MediaPoolArea::new(),
            source_viewer: ViewerWidget::new(ViewerMode::Source),
            timeline_viewer: ViewerWidget::new(ViewerMode::Timeline),
            timeline_area: TimelineArea::new(),
            inspector: InspectorWidget::new(),
            mixer: AudioMixerWidget::new(),
            active_panel: EditPanel::MediaPool,
            layout_mode: LayoutMode::Default,
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline.clone());
        self.timeline_area.set_timeline(timeline.clone());
        self.timeline_viewer.set_timeline(timeline);
    }

    pub fn set_media_pool(&mut self, pool: Arc<RwLock<MediaPool>>) {
        self.media_pool = Some(pool.clone());
        self.media_pool_area.set_pool(pool.clone());
        self.inspector.set_pool(pool);
    }

    pub fn ui(&mut self, ctx: &LayoutContext) -> Box<dyn Widget> {
        match self.layout_mode {
            LayoutMode::Default => self.default_layout(ctx),
            LayoutMode::DualViewer => self.dual_viewer_layout(ctx),
            LayoutMode::SingleViewer => self.single_viewer_layout(ctx),
            LayoutMode::FullTimeline => self.full_timeline_layout(ctx),
        }
    }

    fn default_layout(&mut self, _ctx: &LayoutContext) -> Box<dyn Widget> {
        // Left: Media pool + Source viewer
        let left = Flex::new(Direction::Vertical)
            .child(Box::new(self.media_pool_area.clone()))
            .child(Box::new(self.source_viewer.clone()))
            .flex_grow(1.0);

        // Center: Timeline viewer + Timeline
        let center = Flex::new(Direction::Vertical)
            .child(Box::new(self.timeline_viewer.clone()))
            .child(Box::new(self.timeline_area.clone()))
            .flex_grow(1.0);

        // Right: Inspector/Mixer/Effects
        let right = Flex::new(Direction::Vertical)
            .child(self.side_panel_tabs())
            .child(self.active_panel_widget())
            .flex_grow(1.0);

        let body = Flex::new(Direction::Horizontal)
            .child(Box::new(left).flex_basis(320.0))
            .child(Box::new(center).flex_grow(1.0))
            .child(Box::new(right).flex_basis(300.0))
            .flex_grow(1.0);

        Box::new(
            Flex::new(Direction::Vertical)
                .child(Box::new(self.top_bar.clone()))
                .child(Box::new(body)),
        )
    }

    fn dual_viewer_layout(&mut self, _ctx: &LayoutContext) -> Box<dyn Widget> {
        let viewers = Flex::new(Direction::Horizontal)
            .child(Box::new(self.source_viewer.clone()).flex_grow(1.0))
            .child(Box::new(self.timeline_viewer.clone()).flex_grow(1.0));

        Box::new(
            Flex::new(Direction::Vertical)
                .child(Box::new(self.top_bar.clone()))
                .child(Box::new(viewers).flex_basis(400.0))
                .child(Box::new(self.timeline_area.clone()).flex_grow(1.0)),
        )
    }

    fn single_viewer_layout(&mut self, _ctx: &LayoutContext) -> Box<dyn Widget> {
        let left = Flex::new(Direction::Vertical)
            .child(Box::new(self.media_pool_area.clone()))
            .child(Box::new(self.timeline_viewer.clone()))
            .flex_grow(1.0);

        let body = Flex::new(Direction::Horizontal)
            .child(Box::new(left).flex_basis(320.0))
            .child(Box::new(self.timeline_area.clone()).flex_grow(1.0))
            .child(self.active_panel_widget()).flex_basis(300.0)
            .flex_grow(1.0);

        Box::new(
            Flex::new(Direction::Vertical)
                .child(Box::new(self.top_bar.clone()))
                .child(Box::new(body)),
        )
    }

    fn full_timeline_layout(&mut self, _ctx: &LayoutContext) -> Box<dyn Widget> {
        Box::new(
            Flex::new(Direction::Vertical)
                .child(Box::new(self.top_bar.clone()))
                .child(Box::new(self.timeline_area.clone()).flex_grow(1.0)),
        )
    }

    fn side_panel_tabs(&self) -> Box<dyn Widget> {
        Box::new(Flex::new(Direction::Horizontal)
            .child(Box::new(TabButton::new("Media Pool", self.active_panel == EditPanel::MediaPool, || {})))
            .child(Box::new(TabButton::new("Inspector", self.active_panel == EditPanel::Inspector, || {})))
            .child(Box::new(TabButton::new("Mixer", self.active_panel == EditPanel::Mixer, || {})))
            .child(Box::new(TabButton::new("Effects", self.active_panel == EditPanel::Effects, || {})))
        )
    }

    fn active_panel_widget(&self) -> Box<dyn Widget> {
        match self.active_panel {
            EditPanel::MediaPool => Box::new(self.media_pool_area.clone()),
            EditPanel::Inspector => Box::new(self.inspector.clone()),
            EditPanel::Mixer => Box::new(self.mixer.clone()),
            EditPanel::Effects => Box::new(Label::new("Effects Panel").size(LabelSize::Md)),
        }
    }
}

#[derive(Clone)]
struct TopBar;

impl TopBar {
    fn new() -> Self { Self }
}

impl Widget for TopBar {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) { (f32::INFINITY, ctx.theme.header_height) }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.bg_secondary, rect: [rect.x, rect.y, rect.w, rect.h] });
        // Menu bar, transport controls, timecode display, etc.
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

#[derive(Clone)]
struct MediaPoolArea {
    pool_widget: MediaPoolWidget,
}

impl MediaPoolArea {
    fn new() -> Self { Self { pool_widget: MediaPoolWidget::new() } }
    fn set_pool(&mut self, pool: Arc<RwLock<MediaPool>>) { self.pool_widget.set_pool(pool); }
}

impl Widget for MediaPoolArea {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) { (320.0, 400.0) }
    fn layout(&mut self, ctx: &LayoutContext, rect: Rect) { self.pool_widget.layout(ctx, rect); }
    fn draw(&self, ctx: &DrawContext, rect: Rect) { self.pool_widget.draw(ctx, rect); }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool { self.pool_widget.handle_input(ctx, event) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

#[derive(Clone)]
struct TimelineArea {
    timeline_widget: TimelineWidget,
}

impl TimelineArea {
    fn new() -> Self { Self { timeline_widget: TimelineWidget::new() } }
    fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) { self.timeline_widget.set_timeline(timeline); }
}

impl Widget for TimelineArea {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) { (f32::INFINITY, 400.0) }
    fn layout(&mut self, ctx: &LayoutContext, rect: Rect) { self.timeline_widget.layout(ctx, rect); }
    fn draw(&self, ctx: &DrawContext, rect: Rect) { self.timeline_widget.draw(ctx, rect); }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool { self.timeline_widget.handle_input(ctx, event) }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

#[derive(Clone)]
struct TabButton {
    label: String,
    active: bool,
    on_click: std::sync::Arc<dyn Fn() + Send + Sync>,
    rect: Rect,
}

impl TabButton {
    fn new(label: impl Into<String>, active: bool, on_click: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            label: label.into(),
            active,
            on_click: std::sync::Arc::new(on_click),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        }
    }
}

impl Widget for TabButton {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        let (w, h) = ctx.measure_text(&self.label, ctx.theme.font_size_sm);
        (w + 24.0, h + 12.0)
    }
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) { self.rect = rect; }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        let bg = if self.active { ctx.theme.bg_selected } else { ctx.theme.bg_secondary };
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: bg, rect: [rect.x, rect.y, rect.w, rect.h] });
        ctx.text_renderer.draw_text(&self.label, Point2::new(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5), ctx.theme.font_size_sm, ctx.theme.text_primary);
    }
    fn handle_input(&mut self, _ctx: &InputContext, event: InputEvent) -> bool {
        if let InputEvent::Click(pos) = event {
            if event.hits(self.rect) {
                (self.on_click)();
                return true;
            }
        }
        false
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}