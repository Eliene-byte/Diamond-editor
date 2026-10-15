//! Basic UI widgets

use crate::core::math::{Point2, Rect};
use crate::ui::layout::{Widget, LayoutContext, DrawContext, InputContext, InputEvent, Constraints, LayoutResult};
use crate::ui::theme::Theme;

/// Button widget
pub struct Button {
    pub label: String,
    pub on_click: Option<Box<dyn Fn() + Send + Sync>>,
    pub variant: ButtonVariant,
    pub size: ButtonSize,
    pub enabled: bool,
    pub icon: Option<String>,
    rect: Rect,
    hovered: bool,
    pressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant { Primary, Secondary, Ghost, Danger, Success }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize { Small, Medium, Large }

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), on_click: None, variant: ButtonVariant::Primary, size: ButtonSize::Medium, enabled: true, icon: None, rect: Rect::new(0.0, 0.0, 0.0, 0.0), hovered: false, pressed: false }
    }
    pub fn on_click(mut self, f: impl Fn() + Send + Sync + 'static) -> Self { self.on_click = Some(Box::new(f)); self }
    pub fn variant(mut self, v: ButtonVariant) -> Self { self.variant = v; self }
    pub fn size(mut self, s: ButtonSize) -> Self { self.size = s; self }
    pub fn enabled(mut self, e: bool) -> Self { self.enabled = e; self }
    pub fn icon(mut self, i: impl Into<String>) -> Self { self.icon = Some(i.into()); self }
}

impl Widget for Button {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        let text_w = ctx.theme.font_size_md * self.label.len() as f32 * 0.6;
        let text_h = ctx.theme.font_size_md;
        let (pad_x, pad_y) = match self.size { ButtonSize::Small => (12.0, 6.0), ButtonSize::Medium => (16.0, 8.0), ButtonSize::Large => (24.0, 12.0) };
        (text_w + pad_x * 2.0, text_h + pad_y * 2.0)
    }
    
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) { self.rect = rect; }
    
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        let (bg, border, text) = match (self.variant, self.enabled, self.hovered, self.pressed) {
            (ButtonVariant::Primary, true, _, true) => (ctx.theme.accent_active, ctx.theme.accent_active, ctx.theme.text_primary),
            (ButtonVariant::Primary, true, true, _) => (ctx.theme.accent_hover, ctx.theme.accent_hover, ctx.theme.text_primary),
            (ButtonVariant::Primary, true, _, _) => (ctx.theme.accent, ctx.theme.accent, ctx.theme.text_primary),
            (ButtonVariant::Primary, false, _, _) => (ctx.theme.bg_tertiary, ctx.theme.border_subtle, ctx.theme.text_disabled),
            (ButtonVariant::Secondary, true, _, true) => (ctx.theme.bg_active, ctx.theme.border_normal, ctx.theme.text_primary),
            (ButtonVariant::Secondary, true, true, _) => (ctx.theme.bg_hover, ctx.theme.border_normal, ctx.theme.text_primary),
            (ButtonVariant::Secondary, true, _, _) => (ctx.theme.bg_secondary, ctx.theme.border_normal, ctx.theme.text_primary),
            (ButtonVariant::Secondary, false, _, _) => (ctx.theme.bg_tertiary, ctx.theme.border_subtle, ctx.theme.text_disabled),
            (ButtonVariant::Ghost, true, _, true) => (ctx.theme.bg_active, ctx.theme.bg_active, ctx.theme.text_primary),
            (ButtonVariant::Ghost, true, true, _) => (ctx.theme.bg_hover, ctx.theme.bg_hover, ctx.theme.text_primary),
            (ButtonVariant::Ghost, true, _, _) => (ctx.theme.bg_secondary, ctx.theme.bg_secondary, ctx.theme.text_primary),
            (ButtonVariant::Ghost, false, _, _) => (ctx.theme.bg_tertiary, ctx.theme.bg_tertiary, ctx.theme.text_disabled),
            (ButtonVariant::Danger, true, _, true) => (hex("#cc0000"), hex("#cc0000"), ctx.theme.text_primary),
            (ButtonVariant::Danger, true, true, _) => (hex("#e81123"), hex("#e81123"), ctx.theme.text_primary),
            (ButtonVariant::Danger, true, _, _) => (ctx.theme.error, ctx.theme.error, ctx.theme.text_primary),
            (ButtonVariant::Danger, false, _, _) => (ctx.theme.bg_tertiary, ctx.theme.border_subtle, ctx.theme.text_disabled),
            (ButtonVariant::Success, true, _, true) => (hex("#0e6e0e"), hex("#0e6e0e"), ctx.theme.text_primary),
            (ButtonVariant::Success, true, true, _) => (hex("#107c10"), hex("#107c10"), ctx.theme.text_primary),
            (ButtonVariant::Success, true, _, _) => (ctx.theme.success, ctx.theme.success, ctx.theme.text_primary),
            (ButtonVariant::Success, false, _, _) => (ctx.theme.bg_tertiary, ctx.theme.border_subtle, ctx.theme.text_disabled),
        };
        
        // Draw button background
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: bg, rect: [rect.x, rect.y, rect.w, rect.h] });
        // Draw border
        // Draw text
        ctx.text_renderer.draw_text(&self.label, Point2::new(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5), ctx.theme.font_size_md, text);
    }
    
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        if !self.enabled { return false; }
        match event {
            InputEvent::MouseMove(pos) => {
                let was_hovered = self.hovered;
                self.hovered = pos.x >= self.rect.x && pos.x < self.rect.x + self.rect.w && pos.y >= self.rect.y && pos.y < self.rect.y + self.rect.h;
                was_hovered != self.hovered
            }
            InputEvent::MouseDown(pos, _) => {
                if self.hovered { self.pressed = true; true } else { false }
            }
            InputEvent::MouseUp(pos, _) => {
                if self.pressed && self.hovered {
                    self.pressed = false;
                    if let Some(cb) = &self.on_click { cb(); }
                    true
                } else {
                    self.pressed = false;
                    false
                }
            }
            _ => false
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Label widget
pub struct Label {
    pub text: String,
    pub size: LabelSize,
    pub color: Option<[f32; 4]>,
    pub align: TextAlign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelSize { Xs, Sm, Md, Lg, Xl, Title }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign { Left, Center, Right }

impl Label {
    pub fn new(text: impl Into<String>) -> Self { Self { text: text.into(), size: LabelSize::Md, color: None, align: TextAlign::Left } }
    pub fn size(mut self, s: LabelSize) -> Self { self.size = s; self }
    pub fn color(mut self, c: [f32; 4]) -> Self { self.color = Some(c); self }
    pub fn align(mut self, a: TextAlign) -> Self { self.align = a; self }
}

impl Widget for Label {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        let font_size = match self.size { LabelSize::Xs => ctx.theme.font_size_xs, LabelSize::Sm => ctx.theme.font_size_sm, LabelSize::Md => ctx.theme.font_size_md, LabelSize::Lg => ctx.theme.font_size_lg, LabelSize::Xl => ctx.theme.font_size_xl, LabelSize::Title => ctx.theme.font_size_title };
        let (w, h) = ctx.measure_text(&self.text, font_size);
        (w, h)
    }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        let font_size = match self.size { LabelSize::Xs => ctx.theme.font_size_xs, LabelSize::Sm => ctx.theme.font_size_sm, LabelSize::Md => ctx.theme.font_size_md, LabelSize::Lg => ctx.theme.font_size_lg, LabelSize::Xl => ctx.theme.font_size_xl, LabelSize::Title => ctx.theme.font_size_title };
        let color = self.color.unwrap_or(ctx.theme.text_primary);
        let x = match self.align { TextAlign::Left => rect.x, TextAlign::Center => rect.x + rect.w * 0.5, TextAlign::Right => rect.x + rect.w };
        ctx.text_renderer.draw_text(&self.text, Point2::new(x, rect.y + rect.h * 0.5), font_size, color);
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Input field
pub struct Input {
    pub value: String,
    pub placeholder: String,
    pub on_change: Option<Box<dyn Fn(String) + Send + Sync>>,
    pub on_submit: Option<Box<dyn Fn() + Send + Sync>>,
    pub input_type: InputType,
    rect: Rect,
    focused: bool,
    cursor_pos: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType { Text, Number, Password, Timecode, Frame }

impl Input {
    pub fn new() -> Self { Self { value: String::new(), placeholder: String::new(), on_change: None, on_submit: None, input_type: InputType::Text, rect: Rect::new(0.0, 0.0, 0.0, 0.0), focused: false, cursor_pos: 0 } }
    pub fn value(mut self, v: impl Into<String>) -> Self { self.value = v.into(); self }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self { self.placeholder = p.into(); self }
    pub fn on_change(mut self, f: impl Fn(String) + Send + Sync + 'static) -> Self { self.on_change = Some(Box::new(f)); self }
    pub fn on_submit(mut self, f: impl Fn() + Send + Sync + 'static) -> Self { self.on_submit = Some(Box::new(f)); self }
    pub fn input_type(mut self, t: InputType) -> Self { self.input_type = t; self }
}

impl Widget for Input {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (200.0, ctx.theme.input_height)
    }
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) { self.rect = rect; }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        let bg = if self.focused { ctx.theme.bg_focus } else { ctx.theme.bg_secondary };
        let border = if self.focused { ctx.theme.border_focus } else { ctx.theme.border_normal };
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: bg, rect: [rect.x, rect.y, rect.w, rect.h] });
        let text = if self.value.is_empty() && !self.focused { &self.placeholder } else { &self.value };
        let color = if self.value.is_empty() && !self.focused { ctx.theme.text_disabled } else { ctx.theme.text_primary };
        ctx.text_renderer.draw_text(text, Point2::new(rect.x + 8.0, rect.y + rect.h * 0.5), ctx.theme.font_size_md, color);
    }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        match event {
            InputEvent::MouseDown(pos, _) => {
                let was_focused = self.focused;
                self.focused = pos.x >= self.rect.x && pos.x < self.rect.x + self.rect.w && pos.y >= self.rect.y && pos.y < self.rect.y + self.rect.h;
                was_focused != self.focused
            }
            InputEvent::KeyDown(key) => {
                if self.focused {
                    match key {
                        crate::ui::layout::Key::Enter => { if let Some(cb) = &self.on_submit { cb(); } true }
                        crate::ui::layout::Key::Escape => { self.focused = false; true }
                        _ => false
                    }
                } else { false }
            }
            InputEvent::TextInput(text) => {
                if self.focused {
                    self.value.push_str(&text);
                    if let Some(cb) = &self.on_change { cb(self.value.clone()); }
                    true
                } else { false }
            }
            _ => false
        }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Slider widget
pub struct Slider {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub on_change: Option<Box<dyn Fn(f32) + Send + Sync>>,
    pub show_value: bool,
    rect: Rect,
    dragging: bool,
}

impl Slider {
    pub fn new(min: f32, max: f32) -> Self { Self { value: min, min, max, step: 0.01, on_change: None, show_value: true, rect: Rect::new(0.0, 0.0, 0.0, 0.0), dragging: false } }
    pub fn value(mut self, v: f32) -> Self { self.value = v.clamp(self.min, self.max); self }
    pub fn step(mut self, s: f32) -> Self { self.step = s; self }
    pub fn on_change(mut self, f: impl Fn(f32) + Send + Sync + 'static) -> Self { self.on_change = Some(Box::new(f)); self }
    pub fn show_value(mut self, s: bool) -> Self { self.show_value = s; self }
}

impl Widget for Slider {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (200.0, ctx.theme.slider_height)
    }
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) { self.rect = rect; }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        let track_rect = Rect::new(rect.x, rect.y + rect.h * 0.5 - 2.0, rect.w, 4.0);
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.bg_tertiary, rect: [track_rect.x, track_rect.y, track_rect.w, track_rect.h] });
        
        let progress = (self.value - self.min) / (self.max - self.min);
        let fill_w = track_rect.w * progress.clamp(0.0, 1.0);
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.accent, rect: [track_rect.x, track_rect.y, fill_w, track_rect.h] });
        
        let handle_x = track_rect.x + fill_w;
        let handle_rect = Rect::new(handle_x - 8.0, track_rect.y - 4.0, 16.0, 12.0);
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.accent, rect: [handle_rect.x, handle_rect.y, handle_rect.w, handle_rect.h] });
        
        if self.show_value {
            ctx.text_renderer.draw_text(&format!("{:.2}", self.value), Point2::new(rect.x + rect.w + 10.0, rect.y + rect.h * 0.5), ctx.theme.font_size_sm, ctx.theme.text_secondary);
        }
    }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        match event {
            InputEvent::MouseDown(pos, _) => {
                if pos.x >= self.rect.x && pos.x < self.rect.x + self.rect.w && pos.y >= self.rect.y && pos.y < self.rect.y + self.rect.h {
                    self.dragging = true;
                    self.update_from_pos(pos.x);
                    true
                } else { false }
            }
            InputEvent::MouseMove(pos) => {
                if self.dragging {
                    self.update_from_pos(pos.x);
                    true
                } else { false }
            }
            InputEvent::MouseUp(_, _) => {
                if self.dragging { self.dragging = false; true } else { false }
            }
            _ => false
        }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl Slider {
    fn update_from_pos(&mut self, x: f32) {
        let track_x = self.rect.x;
        let track_w = self.rect.w;
        let progress = ((x - track_x) / track_w).clamp(0.0, 1.0);
        let new_value = self.min + progress * (self.max - self.min);
        let stepped = (new_value / self.step).round() * self.step;
        if (stepped - self.value).abs() > f32::EPSILON {
            self.value = stepped.clamp(self.min, self.max);
            if let Some(cb) = &self.on_change { cb(self.value); }
        }
    }
}

/// Checkbox
pub struct Checkbox {
    pub label: String,
    pub checked: bool,
    pub on_change: Option<Box<dyn Fn(bool) + Send + Sync>>,
    rect: Rect,
    hovered: bool,
}

impl Checkbox {
    pub fn new(label: impl Into<String>) -> Self { Self { label: label.into(), checked: false, on_change: None, rect: Rect::new(0.0, 0.0, 0.0, 0.0), hovered: false } }
    pub fn checked(mut self, c: bool) -> Self { self.checked = c; self }
    pub fn on_change(mut self, f: impl Fn(bool) + Send + Sync + 'static) -> Self { self.on_change = Some(Box::new(f)); self }
}

impl Widget for Checkbox {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        let (text_w, text_h) = ctx.measure_text(&self.label, ctx.theme.font_size_md);
        (20.0 + 8.0 + text_w, text_h.max(20.0))
    }
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) { self.rect = rect; }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        let box_rect = Rect::new(rect.x, rect.y + (rect.h - 16.0) * 0.5, 16.0, 16.0);
        let bg = if self.hovered { ctx.theme.bg_hover } else { ctx.theme.bg_secondary };
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: bg, rect: [box_rect.x, box_rect.y, box_rect.w, box_rect.h] });
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.border_normal, rect: [box_rect.x, box_rect.y, box_rect.w, 1.0] });
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.border_normal, rect: [box_rect.x, box_rect.y + box_rect.h - 1.0, box_rect.w, 1.0] });
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.border_normal, rect: [box_rect.x, box_rect.y, 1.0, box_rect.h] });
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.border_normal, rect: [box_rect.x + box_rect.w - 1.0, box_rect.y, 1.0, box_rect.h] });
        
        if self.checked {
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.accent, rect: [box_rect.x + 3.0, box_rect.y + 3.0, 10.0, 10.0] });
        }
        
        ctx.text_renderer.draw_text(&self.label, Point2::new(box_rect.x + 24.0, rect.y + rect.h * 0.5), ctx.theme.font_size_md, ctx.theme.text_primary);
    }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        match event {
            InputEvent::MouseMove(pos) => {
                let was_hovered = self.hovered;
                self.hovered = pos.x >= self.rect.x && pos.x < self.rect.x + self.rect.w && pos.y >= self.rect.y && pos.y < self.rect.y + self.rect.h;
                was_hovered != self.hovered
            }
            InputEvent::MouseDown(pos, _) => {
                if self.hovered { self.checked = !self.checked; if let Some(cb) = &self.on_change { cb(self.checked); } true } else { false }
            }
            _ => false
        }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Separator
pub struct Separator { pub orientation: SeparatorOrientation, rect: Rect }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorOrientation { Horizontal, Vertical }

impl Separator {
    pub fn horizontal() -> Self { Self { orientation: SeparatorOrientation::Horizontal, rect: Rect::new(0.0, 0.0, 0.0, 0.0) } }
    pub fn vertical() -> Self { Self { orientation: SeparatorOrientation::Vertical, rect: Rect::new(0.0, 0.0, 0.0, 0.0) } }
}

impl Widget for Separator {
    fn measure(&self, _ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        match self.orientation { SeparatorOrientation::Horizontal => (f32::INFINITY, 1.0), SeparatorOrientation::Vertical => (1.0, f32::INFINITY) }
    }
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) { self.rect = rect; }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.border_subtle, rect: [rect.x, rect.y, rect.w, rect.h] });
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Tooltip
pub struct Tooltip {
    pub content: String,
    pub target: Rect,
    pub position: TooltipPosition,
    visible: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TooltipPosition { Top, Bottom, Left, Right }

impl Tooltip {
    pub fn new(content: impl Into<String>) -> Self { Self { content: content.into(), target: Rect::new(0.0, 0.0, 0.0, 0.0), position: TooltipPosition::Top, visible: false } }
}

impl Widget for Tooltip {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        let (w, h) = ctx.measure_text(&self.content, ctx.theme.font_size_sm);
        (w + 16.0, h + 8.0)
    }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        if !self.visible { return; }
        ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: ctx.theme.bg_tertiary, rect: [rect.x, rect.y, rect.w, rect.h] });
        ctx.text_renderer.draw_text(&self.content, Point2::new(rect.x + 8.0, rect.y + rect.h * 0.5), ctx.theme.font_size_sm, ctx.theme.text_primary);
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Panel/Container
pub struct Panel {
    pub children: Vec<Box<dyn Widget>>,
    pub padding: [f32; 4],
    pub background: Option<[f32; 4]>,
    rect: Rect,
}

impl Panel {
    pub fn new() -> Self { Self { children: Vec::new(), padding: [0.0; 4], background: None, rect: Rect::new(0.0, 0.0, 0.0, 0.0) } }
    pub fn child(mut self, widget: Box<dyn Widget>) -> Self { self.children.push(widget); self }
    pub fn padding(mut self, p: [f32; 4]) -> Self { self.padding = p; self }
    pub fn background(mut self, c: [f32; 4]) -> Self { self.background = Some(c); self }
}

impl Widget for Panel {
    fn measure(&self, ctx: &LayoutContext, constraints: Constraints) -> (f32, f32) {
        let mut max_w: f32 = 0.0;
        let mut total_h = 0.0;
        for child in &self.children {
            let (w, h) = child.measure(ctx, constraints);
            max_w = max_w.max(w);
            total_h += h;
        }
        (max_w + self.padding[1] + self.padding[3], total_h + self.padding[0] + self.padding[2])
    }
    fn layout(&mut self, ctx: &LayoutContext, rect: Rect) {
        self.rect = rect;
        let mut y = rect.y + self.padding[0];
        for child in &mut self.children {
            let (w, h) = child.measure(ctx, Constraints::default());
            child.layout(ctx, Rect::new(rect.x + self.padding[3], y, rect.w - self.padding[1] - self.padding[3], h));
            y += h;
        }
    }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        if let Some(bg) = self.background {
            ctx.renderer.submit(crate::gpu::renderer::RenderCommand::DrawQuad { color: bg, rect: [rect.x, rect.y, rect.w, rect.h] });
        }
        for child in &self.children {
            // Would draw child at its layout rect
        }
    }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        for child in &mut self.children {
            if child.handle_input(ctx, event.clone()) { return true; }
        }
        false
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Scroll area
pub struct ScrollArea {
    pub child: Box<dyn Widget>,
    pub scroll_x: f32,
    pub scroll_y: f32,
    rect: Rect,
    content_size: (f32, f32),
}

impl ScrollArea {
    pub fn new(child: Box<dyn Widget>) -> Self { Self { child, scroll_x: 0.0, scroll_y: 0.0, rect: Rect::new(0.0, 0.0, 0.0, 0.0), content_size: (0.0, 0.0) } }
}

impl Widget for ScrollArea {
    fn measure(&self, ctx: &LayoutContext, constraints: Constraints) -> (f32, f32) {
        self.child.measure(ctx, constraints)
    }
    fn layout(&mut self, ctx: &LayoutContext, rect: Rect) {
        self.rect = rect;
        self.content_size = self.child.measure(ctx, Constraints::default());
        self.child.layout(ctx, Rect::new(rect.x - self.scroll_x, rect.y - self.scroll_y, self.content_size.0, self.content_size.1));
    }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        // Would use scissor rect for clipping
        // self.child.draw(ctx, ...)
    }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        match event {
            InputEvent::Scroll(dx, dy) => {
                self.scroll_x = (self.scroll_x + dx * 50.0).clamp(0.0, (self.content_size.0 - self.rect.w).max(0.0));
                self.scroll_y = (self.scroll_y + dy * 50.0).clamp(0.0, (self.content_size.1 - self.rect.h).max(0.0));
                true
            }
            _ => self.child.handle_input(ctx, event)
        }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

fn hex(s: &str) -> [f32; 4] {
    let s = s.trim_start_matches('#');
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0) as f32 / 255.0;
    [r, g, b, 1.0]
}