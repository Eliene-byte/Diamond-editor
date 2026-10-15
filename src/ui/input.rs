//! Input handling for the custom immediate-mode UI.

use crate::core::math::Point2;
use crate::ui::layout::{InputEvent, Key, KeyModifiers, MouseButton};
use std::collections::HashSet;

/// Raw input state accumulated from window events, consumed once per frame.
#[derive(Debug, Default, Clone)]
pub struct InputState {
    pub mouse_pos: Point2,
    pub mouse_delta: Point2,
    pub scroll_delta: (f32, f32),
    pub pressed_buttons: HashSet<MouseButton>,
    pub pressed_keys: HashSet<Key>,
    pub modifiers: KeyModifiers,
    pub events: Vec<InputEvent>,
    pub text_buffer: String,
    pub focus: bool,
    pub cursor_visible: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            cursor_visible: true,
            ..Default::default()
        }
    }

    /// Record a window event, updating continuous state and queueing discrete events.
    pub fn handle_event(&mut self, event: InputEvent) {
        match event.clone() {
            InputEvent::MouseMove(pos) => {
                self.mouse_delta = Point2::new(pos.x - self.mouse_pos.x, pos.y - self.mouse_pos.y);
                self.mouse_pos = pos;
            }
            InputEvent::MouseDown(pos, btn) => {
                self.mouse_pos = pos;
                self.pressed_buttons.insert(btn);
            }
            InputEvent::MouseUp(pos, btn) => {
                self.mouse_pos = pos;
                self.pressed_buttons.remove(&btn);
            }
            InputEvent::Scroll(dx, dy) => {
                self.scroll_delta.0 += dx;
                self.scroll_delta.1 += dy;
            }
            InputEvent::KeyDown(key) => {
                self.pressed_keys.insert(key);
            }
            InputEvent::KeyUp(key) => {
                self.pressed_keys.remove(&key);
            }
            InputEvent::TextInput(text) => {
                self.text_buffer.push_str(&text);
            }
            InputEvent::FocusGained => self.focus = true,
            InputEvent::FocusLost => {
                self.focus = false;
                self.pressed_buttons.clear();
                self.pressed_keys.clear();
            }
            other => {}
        }
        self.events.push(event);
    }

    pub fn is_mouse_down(&self, btn: MouseButton) -> bool {
        self.pressed_buttons.contains(&btn)
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn shift(&self) -> bool {
        self.modifiers.shift
    }
    pub fn ctrl(&self) -> bool {
        self.modifiers.ctrl
    }
    pub fn alt(&self) -> bool {
        self.modifiers.alt
    }
    pub fn meta(&self) -> bool {
        self.modifiers.meta
    }

    /// Take ownership of queued discrete events, clearing the queue.
    pub fn drain_events(&mut self) -> Vec<InputEvent> {
        std::mem::take(&mut self.events)
    }

    /// Take and clear the accumulated text input.
    pub fn take_text(&mut self) -> String {
        std::mem::take(&mut self.text_buffer)
    }

    /// Reset per-frame accumulators. Discrete state (pressed keys/buttons) persists.
    pub fn end_frame(&mut self) {
        self.mouse_delta = Point2::ZERO;
        self.scroll_delta = (0.0, 0.0);
        self.events.clear();
        self.text_buffer.clear();
    }

    /// True if any modifier key is currently held.
    pub fn any_modifier(&self) -> bool {
        self.modifiers.shift || self.modifiers.ctrl || self.modifiers.alt || self.modifiers.meta
    }
}

/// Drag state tracked between mouse-down and mouse-up.
#[derive(Debug, Clone, Copy)]
pub struct DragState {
    pub start: Point2,
    pub current: Point2,
    pub button: MouseButton,
    pub started_moved: bool,
}

impl DragState {
    pub fn begin(start: Point2, button: MouseButton) -> Self {
        Self {
            start,
            current: start,
            button,
            started_moved: false,
        }
    }

    pub fn update(&mut self, pos: Point2, drag_threshold: f32) {
        self.current = pos;
        let dx = pos.x - self.start.x;
        let dy = pos.y - self.start.y;
        if dx * dx + dy * dy > drag_threshold * drag_threshold {
            self.started_moved = true;
        }
    }

    pub fn delta(&self) -> Point2 {
        Point2::new(self.current.x - self.start.x, self.current.y - self.start.y)
    }

    pub fn distance(&self) -> f32 {
        self.delta().length()
    }
}

/// Double-click detection with a time + distance threshold.
#[derive(Debug, Clone, Copy)]
pub struct ClickDetector {
    pub last_click_time: f64,
    pub last_click_pos: Point2,
    pub threshold_ms: u64,
    pub threshold_px: f32,
}

impl Default for ClickDetector {
    fn default() -> Self {
        Self {
            last_click_time: -1.0,
            last_click_pos: Point2::ZERO,
            threshold_ms: 400,
            threshold_px: 4.0,
        }
    }
}

impl ClickDetector {
    /// Returns true when this click completes a double-click.
    pub fn register(&mut self, pos: Point2, now_ms: f64) -> bool {
        let within_time = (now_ms - self.last_click_time) < self.threshold_ms as f64;
        let within_dist = {
            let dx = pos.x - self.last_click_pos.x;
            let dy = pos.y - self.last_click_pos.y;
            dx * dx + dy * dy <= self.threshold_px * self.threshold_px
        };

        // A click only counts as the second of a pair if there *was* a first click.
        // `last_click_time` is reset to -1.0 after a completed pair, so a third
        // click does not chain into a second double-click.
        let had_previous = self.last_click_time >= 0.0;
        let is_double = had_previous && within_time && within_dist;

        self.last_click_time = if is_double { -1.0 } else { now_ms };
        self.last_click_pos = pos;

        is_double
    }

    pub fn reset(&mut self) {
        self.last_click_time = -1.0;
        self.last_click_pos = Point2::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_reports_movement_after_threshold() {
        let mut d = DragState::begin(Point2::new(10.0, 10.0), MouseButton::Left);
        assert!(!d.started_moved);

        d.update(Point2::new(11.0, 10.0), 4.0);
        assert!(!d.started_moved, "1px move should stay under threshold");

        d.update(Point2::new(40.0, 10.0), 4.0);
        assert!(d.started_moved, "30px move should exceed threshold");
        assert_eq!(d.delta(), Point2::new(30.0, 0.0));
    }

    #[test]
    fn double_click_requires_both_time_and_distance() {
        let mut c = ClickDetector::default();

        // First click: never a double.
        assert!(!c.register(Point2::new(0.0, 0.0), 1000.0));

        // Quick second click in the same spot: double.
        assert!(c.register(Point2::new(1.0, 0.0), 1100.0));

        // Too slow: not a double.
        assert!(!c.register(Point2::new(0.0, 0.0), 5000.0));
        assert!(!c.register(Point2::new(0.0, 0.0), 9000.0));

        // Too far from the previous click: not a double.
        assert!(!c.register(Point2::new(500.0, 0.0), 9200.0));

        // The click at 9200 is still a valid anchor, so a close, quick
        // follow-up *is* a double - a distance check only rejects one pairing.
        assert!(c.register(Point2::new(501.0, 0.0), 9250.0));

        // A third click in the same spot must not chain into a second double.
        assert!(!c.register(Point2::new(501.0, 0.0), 9300.0));
    }

    #[test]
    fn end_frame_clears_accumulators_but_not_held_keys() {
        let mut s = InputState::new();
        s.handle_event(InputEvent::KeyDown(Key::A));
        s.handle_event(InputEvent::Scroll(0.0, 3.0));
        assert!(s.is_key_down(Key::A));
        assert_eq!(s.scroll_delta.1, 3.0);

        s.end_frame();
        assert!(
            s.is_key_down(Key::A),
            "held key should persist across frames"
        );
        assert_eq!(s.scroll_delta.1, 0.0, "scroll should not accumulate");

        s.handle_event(InputEvent::KeyUp(Key::A));
        assert!(!s.is_key_down(Key::A));
    }
}
