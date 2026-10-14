//! Fairlight page - professional audio post-production

use crate::ui::widgets::*;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::timeline::clip::Timeline;
use crate::ui::viewer::{ViewerWidget, ViewerMode};
use crate::ui::mixer::AudioMixerWidget;
use crate::ui::inspector::FairlightInspector;
use crate::ui::timeline_widget::TimelineWidget;
use std::sync::{Arc, RwLock};

pub struct FairlightPage {
    timeline: Option<Arc<RwLock<Timeline>>>,
    viewer: ViewerWidget,
    mixer: AudioMixerWidget,
    inspector: FairlightInspector,
    timeline_widget: TimelineWidget,
    toolbar: FairlightToolbar,
    meters_panel: MetersPanel,
}

impl FairlightPage {
    pub fn new() -> Self {
        Self {
            timeline: None,
            viewer: ViewerWidget::new(ViewerMode::Fairlight),
            mixer: AudioMixerWidget::new(),
            inspector: FairlightInspector::new(),
            timeline_widget: TimelineWidget::new(),
            toolbar: FairlightToolbar::new(),
            meters_panel: MetersPanel::new(),
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline.clone());
        self.timeline_widget.set_timeline(timeline.clone());
        self.viewer.set_timeline(timeline);
        self.mixer.set_timeline(timeline);
    }

    pub fn ui(&mut self, ctx: &LayoutContext) -> Box<dyn Widget> {
        Box::new(Flex::new(Direction::Vertical)
            .child(Box::new(self.toolbar.clone()))
            .child(Box::new(Flex::new(Direction::Horizontal)
                // Left: Mixer
                .child(Box::new(self.mixer.clone()).flex_basis(500.0))
                
                // Center: Timeline
                .child(Box::new(Flex::new(Direction::Vertical)
                    .child(Box::new(self.timeline_widget.clone()).flex_grow(1.0))
                ).flex_grow(1.0))
                
                // Right: Inspector + Meters
                .child(Box::new(Flex::new(Direction::Vertical)
                    .child(Box::new(self.inspector.clone()).flex_grow(1.0))
                    .child(Box::new(self.meters_panel.clone()).flex_basis(200.0))
                ).flex_basis(350.0))
            ).flex_grow(1.0))
        )
    }
}

#[derive(Clone)]
struct FairlightToolbar;

impl FairlightToolbar {
    fn new() -> Self { Self }
}

impl Widget for FairlightToolbar {
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
struct MetersPanel;

impl MetersPanel {
    fn new() -> Self { Self }
}

impl Widget for MetersPanel {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) { (350.0, 200.0) }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.bg_tertiary, rect: [rect.x, rect.y, rect.w, rect.h] });
        ctx.text_renderer.draw_text("Meters", Point2::new(rect.x + 10.0, rect.y + 20.0), ctx.theme.font_size_md, ctx.theme.text_primary);
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}