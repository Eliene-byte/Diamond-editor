//! Selection management for timeline

use crate::core::id::{TrackId, TrackItemId, MarkerId};
use crate::core::time::{Frame, FrameRange};
use crate::timeline::clip::Timeline;
use std::collections::HashSet;

/// Selection state
#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub items: HashSet<TrackItemId>,
    pub range: Option<FrameRange>,
    pub tracks: HashSet<TrackId>,
    pub markers: HashSet<MarkerId>,
    pub anchor: Option<Frame>, // For range selection
}

impl Selection {
    pub fn new() -> Self { Self::default() }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.range.is_none() && self.markers.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.range = None;
        self.tracks.clear();
        self.markers.clear();
        self.anchor = None;
    }

    pub fn select_item(&mut self, item_id: TrackItemId, add: bool) {
        if !add { self.items.clear(); }
        self.items.insert(item_id);
    }

    pub fn deselect_item(&mut self, item_id: TrackItemId) {
        self.items.remove(&item_id);
    }

    pub fn select_items(&mut self, item_ids: &[TrackItemId], add: bool) {
        if !add { self.items.clear(); }
        self.items.extend(item_ids.iter().copied());
    }

    pub fn select_all(&mut self, timeline: &Timeline) {
        self.items = timeline.items.keys().copied().collect();
    }

    pub fn select_track(&mut self, track_id: TrackId, add: bool) {
        if !add { self.tracks.clear(); }
        self.tracks.insert(track_id);
    }

    pub fn deselect_track(&mut self, track_id: TrackId) {
        self.tracks.remove(&track_id);
    }

    pub fn set_range(&mut self, range: FrameRange) {
        self.range = Some(range);
    }

    pub fn start_range(&mut self, anchor: Frame) {
        self.anchor = Some(anchor);
        self.range = Some(FrameRange::new(anchor, anchor));
    }

    pub fn update_range(&mut self, current: Frame) {
        if let Some(anchor) = self.anchor {
            let start = anchor.min(current);
            let end = anchor.max(current);
            self.range = Some(FrameRange::new(start, end));
        }
    }

    pub fn end_range(&mut self) {
        self.anchor = None;
    }

    pub fn select_marker(&mut self, marker_id: MarkerId, add: bool) {
        if !add { self.markers.clear(); }
        self.markers.insert(marker_id);
    }

    pub fn has_item(&self, item_id: TrackItemId) -> bool {
        self.items.contains(&item_id)
    }

    pub fn has_track(&self, track_id: TrackId) -> bool {
        self.tracks.contains(&track_id)
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn get_items(&self) -> Vec<TrackItemId> {
        self.items.iter().copied().collect()
    }

    pub fn get_tracks(&self) -> Vec<TrackId> {
        self.tracks.iter().copied().collect()
    }

    pub fn get_range(&self) -> Option<FrameRange> {
        self.range
    }

    /// Get all items within selection range
    pub fn items_in_range(&self, timeline: &Timeline) -> Vec<TrackItemId> {
        if let Some(range) = self.range {
            timeline.items_in_range(range).into_iter().map(|i| i.id).collect()
        } else {
            Vec::new()
        }
    }

    /// Select all items in range
    pub fn select_range(&mut self, timeline: &Timeline, range: FrameRange) {
        self.range = Some(range);
        self.items = timeline.items_in_range(range).into_iter().map(|i| i.id).collect();
    }
}

/// Selection modifier keys
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectionModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub cmd: bool,
}

impl SelectionModifiers {
    pub fn add_to_selection(&self) -> bool { self.shift }
    pub fn toggle_selection(&self) -> bool { self.ctrl }
    pub fn range_select(&self) -> bool { self.shift }
    pub fn duplicate(&self) -> bool { self.alt }
    pub fn ripple(&self) -> bool { self.shift }
}

/// Selection tool
pub struct SelectionTool {
    pub selection: Selection,
    pub modifiers: SelectionModifiers,
    pub drag_start: Option<Frame>,
    pub drag_current: Option<Frame>,
    pub drag_track: Option<TrackId>,
    pub marquee_rect: Option<MarqueeRect>,
}

#[derive(Debug, Clone, Copy)]
pub struct MarqueeRect {
    pub start_frame: Frame,
    pub end_frame: Frame,
    pub start_track: TrackId,
    pub end_track: TrackId,
}

impl SelectionTool {
    pub fn new() -> Self {
        Self {
            selection: Selection::new(),
            modifiers: SelectionModifiers::default(),
            drag_start: None,
            drag_current: None,
            drag_track: None,
            marquee_rect: None,
        }
    }

    pub fn set_modifiers(&mut self, modifiers: SelectionModifiers) {
        self.modifiers = modifiers;
    }

    pub fn start_drag(&mut self, frame: Frame, track: TrackId, item: Option<TrackItemId>) {
        self.drag_start = Some(frame);
        self.drag_current = Some(frame);
        self.drag_track = Some(track);
        
        if let Some(item_id) = item {
            if self.modifiers.toggle_selection() {
                if self.selection.has_item(item_id) {
                    self.selection.deselect_item(item_id);
                } else {
                    self.selection.select_item(item_id, self.modifiers.add_to_selection());
                }
            } else if self.modifiers.range_select() {
                self.selection.select_item(item_id, self.modifiers.add_to_selection());
            } else {
                self.selection.select_item(item_id, false);
            }
        }
    }

    pub fn update_drag(&mut self, frame: Frame, track: TrackId) {
        self.drag_current = Some(frame);
        
        if self.modifiers.range_select() && self.drag_start.is_some() {
            let start = self.drag_start.unwrap();
            let start_track = self.drag_track.unwrap();
            
            self.marquee_rect = Some(MarqueeRect {
                start_frame: start.min(frame),
                end_frame: start.max(frame),
                start_track: start_track.min(track),
                end_track: start_track.max(track),
            });
        }
    }

    pub fn end_drag(&mut self, timeline: &Timeline) -> DragResult {
        let result = if let Some(marquee) = self.marquee_rect {
            // Marquee selection
            self.select_marquee(timeline, marquee);
            DragResult::MarqueeSelect
        } else if let Some(start) = self.drag_start {
            if let Some(current) = self.drag_current {
                let delta = current - start;
                if delta != Frame::ZERO {
                    // Move selected items
                    self.move_selection(timeline, delta);
                    DragResult::Move { delta }
                } else {
                    DragResult::Click
                }
            } else {
                DragResult::Click
            }
        } else {
            DragResult::None
        };
        
        self.drag_start = None;
        self.drag_current = None;
        self.drag_track = None;
        self.marquee_rect = None;
        
        result
    }

    fn select_marquee(&mut self, timeline: &Timeline, marquee: MarqueeRect) {
        let add = self.modifiers.add_to_selection();
        if !add { self.selection.clear(); }
        
        // Select tracks in range
        let track_ids = self.tracks_in_range(timeline, marquee.start_track, marquee.end_track);
        for track_id in track_ids {
            self.selection.select_track(track_id, true);
        }
        
        // Select items in range
        for item in timeline.items_in_range(FrameRange::new(marquee.start_frame, marquee.end_frame)) {
            if track_ids.contains(&item.track_id) {
                self.selection.select_item(item.id, true);
            }
        }
    }

    fn tracks_in_range(&self, timeline: &Timeline, start: TrackId, end: TrackId) -> Vec<TrackId> {
        let mut tracks = Vec::new();
        
        // Video tracks
        for track in &timeline.tracks.video_tracks {
            if track.id >= start && track.id <= end {
                tracks.push(track.id);
            }
        }
        // Audio tracks
        for track in &timeline.tracks.audio_tracks {
            if track.id >= start && track.id <= end {
                tracks.push(track.id);
            }
        }
        // Subtitle tracks
        for track in &timeline.tracks.subtitle_tracks {
            if track.id >= start && track.id <= end {
                tracks.push(track.id);
            }
        }
        
        tracks
    }

    fn move_selection(&self, timeline: &Timeline, delta: Frame) {
        for item_id in self.selection.get_items() {
            if let Some(item) = timeline.get_item(item_id) {
                if !item.locked {
                    let new_pos = item.timeline_in() + delta;
                    if new_pos >= Frame::ZERO {
                        // Would move - actual move happens in editor
                    }
                }
            }
        }
    }

    pub fn cancel_drag(&mut self) {
        self.drag_start = None;
        self.drag_current = None;
        self.drag_track = None;
        self.marquee_rect = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragResult {
    None,
    Click,
    Move { delta: Frame },
    MarqueeSelect,
}

/// Keyboard navigation for selection
pub struct SelectionNavigation {
    pub timeline: Option<Timeline>,
}

impl SelectionNavigation {
    pub fn new() -> Self { Self { timeline: None } }

    pub fn set_timeline(&mut self, timeline: Timeline) { self.timeline = Some(timeline); }

    pub fn select_next_item(&self, wrap: bool) -> Option<TrackItemId> {
        // Implementation would find next item after current selection
        None
    }

    pub fn select_prev_item(&self, wrap: bool) -> Option<TrackItemId> { None }

    pub fn select_next_track(&self, track_type: crate::core::types::TrackType) -> Option<TrackId> { None }

    pub fn select_prev_track(&self, track_type: crate::core::types::TrackType) -> Option<TrackId> { None }

    pub fn extend_selection_next(&self) -> Option<TrackItemId> { None }

    pub fn extend_selection_prev(&self) -> Option<TrackItemId> { None }

    pub fn select_all_on_track(&self, track_id: TrackId) -> Vec<TrackItemId> { Vec::new() }

    pub fn deselect_all(&self) {}
}