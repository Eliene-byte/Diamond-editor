//! Layout system - flexbox-like layout for UI

use crate::core::math::{Point2, Rect};
use crate::ui::theme::Theme;

/// Layout direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

/// Alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Layout constraints
#[derive(Debug, Clone, Copy)]
pub struct Constraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Default for Constraints {
    fn default() -> Self {
        Self { min_width: 0.0, max_width: f32::INFINITY, min_height: 0.0, max_height: f32::INFINITY }
    }
}

impl Constraints {
    pub fn tight(width: f32, height: f32) -> Self {
        Self { min_width: width, max_width: width, min_height: height, max_height: height }
    }
    
    pub fn loosen(&self) -> Self {
        Self { min_width: 0.0, min_height: 0.0, ..*self }
    }
}

/// Layout context
pub struct LayoutContext {
    pub theme: Theme,
    pub constraints: Constraints,
    pub scale: f32,
}

impl LayoutContext {
    pub fn new(theme: Theme) -> Self {
        Self { theme, constraints: Constraints::default(), scale: 1.0 }
    }
}

/// Layout result
#[derive(Debug, Clone, Copy)]
pub struct LayoutResult {
    pub rect: Rect,
    pub size: (f32, f32),
}

/// Flex container
pub struct Flex {
    pub direction: Direction,
    pub align_items: Align,
    pub justify_content: Align,
    pub gap: f32,
    pub padding: [f32; 4], // top, right, bottom, left
    pub children: Vec<FlexChild>,
}

#[derive(Debug, Clone)]
pub struct FlexChild {
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: f32,
    pub align_self: Option<Align>,
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub widget: Box<dyn Widget>,
}

impl Default for FlexChild {
    fn default() -> Self {
        Self { flex_grow: 0.0, flex_shrink: 1.0, flex_basis: 0.0, align_self: None, min_width: 0.0, max_width: f32::INFINITY, min_height: 0.0, max_height: f32::INFINITY, widget: Box::new(EmptyWidget) }
    }
}

impl Flex {
    pub fn new(direction: Direction) -> Self {
        Self { direction, align_items: Align::Stretch, justify_content: Align::Start, gap: 0.0, padding: [0.0; 4], children: Vec::new() }
    }

    pub fn child(mut self, widget: Box<dyn Widget>) -> Self {
        self.children.push(FlexChild { widget, ..Default::default() });
        self
    }

    pub fn gap(mut self, gap: f32) -> Self { self.gap = gap; self }
    pub fn padding(mut self, padding: [f32; 4]) -> Self { self.padding = padding; self }
    pub fn align_items(mut self, align: Align) -> Self { self.align_items = align; self }
    pub fn justify_content(mut self, justify: Align) -> Self { self.justify_content = justify; self }

    pub fn layout(&self, ctx: &LayoutContext, rect: Rect) -> Vec<LayoutResult> {
        let inner_rect = Rect::new(
            rect.x + self.padding[3],
            rect.y + self.padding[0],
            rect.w - self.padding[1] - self.padding[3],
            rect.h - self.padding[0] - self.padding[2],
        );
        
        let total_gap = self.gap * (self.children.len().saturating_sub(1)) as f32;
        let available = match self.direction {
            Direction::Horizontal => inner_rect.w - total_gap,
            Direction::Vertical => inner_rect.h - total_gap,
        };
        
        // Calculate flex sizes
        let mut results = Vec::new();
        let mut flex_grow_sum = 0.0;
        let mut used_space = 0.0;
        
        for child in &self.children {
            let basis = if child.flex_basis > 0.0 { child.flex_basis } else {
                let size = child.widget.measure(ctx, Constraints::default());
                match self.direction {
                    Direction::Horizontal => size.0,
                    Direction::Vertical => size.1,
                }
            };
            used_space += basis;
            flex_grow_sum += child.flex_grow;
        }
        
        let remaining = (available - used_space).max(0.0);
        let grow_per_unit = if flex_grow_sum > 0.0 { remaining / flex_grow_sum } else { 0.0 };
        
        let mut pos = match self.direction {
            Direction::Horizontal => inner_rect.x,
            Direction::Vertical => inner_rect.y,
        };
        
        for child in &self.children {
            let basis = if child.flex_basis > 0.0 { child.flex_basis } else {
                let size = child.widget.measure(ctx, Constraints::default());
                match self.direction {
                    Direction::Horizontal => size.0,
                    Direction::Vertical => size.1,
                }
            };
            
            let grow = child.flex_grow * grow_per_unit;
            let size = basis + grow;
            
            let child_rect = match self.direction {
                Direction::Horizontal => {
                    let h = match child.align_self.unwrap_or(self.align_items) {
                        Align::Stretch => inner_rect.h,
                        _ => {
                            let sz = child.widget.measure(ctx, Constraints::default());
                            sz.1.min(inner_rect.h)
                        }
                    };
                    let y = match child.align_self.unwrap_or(self.align_items) {
                        Align::Start => inner_rect.y,
                        Align::Center => inner_rect.y + (inner_rect.h - h) * 0.5,
                        Align::End => inner_rect.y + inner_rect.h - h,
                        Align::Stretch => inner_rect.y,
                        _ => inner_rect.y,
                    };
                    Rect::new(pos, y, size, h)
                }
                Direction::Vertical => {
                    let w = match child.align_self.unwrap_or(self.align_items) {
                        Align::Stretch => inner_rect.w,
                        _ => {
                            let sz = child.widget.measure(ctx, Constraints::default());
                            sz.0.min(inner_rect.w)
                        }
                    };
                    let x = match child.align_self.unwrap_or(self.align_items) {
                        Align::Start => inner_rect.x,
                        Align::Center => inner_rect.x + (inner_rect.w - w) * 0.5,
                        Align::End => inner_rect.x + inner_rect.w - w,
                        Align::Stretch => inner_rect.x,
                        _ => inner_rect.x,
                    };
                    Rect::new(x, pos, w, size)
                }
            };
            
            results.push(LayoutResult { rect: child_rect, size: (child_rect.w, child_rect.h) });
            pos += size + self.gap;
        }
        
        results
    }
}

/// Grid container
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
    pub gap: f32,
    pub children: Vec<Box<dyn Widget>>,
}

impl Grid {
    pub fn new(columns: usize) -> Self {
        Self { columns, rows: 0, gap: 0.0, children: Vec::new() }
    }

    pub fn child(mut self, widget: Box<dyn Widget>) -> Self {
        self.children.push(widget);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self { self.gap = gap; self }

    pub fn layout(&self, ctx: &LayoutContext, rect: Rect) -> Vec<LayoutResult> {
        let mut results = Vec::new();
        let cell_w = (rect.w - self.gap * (self.columns - 1) as f32) / self.columns as f32;
        let cell_h = (rect.h - self.gap * (self.rows - 1) as f32) / self.rows as f32;
        
        for (i, child) in self.children.iter().enumerate() {
            let col = i % self.columns;
            let row = i / self.columns;
            let x = rect.x + col as f32 * (cell_w + self.gap);
            let y = rect.y + row as f32 * (cell_h + self.gap);
            let child_rect = Rect::new(x, y, cell_w, cell_h);
            results.push(LayoutResult { rect: child_rect, size: (cell_w, cell_h) });
        }
        
        results
    }
}

/// Widget trait
pub trait Widget: Send + Sync {
    fn measure(&self, ctx: &LayoutContext, constraints: Constraints) -> (f32, f32);
    fn layout(&mut self, ctx: &LayoutContext, rect: Rect);
    fn draw(&self, ctx: &DrawContext, rect: Rect);
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

struct EmptyWidget;
impl Widget for EmptyWidget {
    fn measure(&self, _ctx: &LayoutContext, _constraints: Constraints) -> (f32, f32) { (0.0, 0.0) }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, _ctx: &DrawContext, _rect: Rect) {}
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool { false }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Draw context
pub struct DrawContext<'a> {
    pub theme: &'a Theme,
    pub renderer: &'a mut crate::gpu::renderer::GpuRenderer,
    pub text_renderer: &'a mut TextRenderer,
}

/// Text renderer (would use GPU text)
pub struct TextRenderer {
    // Would contain font atlas, etc.
}

impl TextRenderer {
    pub fn new() -> Self { Self {} }
    pub fn draw_text(&mut self, text: &str, pos: Point2, size: f32, color: [f32; 4]) {}
    pub fn measure_text(&self, text: &str, size: f32) -> (f32, f32) { (text.len() as f32 * size * 0.6, size) }
}

/// Input context
pub struct InputContext<'a> {
    pub mouse_pos: Point2,
    pub mouse_down: bool,
    pub key_modifiers: KeyModifiers,
}

/// Input events
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    MouseMove(Point2),
    MouseDown(Point2, MouseButton),
    MouseUp(Point2, MouseButton),
    Click(Point2),
    DoubleClick(Point2),
    KeyDown(Key),
    KeyUp(Key),
    TextInput(String),
    Scroll(f32, f32),
    FocusGained,
    FocusLost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton { Left, Right, Middle }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key { Escape, Enter, Tab, Backspace, Delete, Left, Right, Up, Down, Home, End, PageUp, PageDown, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, Space, Shift, Ctrl, Alt, Meta }

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyModifiers { pub shift: bool, pub ctrl: bool, pub alt: bool, pub meta: bool }