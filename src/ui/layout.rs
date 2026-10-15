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
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }
}

impl Constraints {
    pub fn tight(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }

    pub fn loosen(&self) -> Self {
        Self {
            min_width: 0.0,
            min_height: 0.0,
            ..*self
        }
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
        Self {
            theme,
            constraints: Constraints::default(),
            scale: 1.0,
        }
    }

    /// Estimate the pixel size of `text` at `font_size`.
    ///
    /// There is no font atlas yet, so this uses the same 0.6 em average advance
    /// the rest of the UI assumes for layout. Widgets call this instead of
    /// reaching for a text renderer that layout context does not own.
    pub fn measure_text(&self, text: &str, font_size: f32) -> (f32, f32) {
        (text.chars().count() as f32 * font_size * 0.6, font_size)
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

pub struct FlexChild {
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: f32,
    pub align_self: Option<Align>,
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
    /// Rectangle assigned during the last layout pass, if any.
    pub rect: Option<Rect>,
    pub widget: Box<dyn Widget>,
}

impl Default for FlexChild {
    fn default() -> Self {
        Self {
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: 0.0,
            align_self: None,
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            rect: None,
            widget: Box::new(EmptyWidget),
        }
    }
}

impl FlexChild {
    /// Effective `(grow, shrink, basis)`.
    ///
    /// Weights wrapped around the widget take precedence over the ones set on
    /// this `FlexChild`, because the wrapper is the more specific intent.
    fn spec(&self) -> (f32, f32, f32) {
        if self.widget.as_any().downcast_ref::<Weighted>().is_some() {
            return merged_spec(self.widget.as_ref());
        }
        (self.flex_grow, self.flex_shrink, self.flex_basis)
    }
}

impl Flex {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            align_items: Align::Stretch,
            justify_content: Align::Start,
            gap: 0.0,
            padding: [0.0; 4],
            children: Vec::new(),
        }
    }

    pub fn child(mut self, widget: Box<dyn Widget>) -> Self {
        self.children.push(FlexChild {
            widget,
            ..Default::default()
        });
        self
    }

    /// Set the growth weight of the child that was added most recently.
    ///
    /// Returns `self` unchanged when there are no children, so chained builders
    /// in page layouts do not need to special-case an empty container.
    pub fn flex_grow(mut self, factor: f32) -> Self {
        if let Some(child) = self.children.last_mut() {
            child.flex_grow = factor;
        }
        self
    }

    /// Set the shrink weight of the child that was added most recently.
    pub fn flex_shrink(mut self, factor: f32) -> Self {
        if let Some(child) = self.children.last_mut() {
            child.flex_shrink = factor;
        }
        self
    }

    /// Set the base size of the child that was added most recently, overriding
    /// its measured size.
    pub fn flex_basis(mut self, basis: f32) -> Self {
        if let Some(child) = self.children.last_mut() {
            child.flex_basis = basis;
        }
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
    pub fn padding(mut self, padding: [f32; 4]) -> Self {
        self.padding = padding;
        self
    }
    pub fn align_items(mut self, align: Align) -> Self {
        self.align_items = align;
        self
    }
    pub fn justify_content(mut self, justify: Align) -> Self {
        self.justify_content = justify;
        self
    }

    pub fn layout(&self, ctx: &LayoutContext, rect: Rect) -> Vec<LayoutResult> {
        self.layout_children(ctx, rect)
    }

    /// Compute each child's rectangle, record it on the child so `draw` and
    /// `handle_input` can reuse it, and return the results.
    fn layout_children(&self, ctx: &LayoutContext, rect: Rect) -> Vec<LayoutResult> {
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

        // Calculate flex sizes. A child's weights may be attached either directly on
        // the `FlexChild` (set with `Flex::flex_grow` and friends) or wrapped
        // around the widget (set with `Box<dyn Widget>::flex_grow`); the wrapped
        // form wins, because it is the more specific of the two.
        let mut results = Vec::new();
        let mut flex_grow_sum = 0.0;
        let mut used_space = 0.0;

        for child in &self.children {
            let (grow, _shrink, basis_override) = child.spec();
            let basis = if basis_override > 0.0 {
                basis_override
            } else {
                let size = child.widget.measure(ctx, Constraints::default());
                match self.direction {
                    Direction::Horizontal => size.0,
                    Direction::Vertical => size.1,
                }
            };
            used_space += basis;
            flex_grow_sum += grow;
        }

        let remaining = (available - used_space).max(0.0);
        let grow_per_unit = if flex_grow_sum > 0.0 {
            remaining / flex_grow_sum
        } else {
            0.0
        };

        let mut pos = match self.direction {
            Direction::Horizontal => inner_rect.x,
            Direction::Vertical => inner_rect.y,
        };

        for child in &self.children {
            let (grow_weight, _shrink, basis_override) = child.spec();
            let basis = if basis_override > 0.0 {
                basis_override
            } else {
                let size = child.widget.measure(ctx, Constraints::default());
                match self.direction {
                    Direction::Horizontal => size.0,
                    Direction::Vertical => size.1,
                }
            };

            let grow = grow_weight * grow_per_unit;
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

            results.push(LayoutResult {
                rect: child_rect,
                size: (child_rect.w, child_rect.h),
            });
            pos += size + self.gap;
        }

        results
    }

    /// Assign each child's rectangle and forward the layout to the child itself.
    pub(crate) fn apply_layout(&mut self, ctx: &LayoutContext, rect: Rect) {
        let rects: Vec<Rect> = self
            .layout_children(ctx, rect)
            .into_iter()
            .map(|r| r.rect)
            .collect();

        for (child, child_rect) in self.children.iter_mut().zip(rects) {
            child.rect = Some(child_rect);
            child.widget.layout(ctx, child_rect);
        }
    }
}

/// A container that lays out its children in a row or column, weighted by the
/// `flex_grow` set on each child.
impl Widget for Flex {
    fn measure(&self, ctx: &LayoutContext, constraints: Constraints) -> (f32, f32) {
        let mut cross = 0.0f32;
        let mut main = 0.0f32;
        let mut main_count = 0usize;

        for child in &self.children {
            let (w, h) = child.widget.measure(ctx, constraints);
            let (main_size, cross_size) = match self.direction {
                Direction::Horizontal => (w, h),
                Direction::Vertical => (h, w),
            };
            main += main_size + self.gap;
            main_count += 1;
            cross = cross.max(cross_size);
        }

        if main_count > 0 {
            main -= self.gap;
        }
        let (w, h) = match self.direction {
            Direction::Horizontal => (main, cross),
            Direction::Vertical => (cross, main),
        };
        (
            w + self.padding[1] + self.padding[3],
            h + self.padding[0] + self.padding[2],
        )
    }

    fn layout(&mut self, ctx: &LayoutContext, rect: Rect) {
        self.apply_layout(ctx, rect);
    }

    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        for child in &self.children {
            let child_rect = child.rect.unwrap_or(rect);
            child.widget.draw(ctx, child_rect);
        }
    }

    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        for child in &mut self.children {
            if let Some(rect) = child.rect {
                if !event.hits(rect) {
                    continue;
                }
            }
            if child.widget.handle_input(ctx, event.clone()) {
                return true;
            }
        }
        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
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
        Self {
            columns,
            rows: 0,
            gap: 0.0,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, widget: Box<dyn Widget>) -> Self {
        self.children.push(widget);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

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
            results.push(LayoutResult {
                rect: child_rect,
                size: (cell_w, cell_h),
            });
        }

        results
    }
}

/// Mouse position carried by an input event, if it has one.
impl InputEvent {
    /// The pointer position this event refers to, if any.
    pub fn position(&self) -> Option<Point2> {
        match self {
            InputEvent::MouseMove(p)
            | InputEvent::MouseDown(p, _)
            | InputEvent::MouseUp(p, _)
            | InputEvent::Click(p)
            | InputEvent::DoubleClick(p) => Some(*p),
            _ => None,
        }
    }

    /// True when the event's pointer falls inside `rect`. Events with no
    /// position (key presses, focus changes) always return true so containers do
    /// not swallow keyboard input.
    pub fn hits(&self, rect: Rect) -> bool {
        match self.position() {
            Some(p) => {
                p.x >= rect.x && p.x < rect.x + rect.w && p.y >= rect.y && p.y < rect.y + rect.h
            }
            None => true,
        }
    }
}

/// A widget paired with the flex weights it should be laid out with.
///
/// Page builders write `Box::new(viewer).flex_grow(1.0)`, which reads naturally
/// but cannot mutate the parent `Flex` from inside the call. Wrapping the widget
/// keeps the weights attached to it, and `Flex::layout_children` picks them up
/// through [`FlexSpec`], merging wrappers so `.flex_grow(..).flex_basis(..)`
/// composes instead of replacing.
pub struct Weighted {
    inner: Box<dyn Widget>,
    grow: Option<f32>,
    shrink: Option<f32>,
    basis: Option<f32>,
}

/// Flex weights a widget wants applied by its parent container.
pub trait FlexSpec {
    /// `(grow, shrink, basis)`; `basis` of 0 means "use the measured size".
    fn flex_spec(&self) -> (f32, f32, f32) {
        (0.0, 1.0, 0.0)
    }
}

impl Weighted {
    pub fn new(
        inner: Box<dyn Widget>,
        grow: Option<f32>,
        shrink: Option<f32>,
        basis: Option<f32>,
    ) -> Self {
        Self {
            inner,
            grow,
            shrink,
            basis,
        }
    }

    /// Merge this wrapper's weights with those of any nested wrappers.
    fn merged_spec(&self) -> (f32, f32, f32) {
        let (mut grow, mut shrink, mut basis) = merged_spec(self.inner.as_ref());

        if let Some(v) = self.grow {
            grow = v;
        }
        if let Some(v) = self.shrink {
            shrink = v;
        }
        if let Some(v) = self.basis {
            basis = v;
        }
        (grow, shrink, basis)
    }
}

/// Walk a widget tree of [`Weighted`] wrappers and combine their weights.
/// Non-wrapped widgets contribute the defaults.
fn merged_spec(widget: &dyn Widget) -> (f32, f32, f32) {
    if let Some(weighted) = widget.as_any().downcast_ref::<Weighted>() {
        weighted.merged_spec()
    } else {
        (0.0, 1.0, 0.0)
    }
}

impl FlexSpec for Weighted {
    fn flex_spec(&self) -> (f32, f32, f32) {
        self.merged_spec()
    }
}

impl Widget for Weighted {
    fn measure(&self, ctx: &LayoutContext, constraints: Constraints) -> (f32, f32) {
        self.inner.measure(ctx, constraints)
    }
    fn layout(&mut self, ctx: &LayoutContext, rect: Rect) {
        self.inner.layout(ctx, rect);
    }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        self.inner.draw(ctx, rect);
    }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        self.inner.handle_input(ctx, event)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Builder helpers for weighted children.
///
/// Implemented for `Box<dyn Widget>` as an extension trait rather than an
/// inherent impl, because `Box` is a foreign type and inherent impls on it are
/// rejected.
pub trait WidgetBoxExt {
    /// Grow to take a share of the parent's leftover space.
    fn flex_grow(self, factor: f32) -> Box<dyn Widget>;
    /// Give up space faster than siblings when the parent overflows.
    fn flex_shrink(self, factor: f32) -> Box<dyn Widget>;
    /// Override the measured base size.
    fn flex_basis(self, basis: f32) -> Box<dyn Widget>;
}

impl<T: Widget + 'static> WidgetBoxExt for Box<T> {
    fn flex_grow(self, factor: f32) -> Box<dyn Widget> {
        Box::new(Weighted::new(self, Some(factor), None, None))
    }

    fn flex_shrink(self, factor: f32) -> Box<dyn Widget> {
        Box::new(Weighted::new(self, None, Some(factor), None))
    }

    fn flex_basis(self, basis: f32) -> Box<dyn Widget> {
        Box::new(Weighted::new(self, None, None, Some(basis)))
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
    fn measure(&self, _ctx: &LayoutContext, _constraints: Constraints) -> (f32, f32) {
        (0.0, 0.0)
    }
    fn layout(&mut self, _ctx: &LayoutContext, _rect: Rect) {}
    fn draw(&self, _ctx: &DrawContext, _rect: Rect) {}
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

/// Draw context
pub struct DrawContext<'a> {
    pub theme: &'a Theme,
    pub renderer: &'a mut crate::gpu::renderer::GpuRenderer,
    /// Shared borrow: draw_text only records glyph runs, so widgets can draw
    /// from a &DrawContext without a unique borrow.
    pub text_renderer: &'a TextRenderer,
}

/// Text renderer (would use GPU text)
pub struct TextRenderer {
    // Would contain font atlas, etc.
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {}
    }
    pub fn draw_text(&self, text: &str, pos: Point2, size: f32, color: [f32; 4]) {}
    pub fn measure_text(&self, text: &str, size: f32) -> (f32, f32) {
        (text.len() as f32 * size * 0.6, size)
    }
}

/// Input context
pub struct InputContext {
    pub mouse_pos: Point2,
    pub mouse_down: bool,
    pub key_modifiers: KeyModifiers,
}

/// Input events
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Escape,
    Enter,
    Tab,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    Space,
    Shift,
    Ctrl,
    Alt,
    Meta,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}
