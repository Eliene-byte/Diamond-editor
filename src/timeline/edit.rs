//! Professional editing operations - ripple, roll, slip, slide, etc.

use crate::core::id::{TrackId, TrackItemId, ClipId, TransitionId};
use crate::core::time::{Frame, FrameRange, Duration};
use crate::core::types::{EditOperation, TrimEdge, TrackType};
use crate::core::error::{Result, TimelineError};
use crate::timeline::clip::{Timeline, TrackItems, TrackItemsMut};
use crate::timeline::track::{TrackContainer, TrackItem, TrackItemKey};
use std::collections::HashMap;

/// Edit modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditMode {
    /// Normal - overwrite, leave gaps
    #[default]
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
            item_id: item_id, edge, old: old_position, new: new_position 
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
            
            for (_, key) in track.items().range(start_search..) {
                if let Some(other) = timeline.item_by_key(*key) {
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
            }
            // Re-key the track index after the range moved. Done outside the item
            // borrow above: `reindex_item` needs `&mut Timeline`.
            timeline.reindex_item(track_id, affected_id);
        }
        
        timeline.update_duration();
        timeline.dirty = true;
        
        Ok(EditOperation::TrimClip { item_id: item_id, edge, old: old_position, new: new_position })
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
        
        // Shift subsequent clips left. Collect the ids first so the immutable track
        // borrow does not overlap the later item mutations.
        let to_shift: Vec<TrackItemId> = {
            let mut ids = Vec::new();
            if let Some(track) = timeline.get_track(track_id) {
                for (_, key) in track.items().range(gap_start..) {
                    if let Some(other) = timeline.item_by_key(*key) {
                        ids.push(other.id);
                    }
                }
            }
            ids
        };

        for affected_id in to_shift {
            if let Some(affected_item) = timeline.get_item_mut(affected_id) {
                affected_item.timeline_range = FrameRange::new(
                    affected_item.timeline_in() - gap_duration,
                    affected_item.timeline_out() - gap_duration,
                );
            }
            timeline.reindex_item(track_id, affected_id);
        }
        
        timeline.update_duration();
        timeline.dirty = true;
        
        Ok(EditOperation::RemoveClip { item_id: item_id })
    }

    /// Ripple insert - opens space for new clip
    pub fn ripple_insert(
        &self,
        timeline: &mut Timeline,
        track_id: TrackId,
        position: Frame,
        duration: Frame,
    ) -> Result<()> {
        let mut to_shift = Vec::new();
        if let Some(track) = timeline.get_track(track_id) {
            for (_, key) in track.items().range(position..) {
                if let Some(item) = timeline.item_by_key(*key) {
                    to_shift.push(item.id);
                }
            }
        }
        
        for affected_id in to_shift {
            // Collect the values first: `get_item_mut` and `get_track_mut` would
            // otherwise both borrow the timeline mutably at the same time.
            let (old_start, new_start, key) = match timeline.get_item_mut(affected_id) {
                Some(affected_item) => {
                    affected_item.timeline_range = FrameRange::new(
                        affected_item.timeline_in() + duration,
                        affected_item.timeline_out() + duration,
                    );
                    (affected_item.timeline_in() - duration, affected_item.timeline_in(), affected_item.key)
                }
                None => continue,
            };
            if let Some(track) = timeline.get_track_mut(track_id) {
                track.items_mut().remove(&old_start);
                track.items_mut().insert(new_start, key);
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
        // Gather everything the edit needs from the two items before mutating, so no
        // immutable borrow is live across the mutations below.
        let (left_in, left_out, left_track, left_clip, left_source_in) = {
            let left = timeline
                .get_item(left_item_id)
                .ok_or(TimelineError::ClipNotFound(left_item_id.to_string()))?;
            (
                left.timeline_in(),
                left.timeline_out(),
                left.track_id,
                left.clip_id,
                left.source_in(),
            )
        };
        let (right_in, right_out, right_clip) = {
            let right = timeline
                .get_item(right_item_id)
                .ok_or(TimelineError::ClipNotFound(right_item_id.to_string()))?;
            (right.timeline_in(), right.timeline_out(), right.clip_id)
        };

        // Must be adjacent
        if left_out != right_in {
            return Err(TimelineError::InvalidOperation("Clips must be adjacent for roll edit".into()).into());
        }
        if left_clip != right_clip {
            return Err(TimelineError::InvalidOperation("Roll edit only works on same source clip".into()).into());
        }

        // Check bounds
        if new_edit_point <= left_in || new_edit_point >= right_out {
            return Err(TimelineError::InvalidOperation("New edit point out of bounds".into()).into());
        }

        let left_source_duration = new_edit_point - left_in;
        let right_source_duration = right_out - new_edit_point;

        // Update left item
        if let Some(left_mut) = timeline.get_item_mut(left_item_id) {
            left_mut.timeline_range = FrameRange::new(left_in, new_edit_point);
            left_mut.source_range =
                FrameRange::new(left_source_in, left_source_in + left_source_duration);
        }

        // Update right item
        let right_key = match timeline.get_item_mut(right_item_id) {
            Some(right_mut) => {
                right_mut.timeline_range = FrameRange::new(new_edit_point, right_out);
                right_mut.source_range = FrameRange::new(
                    right_mut.source_out() - right_source_duration,
                    right_mut.source_out(),
                );
                Some(right_mut.key)
            }
            None => None,
        };

        // Update track index now that the item borrow has ended.
        if right_key.is_some() {
            if let Some(track) = timeline.get_track_mut(left_track) {
                track.items_mut().remove(&right_in);
                track.items_mut().insert(new_edit_point, right_key.expect("checked above"));
            }
        }

        timeline.dirty = true;

        Ok(EditOperation::MoveClip {
            item_id: left_item_id,
            from: left_out,
            to: new_edit_point,
            track: left_track,
        })
    }

    /// Slip edit - move source in/out, keep timeline position
    pub fn slip_edit(
        &self,
        timeline: &mut Timeline,
        item_id: TrackItemId,
        source_delta: Frame,
    ) -> Result<EditOperation> {
        // Copy out what we need before mutating, so the immutable borrow ends first.
        let (track_id, timeline_in, source_duration, timeline_duration) = {
            let item = timeline
                .get_item(item_id)
                .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;
            (
                item.track_id,
                item.timeline_in(),
                item.source_duration(),
                item.timeline_duration(),
            )
        };

        let new_source_in = timeline
            .get_item(item_id)
            .map(|i| i.source_in())
            .unwrap_or(Frame::ZERO)
            + source_delta;
        let new_source_out = new_source_in + source_duration;

        // Check bounds (would need clip duration)
        // For now, allow any slip

        if let Some(item_mut) = timeline.get_item_mut(item_id) {
            item_mut.source_range = FrameRange::new(new_source_in, new_source_out);
        }

        timeline.dirty = true;

        Ok(EditOperation::MoveClip {
            item_id,
            from: timeline_in,
            to: timeline_in,
            track: track_id,
        })
    }

    /// Slide edit - move timeline position, keep source in/out
    pub fn slide_edit(
        &self,
        timeline: &mut Timeline,
        item_id: TrackItemId,
        timeline_delta: Frame,
    ) -> Result<EditOperation> {
        // Copy out what we need before mutating, so the immutable borrow ends first.
        let (track_id, old_position, item_duration) = {
            let item = timeline
                .get_item(item_id)
                .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;
            (item.track_id, item.timeline_in(), item.timeline_duration())
        };

        let new_position = old_position + timeline_delta;

        if new_position < Frame::ZERO {
            return Err(TimelineError::InvalidOperation("Cannot slide before timeline start".into()).into());
        }

        // Check for collisions
        let new_range = FrameRange::new(new_position, new_position + item_duration);
        let collision = {
            let mut found = false;
            if let Some(track) = timeline.get_track(track_id) {
                for (_, key) in track.items().range(..) {
                    if let Some(other) = timeline.item_by_key(*key) {
                        if other.id != item_id && other.timeline_range.intersects(new_range) {
                            found = true;
                            break;
                        }
                    }
                }
            }
            found
        };
        if collision {
            return Err(TimelineError::OverlapNotAllowed("Slide would cause overlap".into()).into());
        }

        // Move item
        timeline.move_clip(item_id, new_position, None)?;

        Ok(EditOperation::MoveClip {
            item_id,
            from: old_position,
            to: new_position,
            track: track_id,
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
        // Copy out what we need before mutating, so the immutable borrow ends first.
        let (track_id, timeline_position, source_duration) = {
            let old_item = timeline
                .get_item(old_item_id)
                .ok_or(TimelineError::ClipNotFound(old_item_id.to_string()))?;
            (
                old_item.track_id,
                old_item.timeline_in(),
                old_item.source_duration(),
            )
        };

        if pool.get_clip(new_clip_id).is_none() {
            return Err(TimelineError::ClipNotFound(new_clip_id.to_string()).into());
        }

        // Remove old clip
        timeline.remove_clip(old_item_id);

        // Add new clip at same position
        timeline.add_clip(pool, new_clip_id, track_id, timeline_position)?;

        let _ = source_duration;

        Ok(EditOperation::RemoveClip { item_id: old_item_id })
    }

    /// Fit to fill - change clip speed to fill duration
    pub fn fit_to_fill(
        &self,
        timeline: &mut Timeline,
        item_id: TrackItemId,
        target_duration: Frame,
    ) -> Result<EditOperation> {
        // Copy out what we need before mutating, so the immutable borrow ends first.
        let (timeline_in, track_id, source_duration) = {
            let item = timeline
                .get_item(item_id)
                .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;
            (item.timeline_in(), item.track_id, item.source_duration())
        };

        if source_duration == Frame::ZERO {
            return Err(TimelineError::InvalidOperation("Source duration is zero".into()).into());
        }

        if target_duration == Frame::ZERO {
            return Err(TimelineError::InvalidOperation("Target duration is zero".into()).into());
        }

        let speed = source_duration.0 as f32 / target_duration.0 as f32;

        if let Some(item_mut) = timeline.get_item_mut(item_id) {
            item_mut.speed = speed;
            item_mut.timeline_range =
                FrameRange::new(timeline_in, timeline_in + target_duration);
        }

        timeline.reindex_item(track_id, item_id);
        timeline.dirty = true;

        Ok(EditOperation::MoveClip {
            item_id,
            from: timeline_in,
            to: timeline_in,
            track: track_id,
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
        // Copy out the fields the guards need before mutating.
        let (left_track, left_clip, left_out) = {
            let left = timeline
                .get_item(left_item_id)
                .ok_or(TimelineError::ClipNotFound(left_item_id.to_string()))?;
            (left.track_id, left.clip_id, left.timeline_out())
        };
        let (right_track, right_clip, right_in) = {
            let right = timeline
                .get_item(right_item_id)
                .ok_or(TimelineError::ClipNotFound(right_item_id.to_string()))?;
            (right.track_id, right.clip_id, right.timeline_in())
        };

        if left_track != right_track {
            return Err(TimelineError::InvalidOperation("Clips must be on same track".into()).into());
        }
        if left_out != right_in {
            return Err(TimelineError::InvalidOperation("Clips must be adjacent".into()).into());
        }
        if left_clip == right_clip {
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
                    if let Some(item) = timeline.item_by_key(*key) {
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