//! Cut page - fast assembly editing
use crate::core::math::{Point2, Rect};

use crate::media::pool::MediaPool;
use crate::timeline::clip::Timeline;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::ui::widgets::*;

pub struct CutPage {
    timeline: Option<Arc<RwLock<Timeline>>>,
    media_pool: Option<Arc<RwLock<MediaPool>>>,
    source_viewer: ViewerWidget,
    timeline_viewer: ViewerWidget,
    timeline_widget: TimelineWidget,
    toolbar: Toolbar,
    media_pool_widget: MediaPoolWidget,
}

impl CutPage {
    pub fn new() -> Self {
        Self {
            timeline: None,
            media_pool: None,
            source_viewer: ViewerWidget::new(ViewerMode::Source),
            timeline_viewer: ViewerWidget::new(ViewerMode::Timeline),
            timeline_widget: TimelineWidget::new(),
            toolbar: Toolbar::cut_toolbar(),
            media_pool_widget: MediaPoolWidget::new(),
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline.clone());
        self.timeline_widget.set_timeline(timeline.clone());
        self.timeline_viewer.set_timeline(timeline);
    }

    pub fn set_media_pool(&mut self, pool: Arc<RwLock<MediaPool>>) {
        self.media_pool = Some(pool.clone());
        self.media_pool_widget.set_pool(pool);
    }

    /// Build the page's widget tree.
    ///
    /// The tree is rebuilt from clones on every call so callers get an owned
    /// `Box<dyn Widget>`; the page keeps its authoritative sub-widget state and
    /// rebuilds only when the layout is (re)computed.
    pub fn ui(&mut self, _ctx: &LayoutContext) -> Box<dyn Widget> {
        let left = Flex::new(Direction::Vertical)
            .child(Box::new(self.source_viewer.clone()))
            .child(Box::new(self.media_pool_widget.clone()))
            .flex_grow(0.0);

        let right = Flex::new(Direction::Vertical)
            .child(Box::new(self.timeline_viewer.clone()))
            .child(Box::new(self.timeline_widget.clone()))
            .flex_grow(1.0);

        let body = Flex::new(Direction::Horizontal)
            .child(Box::new(left))
            .child(Box::new(right))
            .flex_grow(1.0);

        Box::new(
            Flex::new(Direction::Vertical)
                .child(Box::new(self.toolbar.clone()))
                .child(Box::new(body)),
        )
    }
}

use crate::ui::media_pool_widget::MediaPoolWidget;
use crate::ui::timeline_widget::TimelineWidget;
use crate::ui::viewer::{ViewerMode, ViewerWidget};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
struct Toolbar;

impl Toolbar {
    fn cut_toolbar() -> Self {
        Self
    }
}

impl Widget for Toolbar {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (f32::INFINITY, ctx.theme.header_height)
    }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_secondary,
                rect: [rect.x, rect.y, rect.w, rect.h],
            });
        // Draw toolbar buttons
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool {
        false
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
