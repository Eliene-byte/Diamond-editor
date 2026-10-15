//! Professional editing operations - ripple, roll, slip, slide, etc.

use crate::core::id::{TrackId, TrackItemId, ClipId, TransitionId};
use crate::core::time::{Frame, FrameRange, Duration};
use crate::core::types::{EditOperation, TrimEdge, TrackType};
use crate::core::error::{Result, TimelineError};
use crate::timeline::clip::{Timeline, TrackItems, TrackItemsMut};
use crate::timeline::track::{TrackContainer, TrackItem, TrackItemKey};
use std::collections::HashMap;

/// Edit modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    /// Normal - overwrite, leave gaps
    Normal,
    /// Ripple - close gaps, shift subsequent clips
    Ripple,
    /// Roll - adjust edit point between two clips
    Roll,
    /// Slip - move source in/out points, keep timeline position
    Slip,
    /// Slide - move timeline position, keep source in/out
    Slide,
    /// Replace - replace clip maintaining position
    Replace,
    /// Fit to fill - change speed to fit duration
    FitToFill,
}

/// Edit tool state
#[derive(Debug, Clone, Default)]
pub struct EditState {
    pub mode: EditMode,
    pub active_edge: Option<TrimEdge>,
    pub hover_item: Option<TrackItemId>,
    pub hover_track: Option<TrackId>,
    pub drag_start: Option<Frame>,
    pub drag_item: Option<TrackItemId>,
    pub drag_offset: Frame,
    pub selection_rect: Option<FrameRange>,
    pub multi_select: bool,
}

/// Editing operations
pub struct Editor {
    pub state: EditState,
    pub snap_enabled: bool,
    pub snap_threshold: f32,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            state: EditState::default(),
            snap_enabled: true,
            snap_threshold: 10.0,
        }
    }
}

impl Editor {
    /// Ripple trim - shifts all subsequent clips
    pub fn ripple_trim(
        &self,
        timeline: &mut Timeline,
        item_id: TrackItemId,
        edge: TrimEdge,
        new_position: Frame,
    ) -> Result<EditOperation> {
        let item = timeline.get_item(item_id)
            .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;
        
        let track_id = item.track_id;
        let old_position = match edge {
            TrimEdge::In => item.timeline_in(),
            TrimEdge::Out => item.timeline_out(),
        };
        
        let delta = new_position - old_position;
        if delta == Frame::ZERO { return Ok(EditOperation::TrimClip { 
            clip_id: item_id, edge, old: old_position, new: new_position 
        }); }
        
        // Check if trim is valid
        if edge == TrimEdge::In && new_position >= item.timeline_out() {
            return Err(TimelineError::InvalidTrim("In point cannot be at or after out point".into()).into());
        }
        if edge == TrimEdge::Out && new_position <= item.timeline_in() {
            return Err(TimelineError::InvalidTrim("Out point cannot be at or before in point".into()).into());
        }
        
        // Get all items after the edit point on the same track
        let mut affected = Vec::new();
        if let Some(track) = timeline.get_track(track_id) {
            let start_search = match edge {
                TrimEdge::In => item.timeline_in(),
                TrimEdge::Out => item.timeline_out(),
            };
            
            for (_, key) in track.items.range(start_search..) {
                if let Some(other) = timeline.items.get(key) {
                    if other.id != item_id {
                        affected.push(other.id);
                    }
                }
            }
        }
        
        // Apply trim to target item
        timeline.trim_clip(item_id, edge, new_position)?;
        
        // Ripple subsequent items
        for affected_id in affected {
            if let Some(affected_item) = timeline.get_item_mut(affected_id) {
                affected_item.timeline_range = affected_item.timeline_range.offset(delta);
                // Update track index
                if let Some(track) = timeline.get_track_mut(track_id) {
                    track.items.remove(&(affected_item.timeline_in() - delta));
                    track.items.insert(affected_item.timeline_in(), affected_item.key);
                }
            }
        }
        
        timeline.update_duration();
        timeline.dirty = true;
        
        Ok(EditOperation::TrimClip { clip_id: item_id, edge, old: old_position, new: new_position })
    }

    /// Ripple delete - removes clip and closes gap
    pub fn ripple_delete(&self, timeline: &mut Timeline, item_id: TrackItemId) -> Result<EditOperation> {
        let item = timeline.get_item(item_id)
            .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;
        
        let track_id = item.track_id;
        let gap_duration = item.timeline_duration();
        let gap_start = item.timeline_in();
        
        // Remove the clip
        timeline.remove_clip(item_id);
        
        // Shift subsequent clips left
        if let Some(track) = timeline.get_track(track_id) {
            let mut to_shift = Vec::new();
            for (_, key) in track.items.range(gap_start..) {
                if let Some(other) = timeline.items.get(key) {
                    to_shift.push(other.id);
                }
            }
            
            for affected_id in to_shift {
                if let Some(affected_item) = timeline.get_item_mut(affected_id) {
                    affected_item.timeline_range = FrameRange::new(
                        affected_item.timeline_in() - gap_duration,
                        affected_item.timeline_out() - gap_duration,
                    );
                    if let Some(track) = timeline.get_track_mut(track_id) {
                        track.items.remove(&(affected_item.timeline_in() + gap_duration));
                        track.items.insert(affected_item.timeline_in(), affected_item.key);
                    }
                }
            }
        }
        
        timeline.update_duration();
        timeline.dirty = true;
        
        Ok(EditOperation::RemoveClip { clip_id: item_id })
    }

    /// Ripple insert - opens space for new clip
    pub fn ripple_insert(
        &self,
        timeline: &mut Timeline,
        track_id: TrackId,
        position: Frame,
        duration: Frame,
    ) -> Result<()> {
        if let Some(track) = timeline.get_track(track_id) {
            let mut to_shift = Vec::new();
            for (_, key) in track.items.range(position..) {
                if let Some(item) = timeline.items.get(key) {
                    to_shift.push(item.id);
                }
            }
            
            for affected_id in to_shift {
                if let Some(affected_item) = timeline.get_item_mut(affected_id) {
                    affected_item.timeline_range = FrameRange::new(
                        affected_item.timeline_in() + duration,
                        affected_item.timeline_out() + duration,
                    );
                    if let Some(track) = timeline.get_track_mut(track_id) {
                        track.items.remove(&(affected_item.timeline_in() - duration));
                        track.items.insert(affected_item.timeline_in(), affected_item.key);
                    }
                }
            }
        }
        
        timeline.update_duration();
        timeline.dirty = true;
        Ok(())
    }

    /// Roll edit - adjusts edit point between two adjacent clips
    pub fn roll_edit(
        &self,
        timeline: &mut Timeline,
        left_item_id: TrackItemId,
        right_item_id: TrackItemId,
        new_edit_point: Frame,
    ) -> Result<EditOperation> {
        let left = timeline.get_item(left_item_id)
            .ok_or(TimelineError::ClipNotFound(left_item_id.to_string()))?;
        let right = timeline.get_item(right_item_id)
            .ok_or(TimelineError::ClipNotFound(right_item_id.to_string()))?;
        
        // Must be adjacent
        if left.timeline_out() != right.timeline_in() {
            return Err(TimelineError::InvalidOperation("Clips must be adjacent for roll edit".into()).into());
        }
        if left.track_id != right.track_id {
            return Err(TimelineError::InvalidOperation("Clips must be on same track".into()).into());
        }
        if left.clip_id != right.clip_id {
            return Err(TimelineError::InvalidOperation("Roll edit only works on same source clip".into()).into());
        }
        
        // Check bounds
        if new_edit_point <= left.timeline_in() || new_edit_point >= right.timeline_out() {
            return Err(TimelineError::InvalidOperation("New edit point out of bounds".into()).into());
        }
        
        let left_source_duration = new_edit_point - left.timeline_in();
        let right_source_duration = right.timeline_out() - new_edit_point;
        
        // Update left item
        if let Some(left_mut) = timeline.get_item_mut(left_item_id) {
            left_mut.timeline_range = FrameRange::new(left_mut.timeline_in(), new_edit_point);
            left_mut.source_range = FrameRange::new(left_mut.source_in(), left_mut.source_in() + left_source_duration);
        }
        
        // Update right item
        if let Some(right_mut) = timeline.get_item_mut(right_item_id) {
            right_mut.timeline_range = FrameRange::new(new_edit_point, right_mut.timeline_out());
            right_mut.source_range = FrameRange::new(right_mut.source_out() - right_source_duration, right_mut.source_out());
            // Update track index
            if let Some(track) = timeline.get_track_mut(left.track_id) {
                track.items.remove(&right.timeline_in());
                track.items.insert(new_edit_point, right_mut.key);
            }
        }
        
        timeline.dirty = true;
        
        Ok(EditOperation::MoveClip { 
            clip_id: left_item_id, 
            from: left.timeline_out(), 
            to: new_edit_point, 
            track: left.track_id 
        })
    }

    /// Slip edit - move source in/out, keep timeline position
    pub fn slip_edit(
        &self,
        timeline: &mut Timeline,
        item_id: TrackItemId,
        source_delta: Frame,
    ) -> Result<EditOperation> {
        let item = timeline.get_item(item_id)
            .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;
        
        let clip = timeline.project_id; // Would need clip metadata
        let source_duration = item.source_duration();
        let timeline_duration = item.timeline_duration();
        
        let new_source_in = item.source_in() + source_delta;
        let new_source_out = new_source_in + source_duration;
        
        // Check bounds (would need clip duration)
        // For now, allow any slip
        
        if let Some(item_mut) = timeline.get_item_mut(item_id) {
            item_mut.source_range = FrameRange::new(new_source_in, new_source_out);
        }
        
        timeline.dirty = true;
        
        Ok(EditOperation::MoveClip { 
            clip_id: item_id, 
            from: item.timeline_in(), 
            to: item.timeline_in(), 
            track: item.track_id 
        })
    }

    /// Slide edit - move timeline position, keep source in/out
    pub fn slide_edit(
        &self,
        timeline: &mut Timeline,
        item_id: TrackItemId,
        timeline_delta: Frame,
    ) -> Result<EditOperation> {
        let item = timeline.get_item(item_id)
            .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;
        
        let track_id = item.track_id;
        let new_position = item.timeline_in() + timeline_delta;
        
        if new_position < Frame::ZERO {
            return Err(TimelineError::InvalidOperation("Cannot slide before timeline start".into()).into());
        }
        
        // Check for collisions
        if let Some(track) = timeline.get_track(track_id) {
            for (_, key) in track.items.range(..) {
                if let Some(other) = timeline.items.get(key) {
                    if other.id != item_id {
                        let other_range = other.timeline_range;
                        let new_range = FrameRange::new(new_position, new_position + item.timeline_duration());
                        if other_range.intersects(new_range) {
                            return Err(TimelineError::OverlapNotAllowed("Slide would cause overlap".into()).into());
                        }
                    }
                }
            }
        }
        
        // Move item
        timeline.move_clip(item_id, new_position, None)?;
        
        Ok(EditOperation::MoveClip { 
            clip_id: item_id, 
            from: item.timeline_in(), 
            to: new_position, 
            track: track_id 
        })
    }

    /// Replace edit - replace clip maintaining timeline position
    pub fn replace_edit(
        &self,
        timeline: &mut Timeline,
        pool: &crate::media::pool::MediaPool,
        old_item_id: TrackItemId,
        new_clip_id: ClipId,
    ) -> Result<EditOperation> {
        let old_item = timeline.get_item(old_item_id)
            .ok_or(TimelineError::ClipNotFound(old_item_id.to_string()))?;
        
        let new_clip = pool.get_clip(new_clip_id)
            .ok_or(TimelineError::ClipNotFound(new_clip_id.to_string()))?;
        
        let track_id = old_item.track_id;
        let timeline_position = old_item.timeline_in();
        
        // Remove old clip
        timeline.remove_clip(old_item_id);
        
        // Add new clip at same position
        timeline.add_clip(pool, new_clip_id, track_id, timeline_position)?;
        
        Ok(EditOperation::RemoveClip { clip_id: old_item_id })
    }

    /// Fit to fill - change clip speed to fill duration
    pub fn fit_to_fill(
        &self,
        timeline: &mut Timeline,
        item_id: TrackItemId,
        target_duration: Frame,
    ) -> Result<EditOperation> {
        let item = timeline.get_item(item_id)
            .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;
        
        let source_duration = item.source_duration();
        if source_duration == Frame::ZERO {
            return Err(TimelineError::InvalidOperation("Source duration is zero".into()).into());
        }
        
        let speed = source_duration.0 as f32 / target_duration.0 as f32;
        
        if let Some(item_mut) = timeline.get_item_mut(item_id) {
            item_mut.speed = speed;
            item_mut.timeline_range = FrameRange::new(item_mut.timeline_in(), item_mut.timeline_in() + target_duration);
        }
        
        timeline.dirty = true;
        
        Ok(EditOperation::MoveClip { 
            clip_id: item_id, 
            from: item.timeline_in(), 
            to: item.timeline_in(), 
            track: item.track_id 
        })
    }

    /// Add transition between two clips
    pub fn add_transition(
        &self,
        timeline: &mut Timeline,
        left_item_id: TrackItemId,
        right_item_id: TrackItemId,
        transition_type: crate::core::types::EffectType,
        duration: Frame,
    ) -> Result<TransitionId> {
        let left = timeline.get_item(left_item_id)
            .ok_or(TimelineError::ClipNotFound(left_item_id.to_string()))?;
        let right = timeline.get_item(right_item_id)
            .ok_or(TimelineError::ClipNotFound(right_item_id.to_string()))?;
        
        if left.track_id != right.track_id {
            return Err(TimelineError::InvalidOperation("Clips must be on same track".into()).into());
        }
        if left.timeline_out() != right.timeline_in() {
            return Err(TimelineError::InvalidOperation("Clips must be adjacent".into()).into());
        }
        if left.clip_id == right.clip_id {
            return Err(TimelineError::InvalidOperation("Cannot add transition within same clip".into()).into());
        }
        
        let transition_id = TransitionId::generate();
        
        // Apply to items
        if let Some(left_mut) = timeline.get_item_mut(left_item_id) {
            left_mut.transitions.1 = Some(transition_id);
        }
        if let Some(right_mut) = timeline.get_item_mut(right_item_id) {
            right_mut.transitions.0 = Some(transition_id);
        }
        
        timeline.dirty = true;
        
        Ok(transition_id)
    }

    /// Remove transition
    pub fn remove_transition(
        &self,
        timeline: &mut Timeline,
        transition_id: TransitionId,
    ) -> Result<()> {
        for item in timeline.items.values_mut() {
            if item.transitions.0 == Some(transition_id) {
                item.transitions.0 = None;
            }
            if item.transitions.1 == Some(transition_id) {
                item.transitions.1 = None;
            }
        }
        timeline.dirty = true;
        Ok(())
    }

    /// Get snap points for a frame
    pub fn snap_points(&self, timeline: &Timeline, frame: Frame, track_id: Option<TrackId>) -> Vec<Frame> {
        let mut points = Vec::new();
        
        // Timeline start/end
        points.push(Frame::ZERO);
        points.push(timeline.duration);
        
        // Markers
        for marker in timeline.markers.values() {
            points.push(marker.position);
            if let Some(dur) = marker.duration {
                points.push(marker.position + dur);
            }
        }
        
        // Track items
        let tracks = if let Some(tid) = track_id {
            vec![tid]
        } else {
            timeline.tracks.video_tracks.iter().map(|t| t.id)
                .chain(timeline.tracks.audio_tracks.iter().map(|t| t.id))
                .collect()
        };
        
        for tid in tracks {
            if let Some(track) = timeline.get_track(tid) {
                for (_, key) in track.items() {
                    if let Some(item) = timeline.items.get(key) {
                        points.push(item.timeline_in());
                        points.push(item.timeline_out());
                    }
                }
            }
        }
        
        points.sort();
        points.dedup();
        points
    }

    /// Snap frame to nearest point
    pub fn snap(&self, timeline: &Timeline, frame: Frame, track_id: Option<TrackId>) -> Frame {
        if !self.snap_enabled { return frame; }
        
        let points = self.snap_points(timeline, frame, track_id);
        let threshold = Frame((self.snap_threshold * timeline.settings.frame_rate.fps_f32()) as i64);
        
        for point in points {
            if (frame.0 - point.0).abs() <= threshold.0 {
                return point;
            }
        }
        
        frame
    }
}

impl Timeline {
    fn get_track(&self, track_id: TrackId) -> Option<&dyn TrackItems> {
        self.tracks.get_video_track(track_id)
            .map(|t| t as &dyn TrackItems)
            .or_else(|| self.tracks.get_audio_track(track_id).map(|t| t as &dyn TrackItems))
            .or_else(|| self.tracks.get_subtitle_track(track_id).map(|t| t as &dyn TrackItems))
    }

    fn get_track_mut(&mut self, track_id: TrackId) -> Option<&mut dyn TrackItemsMut> {
        self.tracks.get_video_track_mut(track_id)
            .map(|t| t as &mut dyn TrackItemsMut)
            .or_else(|| self.tracks.get_audio_track_mut(track_id).map(|t| t as &mut dyn TrackItemsMut))
            .or_else(|| self.tracks.get_subtitle_track_mut(track_id).map(|t| t as &mut dyn TrackItemsMut))
    }
}