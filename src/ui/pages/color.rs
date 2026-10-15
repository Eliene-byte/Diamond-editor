//! Color page - professional color grading
use crate::core::math::{Rect, Point2};

use crate::ui::widgets::*;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::timeline::clip::Timeline;
use crate::ui::viewer::{ViewerWidget, ViewerMode};
use crate::ui::scopes_widget::ScopesWidget;
use crate::ui::node_graph::NodeGraphWidget;
use crate::ui::inspector::ColorInspector;
use std::sync::{Arc, RwLock};

pub struct ColorPage {
    timeline: Option<Arc<RwLock<Timeline>>>,
    viewer: ViewerWidget,
    scopes: ScopesWidget,
    node_graph: NodeGraphWidget,
    color_inspector: ColorInspector,
    gallery: GalleryWidget,
    timeline_widget: crate::ui::timeline_widget::TimelineWidget,
    top_bar: ColorTopBar,
}

impl ColorPage {
    pub fn new() -> Self {
        Self {
            timeline: None,
            viewer: ViewerWidget::new(ViewerMode::Color),
            scopes: ScopesWidget::new(),
            node_graph: NodeGraphWidget::new(),
            color_inspector: ColorInspector::new(),
            gallery: GalleryWidget::new(),
            timeline_widget: crate::ui::timeline_widget::TimelineWidget::new(),
            top_bar: ColorTopBar::new(),
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline.clone());
        self.timeline_widget.set_timeline(timeline.clone());
        self.viewer.set_timeline(timeline.clone());
        self.node_graph.set_timeline(timeline.clone());
    }

    pub fn ui(&mut self, ctx: &LayoutContext) -> Box<dyn Widget> {
        Box::new(Flex::new(Direction::Vertical)
            .child(Box::new(self.top_bar.clone()))
            .child(Box::new(Flex::new(Direction::Horizontal)
                // Left: Gallery + Node graph
                .child(Box::new(Flex::new(Direction::Vertical)
                    .child(Box::new(self.gallery.clone()))
                    .child(Box::new(self.node_graph.clone()))
                    .flex_grow(1.0)
                ).flex_basis(300.0))
                
                // Center: Viewer + Scopes
                .child(Box::new(Flex::new(Direction::Vertical)
                    .child(Box::new(self.viewer.clone()).flex_grow(1.0))
                    .child(Box::new(self.scopes.clone()).flex_basis(200.0))
                ).flex_grow(1.0))
                
                // Right: Color inspector + Timeline
                .child(Box::new(Flex::new(Direction::Vertical)
                    .child(Box::new(self.color_inspector.clone()).flex_grow(1.0))
                    .child(Box::new(self.timeline_widget.clone()).flex_basis(200.0))
                ).flex_basis(300.0))
            ).flex_grow(1.0))
        )
    }
}

#[derive(Clone)]
struct ColorTopBar;

impl ColorTopBar {
    fn new() -> Self { Self }
}

impl Widget for ColorTopBar {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) { (f32::INFINITY, ctx.theme.header_height) }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.bg_secondary, rect: [rect.x, rect.y, rect.w, rect.h] });
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

#[derive(Clone)]
struct GalleryWidget;

impl GalleryWidget {
    fn new() -> Self { Self }
}

impl Widget for GalleryWidget {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) { (300.0, 200.0) }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.bg_tertiary, rect: [rect.x, rect.y, rect.w, rect.h] });
        ctx.text_renderer.draw_text("Gallery", Point2::new(rect.x + 10.0, rect.y + 20.0), ctx.theme.font_size_md, ctx.theme.text_primary);
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}