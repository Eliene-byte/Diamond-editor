//! Fusion page - visual effects and motion graphics
use crate::core::math::{Point2, Rect};

use crate::timeline::clip::Timeline;
use crate::ui::inspector::FusionInspector;
use crate::ui::layout::*;
use crate::ui::node_graph::NodeGraphWidget;
use crate::ui::theme::Theme;
use crate::ui::timeline_widget::TimelineWidget;
use crate::ui::viewer::{ViewerMode, ViewerWidget};
use crate::ui::widgets::*;
use std::sync::{Arc, RwLock};

pub struct FusionPage {
    timeline: Option<Arc<RwLock<Timeline>>>,
    viewer: ViewerWidget,
    node_graph: NodeGraphWidget,
    inspector: FusionInspector,
    toolbar: FusionToolbar,
    timeline_widget: TimelineWidget,
    tools_panel: ToolsPanel,
}

impl FusionPage {
    pub fn new() -> Self {
        Self {
            timeline: None,
            viewer: ViewerWidget::new(ViewerMode::Fusion),
            node_graph: NodeGraphWidget::new(),
            inspector: FusionInspector::new(),
            toolbar: FusionToolbar::new(),
            timeline_widget: TimelineWidget::new(),
            tools_panel: ToolsPanel::new(),
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline.clone());
        self.timeline_widget.set_timeline(timeline.clone());
        self.viewer.set_timeline(timeline.clone());
        self.node_graph.set_timeline(timeline.clone());
    }

    pub fn ui(&mut self, ctx: &LayoutContext) -> Box<dyn Widget> {
        Box::new(
            Flex::new(Direction::Vertical)
                .child(Box::new(self.toolbar.clone()))
                .child(
                    Box::new(
                        Flex::new(Direction::Horizontal)
                            // Left: Tools
                            .child(Box::new(self.tools_panel.clone()).flex_basis(250.0))
                            // Center: Node graph
                            .child(
                                Box::new(
                                    Flex::new(Direction::Vertical)
                                        .child(Box::new(self.node_graph.clone()).flex_grow(1.0))
                                        .child(
                                            Box::new(self.timeline_widget.clone())
                                                .flex_basis(150.0),
                                        ),
                                )
                                .flex_grow(1.0),
                            )
                            // Right: Viewer + Inspector
                            .child(
                                Box::new(
                                    Flex::new(Direction::Vertical)
                                        .child(Box::new(self.viewer.clone()).flex_grow(1.0))
                                        .child(Box::new(self.inspector.clone()).flex_basis(300.0)),
                                )
                                .flex_basis(400.0),
                            ),
                    )
                    .flex_grow(1.0),
                ),
        )
    }
}

#[derive(Clone)]
struct FusionToolbar;

impl FusionToolbar {
    fn new() -> Self {
        Self
    }
}

impl Widget for FusionToolbar {
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

#[derive(Clone)]
struct ToolsPanel;

impl ToolsPanel {
    fn new() -> Self {
        Self
    }
}

impl Widget for ToolsPanel {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (250.0, f32::INFINITY)
    }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_tertiary,
                rect: [rect.x, rect.y, rect.w, rect.h],
            });
        ctx.text_renderer.draw_text(
            "Tools",
            Point2::new(rect.x + 10.0, rect.y + 20.0),
            ctx.theme.font_size_md,
            ctx.theme.text_primary,
        );
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
