//! Timeline clip operations and management

use crate::core::color::{ColorSpace, PixelFormat};
use crate::core::error::{Result, TimelineError};
use crate::core::id::{ClipId, ProjectId, TimelineId, TrackId, TrackItemId};
use crate::core::time::{Frame, FrameRange, FrameRate, Timecode};
use crate::core::types::{EditOperation, MediaType, TrackType, TrimEdge};
use crate::media::pool::MediaPool;
use crate::timeline::track::{
    AudioTrack, SubtitleTrack, TrackContainer, TrackItem, TrackItemColor, TrackItemKey, VideoTrack,
};
use std::collections::HashMap;

/// Timeline clip - higher level representation
#[derive(Debug, Clone)]
pub struct TimelineClip {
    pub item_id: TrackItemId,
    pub clip_id: ClipId,
    pub track_id: TrackId,
    pub track_type: TrackType,
    pub source_range: FrameRange,
    pub timeline_range: FrameRange,
    pub name: String,
    pub color_label: Option<crate::core::types::ColorLabel>,
    pub enabled: bool,
    pub locked: bool,
    pub effects: Vec<TrackItemEffect>,
}

#[derive(Debug, Clone)]
pub struct TrackItemEffect {
    pub effect_type: crate::core::types::EffectType,
    pub enabled: bool,
    pub parameters: std::collections::HashMap<String, crate::core::types::EffectValue>,
    pub keyframes: std::collections::HashMap<String, Vec<crate::core::types::Keyframe>>,
}

/// Timeline - the main editing sequence
pub struct Timeline {
    pub id: TimelineId,
    pub project_id: ProjectId,
    pub name: String,
    pub settings: TimelineSettings,
    pub tracks: TrackContainer,
    pub items: HashMap<TrackItemId, TrackItem>,
    pub markers: HashMap<crate::core::id::MarkerId, Marker>,
    pub selection: Selection,
    pub playhead: Frame,
    pub duration: Frame,
    pub dirty: bool,
}

#[derive(Debug, Clone)]
pub struct TimelineSettings {
    pub resolution: crate::core::types::Resolution,
    pub frame_rate: FrameRate,
    pub color_space: ColorSpace,
    pub pixel_format: PixelFormat,
    pub sample_rate: u32,
    pub start_timecode: Timecode,
    pub video_track_count: u32,
    pub audio_track_count: u32,
}

impl Default for TimelineSettings {
    fn default() -> Self {
        Self {
            resolution: crate::core::types::Resolution::Uhd4k,
            frame_rate: FrameRate::Fps24,
            color_space: ColorSpace::Rec709,
            pixel_format: PixelFormat::Rgba16F,
            sample_rate: 48000,
            start_timecode: Timecode::zero(FrameRate::Fps24),
            video_track_count: 4,
            audio_track_count: 8,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Marker {
    pub id: crate::core::id::MarkerId,
    pub position: Frame,
    pub duration: Option<Frame>,
    pub name: String,
    pub note: String,
    pub color: crate::core::types::ColorLabel,
    pub marker_type: crate::core::types::MarkerType,
}

#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub items: Vec<TrackItemId>,
    pub range: Option<FrameRange>,
    pub tracks: Vec<TrackId>,
}

impl Timeline {
    pub fn new(project_id: ProjectId, name: String, settings: TimelineSettings) -> Self {
        let tracks = TrackContainer::new(settings.video_track_count, settings.audio_track_count);
        Self {
            id: TimelineId::generate(),
            project_id,
            name,
            settings,
            tracks,
            items: HashMap::new(),
            markers: HashMap::new(),
            selection: Selection::default(),
            playhead: Frame::ZERO,
            duration: Frame::ZERO,
            dirty: false,
        }
    }

    /// Add a clip to the timeline
    pub fn add_clip(
        &mut self,
        pool: &MediaPool,
        clip_id: ClipId,
        track_id: TrackId,
        timeline_position: Frame,
    ) -> Result<TrackItemId> {
        let clip = pool
            .get_clip(clip_id)
            .ok_or(TimelineError::ClipNotFound(clip_id.to_string()))?;

        let track_type = match clip.metadata.media_type {
            MediaType::Video => TrackType::Video,
            MediaType::Audio => TrackType::Audio,
            MediaType::Image => TrackType::Video,
            _ => TrackType::Video,
        };

        // Verify track type matches
        let (locked, track_item_count) = match track_type {
            TrackType::Video => {
                let t = self
                    .tracks
                    .get_video_track_mut(track_id)
                    .ok_or(TimelineError::TrackNotFound(track_id.to_string()))?;
                (t.locked, t.items.len() as u32)
            }
            TrackType::Audio => {
                let t = self
                    .tracks
                    .get_audio_track_mut(track_id)
                    .ok_or(TimelineError::TrackNotFound(track_id.to_string()))?;
                (t.locked, t.items.len() as u32)
            }
            TrackType::Subtitle => {
                let t = self
                    .tracks
                    .get_subtitle_track_mut(track_id)
                    .ok_or(TimelineError::TrackNotFound(track_id.to_string()))?;
                (t.locked, t.items.len() as u32)
            }
        };

        if locked {
            return Err(TimelineError::TrackLocked(track_id.to_string()).into());
        }

        // Find available space if position is occupied
        let position = self.find_space(track_id, timeline_position, clip.metadata.duration)?;

        let source_range = FrameRange::new(Frame::ZERO, clip.metadata.duration);
        let timeline_range = FrameRange::new(position, position + clip.metadata.duration);

        let mut item = TrackItem::new(
            track_id,
            Some(clip_id),
            source_range,
            position,
            track_item_count,
        );

        item.metadata.name = clip.metadata.tags.name.clone();
        item.color = clip
            .metadata
            .tags
            .color_label
            .map(|c| match c {
                crate::core::types::ColorLabel::Red => TrackItemColor::Red,
                crate::core::types::ColorLabel::Orange => TrackItemColor::Orange,
                crate::core::types::ColorLabel::Yellow => TrackItemColor::Yellow,
                crate::core::types::ColorLabel::Green => TrackItemColor::Green,
                crate::core::types::ColorLabel::Cyan => TrackItemColor::Cyan,
                crate::core::types::ColorLabel::Blue => TrackItemColor::Blue,
                crate::core::types::ColorLabel::Purple => TrackItemColor::Purple,
                crate::core::types::ColorLabel::Pink => TrackItemColor::Pink,
                _ => TrackItemColor::None,
            })
            .unwrap_or(TrackItemColor::None);

        let item_key = item.key;
        let item_id = item.id;

        // Add to track
        match track_type {
            TrackType::Video => {
                if let Some(vtrack) = self.tracks.get_video_track_mut(track_id) {
                    vtrack.items.insert(position, item_key);
                }
            }
            TrackType::Audio => {
                if let Some(atrack) = self.tracks.get_audio_track_mut(track_id) {
                    atrack.items.insert(position, item_key);
                }
            }
            TrackType::Subtitle => {
                if let Some(strack) = self.tracks.get_subtitle_track_mut(track_id) {
                    strack.items.insert(position, item_key);
                }
            }
        }

        self.items.insert(item_id, item);
        self.update_duration();
        self.dirty = true;

        Ok(item_id)
    }

    /// Find available space on track (no overlap)
    fn find_space(&self, track_id: TrackId, preferred: Frame, duration: Frame) -> Result<Frame> {
        let track = match self.get_track(track_id) {
            Some(t) => t,
            None => return Ok(preferred),
        };

        let mut position = preferred;
        let end = position + duration;

        // Check for overlaps
        for (_, item_key) in track.items().range(..end) {
            if let Some(item) = self.item_by_key(*item_key) {
                if item
                    .timeline_range
                    .intersects(FrameRange::new(position, end))
                {
                    // Move to after this item
                    position = item.timeline_range.end;
                    // Restart check
                    return self.find_space(track_id, position, duration);
                }
            }
        }

        Ok(position)
    }

    pub(crate) fn get_track(&self, track_id: TrackId) -> Option<&dyn TrackItems> {
        self.tracks
            .get_video_track(track_id)
            .map(|t| t as &dyn TrackItems)
            .or_else(|| {
                self.tracks
                    .get_audio_track(track_id)
                    .map(|t| t as &dyn TrackItems)
            })
            .or_else(|| {
                self.tracks
                    .get_subtitle_track(track_id)
                    .map(|t| t as &dyn TrackItems)
            })
    }

    pub fn remove_clip(&mut self, item_id: TrackItemId) -> Option<TrackItem> {
        if let Some(item) = self.items.remove(&item_id) {
            // Remove from track
            match self.get_track_mut(item.track_id) {
                Some(track) => {
                    track.items_mut().remove(&item.timeline_range.start);
                }
                None => {}
            }
            self.update_duration();
            self.dirty = true;
            Some(item)
        } else {
            None
        }
    }

    pub(crate) fn get_track_mut(&mut self, track_id: TrackId) -> Option<&mut dyn TrackItemsMut> {
        // Resolve the track kind first: returning a borrow from inside an
        // `if let` chain would keep every branch's `&mut self.tracks` borrow
        // alive at once.
        match self.get_track_type(track_id).ok()? {
            TrackType::Video => self
                .tracks
                .get_video_track_mut(track_id)
                .map(|t| t as &mut dyn TrackItemsMut),
            TrackType::Audio => self
                .tracks
                .get_audio_track_mut(track_id)
                .map(|t| t as &mut dyn TrackItemsMut),
            TrackType::Subtitle => self
                .tracks
                .get_subtitle_track_mut(track_id)
                .map(|t| t as &mut dyn TrackItemsMut),
        }
    }

    pub fn move_clip(
        &mut self,
        item_id: TrackItemId,
        new_position: Frame,
        new_track: Option<TrackId>,
    ) -> Result<()> {
        // Read the item through a shared borrow first: the track lookups below
        // need `&mut self`, which cannot coexist with a live `&mut TrackItem`.
        let (old_track_id, old_start, item_key, duration) = {
            let item = self
                .items
                .get(&item_id)
                .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;
            (
                item.track_id,
                item.timeline_in(),
                item.key,
                item.timeline_duration(),
            )
        };

        let target_track_id = new_track.unwrap_or(old_track_id);

        if target_track_id != old_track_id {
            // Move between tracks
            let track_type = self.get_track_type(target_track_id)?;
            let item_track_type = self.get_track_type(old_track_id)?;

            if track_type != item_track_type {
                return Err(TimelineError::InvalidOperation(
                    "Cannot move clip to different track type".into(),
                )
                .into());
            }

            // Remove from old track
            if let Some(track) = self.get_track_mut(old_track_id) {
                track.items_mut().remove(&old_start);
            }

            // Add to new track
            if let Some(track) = self.get_track_mut(target_track_id) {
                track.items_mut().insert(new_position, item_key);
            }
        } else {
            // Same track - just move
            if let Some(track) = self.get_track_mut(old_track_id) {
                track.items_mut().remove(&old_start);
                track.items_mut().insert(new_position, item_key);
            }
        }

        if let Some(item) = self.items.get_mut(&item_id) {
            item.track_id = target_track_id;
            item.timeline_range = FrameRange::new(new_position, new_position + duration);
        }

        self.update_duration();
        self.dirty = true;
        Ok(())
    }

    pub fn trim_clip(
        &mut self,
        item_id: TrackItemId,
        edge: TrimEdge,
        new_position: Frame,
    ) -> Result<()> {
        let (locked, track_id, old_in, item_key) = {
            let item = self
                .items
                .get(&item_id)
                .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;
            (item.locked, item.track_id, item.timeline_in(), item.key)
        };

        if locked {
            return Err(TimelineError::InvalidTrim("Clip is locked".into()).into());
        }

        if let Some(item) = self.items.get_mut(&item_id) {
            match edge {
                TrimEdge::In => {
                    if new_position >= item.timeline_out() {
                        return Err(TimelineError::InvalidTrim(
                            "In point cannot be at or after out point".into(),
                        )
                        .into());
                    }
                    let delta = new_position - item.timeline_in();
                    item.trim_in(delta);
                }
                TrimEdge::Out => {
                    if new_position <= item.timeline_in() {
                        return Err(TimelineError::InvalidTrim(
                            "Out point cannot be at or before in point".into(),
                        )
                        .into());
                    }
                    let delta = new_position - item.timeline_out();
                    item.trim_out(delta);
                }
            }
        }

        // Update track index
        let new_in = self
            .items
            .get(&item_id)
            .map(|item| item.timeline_in())
            .unwrap_or(old_in);
        if let Some(track) = self.get_track_mut(track_id) {
            track
                .items_mut()
                .remove(&(new_in - (new_position - new_in)));
            track.items_mut().insert(new_in, item_key);
        }

        self.update_duration();
        self.dirty = true;
        Ok(())
    }

    pub fn split_clip(&mut self, item_id: TrackItemId, position: Frame) -> Result<TrackItemId> {
        // Scope the shared borrow of `self.items` so the track insert below can
        // take `&mut self`.
        let (mut right_item, left_duration, right_duration, track_id, source_in, source_out) = {
            let item = self
                .items
                .get(&item_id)
                .ok_or(TimelineError::ClipNotFound(item_id.to_string()))?;

            if !item.timeline_range.contains(position) || position == item.timeline_in() {
                return Err(TimelineError::InvalidSplit(
                    "Split position must be within clip".into(),
                )
                .into());
            }

            let left_duration = position - item.timeline_in();
            let right_duration = item.timeline_out() - position;

            // Create new item for the right portion
            let mut right_item = item.clone();
            right_item.id = TrackItemId::generate();
            right_item.key = TrackItemKey::default();

            (
                right_item,
                left_duration,
                right_duration,
                item.track_id,
                item.source_in(),
                item.source_out(),
            )
        };

        // Update left item
        if let Some(left) = self.items.get_mut(&item_id) {
            left.timeline_range = FrameRange::new(left.timeline_in(), position);
            left.source_range = FrameRange::new(left.source_in(), left.source_in() + left_duration);
        }

        // Setup right item
        right_item.timeline_range = FrameRange::new(position, position + right_duration);
        right_item.source_range = FrameRange::new(source_in + left_duration, source_out);

        let right_key = right_item.key;
        let right_id = right_item.id;

        // Insert into track
        if let Some(track) = self.get_track_mut(track_id) {
            track.items_mut().insert(position, right_key);
        }

        self.items.insert(right_id, right_item);
        self.dirty = true;

        Ok(right_id)
    }

    pub fn join_clips(&mut self, item_id_a: TrackItemId, item_id_b: TrackItemId) -> Result<()> {
        let item_a = self
            .items
            .get(&item_id_a)
            .ok_or(TimelineError::ClipNotFound(item_id_a.to_string()))?;
        let item_b = self
            .items
            .get(&item_id_b)
            .ok_or(TimelineError::ClipNotFound(item_id_b.to_string()))?;

        // Must be on same track, same clip, adjacent
        if item_a.track_id != item_b.track_id {
            return Err(TimelineError::CannotJoin("Clips on different tracks".into()).into());
        }
        if item_a.clip_id != item_b.clip_id {
            return Err(TimelineError::CannotJoin("Clips from different sources".into()).into());
        }
        if item_a.timeline_out() != item_b.timeline_in() {
            return Err(TimelineError::CannotJoin("Clips are not adjacent".into()).into());
        }
        if item_a.source_out() != item_b.source_in() {
            return Err(TimelineError::CannotJoin("Source ranges are not adjacent".into()).into());
        }

        // Merge B into A
        let (b_timeline_out, b_source_out) = (item_b.timeline_out(), item_b.source_out());
        if let Some(merged) = self.items.get_mut(&item_id_a) {
            merged.timeline_range = FrameRange::new(merged.timeline_in(), b_timeline_out);
            merged.source_range = FrameRange::new(merged.source_in(), b_source_out);
        }

        // Remove B
        self.remove_clip(item_id_b);
        self.dirty = true;
        Ok(())
    }

    fn get_track_type(&self, track_id: TrackId) -> Result<TrackType> {
        if self.tracks.get_video_track(track_id).is_some() {
            return Ok(TrackType::Video);
        }
        if self.tracks.get_audio_track(track_id).is_some() {
            return Ok(TrackType::Audio);
        }
        if self.tracks.get_subtitle_track(track_id).is_some() {
            return Ok(TrackType::Subtitle);
        }
        Err(TimelineError::TrackNotFound(track_id.to_string()).into())
    }

    pub(crate) fn update_duration(&mut self) {
        let mut max_end = Frame::ZERO;
        for item in self.items.values() {
            if item.timeline_out() > max_end {
                max_end = item.timeline_out();
            }
        }
        self.duration = max_end;
    }

    pub fn get_item(&self, item_id: TrackItemId) -> Option<&TrackItem> {
        self.items.get(&item_id)
    }

    pub fn get_item_mut(&mut self, item_id: TrackItemId) -> Option<&mut TrackItem> {
        self.items.get_mut(&item_id)
    }

    /// Look a timeline item up by its slot key.
    pub fn item_by_key(&self, key: TrackItemKey) -> Option<&TrackItem> {
        self.items.values().find(|item| item.key == key)
    }

    /// Iterate every item in the timeline, in slotmap order.
    pub fn iter_items(&self) -> impl Iterator<Item = &TrackItem> {
        self.items.values()
    }

    /// Re-key `item_id` inside its track's item map.
    ///
    /// Track item maps are keyed by timeline start frame, so any operation that
    /// moves an item has to update both the item's range and this index. Doing it
    /// here keeps callers from holding an item borrow while reaching for the
    /// track, which the borrow checker rightly rejects.
    pub fn reindex_item(&mut self, track_id: TrackId, item_id: TrackItemId) {
        let (key, new_in) = match self.items.get(&item_id) {
            Some(item) => (item.key, item.timeline_in()),
            None => return,
        };

        let track = if let Some(t) = self.tracks.get_video_track_mut(track_id) {
            Some(t as &mut dyn TrackItemsMut)
        } else if let Some(t) = self.tracks.get_audio_track_mut(track_id) {
            Some(t as &mut dyn TrackItemsMut)
        } else {
            self.tracks
                .get_subtitle_track_mut(track_id)
                .map(|t| t as &mut dyn TrackItemsMut)
        };

        if let Some(track) = track {
            let items = track.items_mut();
            items.retain(|_, k| *k != key);
            items.insert(new_in, key);
        }
    }

    pub fn items_at_frame(&self, frame: Frame) -> Vec<&TrackItem> {
        self.items
            .values()
            .filter(|item| item.timeline_range.contains(frame))
            .collect()
    }

    pub fn items_in_range(&self, range: FrameRange) -> Vec<&TrackItem> {
        self.items
            .values()
            .filter(|item| item.timeline_range.intersects(range))
            .collect()
    }

    // Markers
    pub fn add_marker(
        &mut self,
        position: Frame,
        name: String,
        marker_type: crate::core::types::MarkerType,
    ) -> crate::core::id::MarkerId {
        let id = crate::core::id::MarkerId::generate();
        let marker = Marker {
            id,
            position,
            duration: None,
            name,
            note: String::new(),
            color: crate::core::types::ColorLabel::None,
            marker_type,
        };
        self.markers.insert(id, marker);
        self.dirty = true;
        id
    }

    pub fn remove_marker(&mut self, marker_id: crate::core::id::MarkerId) -> Option<Marker> {
        self.dirty = true;
        self.markers.remove(&marker_id)
    }

    pub fn get_markers(&self) -> Vec<&Marker> {
        self.markers.values().collect()
    }

    pub fn markers_in_range(&self, range: FrameRange) -> Vec<&Marker> {
        self.markers
            .values()
            .filter(|m| range.contains(m.position))
            .collect()
    }
}

/// Read-only access to a track's item map. Declared here (rather than in
/// `track.rs`) because `Timeline` needs to walk tracks generically, and shared
/// with `edit.rs`, which performs the same generic traversal.
pub(crate) trait TrackItems {
    fn items(&self) -> &std::collections::BTreeMap<Frame, TrackItemKey>;
}

/// Mutable access to a track's item map. See [`TrackItems`].
pub(crate) trait TrackItemsMut {
    fn items_mut(&mut self) -> &mut std::collections::BTreeMap<Frame, TrackItemKey>;
}

impl TrackItems for VideoTrack {
    fn items(&self) -> &std::collections::BTreeMap<Frame, TrackItemKey> {
        &self.items
    }
}

impl TrackItems for AudioTrack {
    fn items(&self) -> &std::collections::BTreeMap<Frame, TrackItemKey> {
        &self.items
    }
}

impl TrackItems for SubtitleTrack {
    fn items(&self) -> &std::collections::BTreeMap<Frame, TrackItemKey> {
        &self.items
    }
}

impl TrackItemsMut for VideoTrack {
    fn items_mut(&mut self) -> &mut std::collections::BTreeMap<Frame, TrackItemKey> {
        &mut self.items
    }
}

impl TrackItemsMut for AudioTrack {
    fn items_mut(&mut self) -> &mut std::collections::BTreeMap<Frame, TrackItemKey> {
        &mut self.items
    }
}

impl TrackItemsMut for SubtitleTrack {
    fn items_mut(&mut self) -> &mut std::collections::BTreeMap<Frame, TrackItemKey> {
        &mut self.items
    }
}
