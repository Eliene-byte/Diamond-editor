//! Deliver page - export and render
use crate::core::math::{Point2, Rect};

use crate::timeline::clip::Timeline;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::ui::timeline_widget::TimelineWidget;
use crate::ui::viewer::{ViewerMode, ViewerWidget};
use crate::ui::widgets::*;
use std::sync::{Arc, RwLock};

pub struct DeliverPage {
    timeline: Option<Arc<RwLock<Timeline>>>,
    viewer: ViewerWidget,
    render_queue: RenderQueueWidget,
    render_settings: RenderSettingsWidget,
    preset_panel: PresetPanel,
    timeline_widget: TimelineWidget,
    toolbar: DeliverToolbar,
}

impl DeliverPage {
    pub fn new() -> Self {
        Self {
            timeline: None,
            viewer: ViewerWidget::new(ViewerMode::Deliver),
            render_queue: RenderQueueWidget::new(),
            render_settings: RenderSettingsWidget::new(),
            preset_panel: PresetPanel::new(),
            timeline_widget: TimelineWidget::new(),
            toolbar: DeliverToolbar::new(),
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline.clone());
        self.timeline_widget.set_timeline(timeline.clone());
        self.viewer.set_timeline(timeline.clone());
        self.render_queue.set_timeline(timeline.clone());
    }

    pub fn ui(&mut self, ctx: &LayoutContext) -> Box<dyn Widget> {
        Box::new(
            Flex::new(Direction::Vertical)
                .child(Box::new(self.toolbar.clone()))
                .child(
                    Box::new(
                        Flex::new(Direction::Horizontal)
                            // Left: Presets + Settings
                            .child(
                                Box::new(
                                    Flex::new(Direction::Vertical)
                                        .child(
                                            Box::new(self.preset_panel.clone()).flex_basis(300.0),
                                        )
                                        .child(
                                            Box::new(self.render_settings.clone()).flex_grow(1.0),
                                        ),
                                )
                                .flex_basis(350.0),
                            )
                            // Center: Viewer + Timeline
                            .child(
                                Box::new(
                                    Flex::new(Direction::Vertical)
                                        .child(Box::new(self.viewer.clone()).flex_grow(1.0))
                                        .child(
                                            Box::new(self.timeline_widget.clone())
                                                .flex_basis(150.0),
                                        ),
                                )
                                .flex_grow(1.0),
                            )
                            // Right: Render Queue
                            .child(Box::new(self.render_queue.clone()).flex_basis(400.0)),
                    )
                    .flex_grow(1.0),
                ),
        )
    }
}

#[derive(Clone)]
struct DeliverToolbar;

impl DeliverToolbar {
    fn new() -> Self {
        Self
    }
}

impl Widget for DeliverToolbar {
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
struct RenderQueueWidget {
    timeline: Option<Arc<RwLock<Timeline>>>,
}

impl RenderQueueWidget {
    fn new() -> Self {
        Self { timeline: None }
    }
    fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline);
    }
}

impl Widget for RenderQueueWidget {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (400.0, f32::INFINITY)
    }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_tertiary,
                rect: [rect.x, rect.y, rect.w, rect.h],
            });
        ctx.text_renderer.draw_text(
            "Render Queue",
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

#[derive(Clone)]
struct RenderSettingsWidget;

impl RenderSettingsWidget {
    fn new() -> Self {
        Self
    }
}

impl Widget for RenderSettingsWidget {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (350.0, 400.0)
    }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_tertiary,
                rect: [rect.x, rect.y, rect.w, rect.h],
            });
        ctx.text_renderer.draw_text(
            "Render Settings",
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

#[derive(Clone)]
struct PresetPanel;

impl PresetPanel {
    fn new() -> Self {
        Self
    }
}

impl Widget for PresetPanel {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (350.0, 300.0)
    }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_tertiary,
                rect: [rect.x, rect.y, rect.w, rect.h],
            });
        ctx.text_renderer.draw_text(
            "Presets",
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
