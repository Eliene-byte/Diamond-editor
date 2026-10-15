//! Track management for timeline

use crate::core::id::{TrackId, TrackItemId, ClipId, TransitionId};
use crate::core::time::{Frame, FrameRange};
use crate::core::types::{TrackType, MediaType, EffectParams, EffectType, EffectValue, Interpolation, Easing, Keyframe};
use crate::core::error::{Result, TimelineError};
use crate::media::pool::PoolClip;
use std::collections::BTreeMap;
use slotmap::SlotMap;

slotmap::new_key_type! { pub struct TrackItemKey; }

/// Track item - a clip or generator on a track
#[derive(Debug, Clone)]
pub struct TrackItem {
    pub id: TrackItemId,
    pub key: TrackItemKey,
    pub track_id: TrackId,
    pub clip_id: Option<ClipId>, // None for generators
    pub source_range: FrameRange, // Range in source media
    pub timeline_range: FrameRange, // Range on timeline
    pub track_index: u32, // Position within track
    pub enabled: bool,
    pub locked: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub volume: f32, // For audio
    pub pan: f32, // For audio
    pub speed: f32, // 1.0 = normal, -1.0 = reverse, 2.0 = 2x
    pub effects: Vec<TrackItemEffect>,
    pub transitions: (Option<TransitionId>, Option<TransitionId>), // (in, out)
    pub color: TrackItemColor,
    pub metadata: TrackItemMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Add,
    Screen,
    Multiply,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    SoftLight,
    HardLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackItemColor {
    None,
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Purple,
    Pink,
}

#[derive(Debug, Clone)]
pub struct TrackItemEffect {
    pub effect_type: EffectType,
    pub enabled: bool,
    pub parameters: std::collections::HashMap<String, EffectValue>,
    pub keyframes: std::collections::HashMap<String, Vec<Keyframe>>,
}

#[derive(Debug, Clone, Default)]
pub struct TrackItemMetadata {
    pub name: String,
    pub notes: String,
    pub tags: Vec<String>,
    pub custom_data: std::collections::HashMap<String, String>,
}

impl TrackItem {
    pub fn new(
        track_id: TrackId,
        clip_id: Option<ClipId>,
        source_range: FrameRange,
        timeline_start: Frame,
        track_index: u32,
    ) -> Self {
        let timeline_end = timeline_start + source_range.length();
        Self {
            id: TrackItemId::generate(),
            key: TrackItemKey::default(),
            track_id,
            clip_id,
            source_range,
            timeline_range: FrameRange::new(timeline_start, timeline_end),
            track_index,
            enabled: true,
            locked: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            volume: 1.0,
            pan: 0.0,
            speed: 1.0,
            effects: Vec::new(),
            transitions: (None, None),
            color: TrackItemColor::None,
            metadata: TrackItemMetadata::default(),
        }
    }

    pub fn timeline_in(&self) -> Frame { self.timeline_range.start }
    pub fn timeline_out(&self) -> Frame { self.timeline_range.end }
    pub fn timeline_duration(&self) -> Frame { self.timeline_range.length() }
    
    pub fn source_in(&self) -> Frame { self.source_range.start }
    pub fn source_out(&self) -> Frame { self.source_range.end }
    pub fn source_duration(&self) -> Frame { self.source_range.length() }

    pub fn set_timeline_in(&mut self, frame: Frame) {
        let duration = self.timeline_duration();
        self.timeline_range = FrameRange::new(frame, frame + duration);
    }

    pub fn set_timeline_out(&mut self, frame: Frame) {
        self.timeline_range = FrameRange::new(self.timeline_range.start, frame);
    }

    pub fn set_source_in(&mut self, frame: Frame) {
        self.source_range = FrameRange::new(frame, self.source_range.end);
    }

    pub fn set_source_out(&mut self, frame: Frame) {
        self.source_range = FrameRange::new(self.source_range.start, frame);
    }

    pub fn trim_in(&mut self, delta: Frame) {
        self.source_range = FrameRange::new(self.source_range.start + delta, self.source_range.end);
        self.timeline_range = FrameRange::new(self.timeline_range.start + delta * self.speed_sign(), self.timeline_range.end);
    }

    pub fn trim_out(&mut self, delta: Frame) {
        self.source_range = FrameRange::new(self.source_range.start, self.source_range.end + delta);
        self.timeline_range = FrameRange::new(self.timeline_range.start, self.timeline_range.end + delta * self.speed_sign());
    }

    fn speed_sign(&self) -> i64 { if self.speed < 0.0 { -1 } else { 1 } }

    pub fn contains_frame(&self, frame: Frame) -> bool {
        self.timeline_range.contains(frame)
    }

    pub fn frame_at_timeline(&self, timeline_frame: Frame) -> Option<Frame> {
        if !self.timeline_range.contains(timeline_frame) { return None; }
        let offset = timeline_frame - self.timeline_range.start;
        let source_offset = if self.speed != 0.0 {
            Frame((offset.0 as f32 / self.speed.abs()) as i64)
        } else { Frame::ZERO };
        if self.speed < 0.0 {
            Some(self.source_range.end - source_offset - Frame(1))
        } else {
            Some(self.source_range.start + source_offset)
        }
    }

    pub fn add_effect(&mut self, effect_type: EffectType) -> TrackItemEffect {
        let effect = TrackItemEffect {
            effect_type,
            enabled: true,
            parameters: Self::default_params(effect_type),
            keyframes: std::collections::HashMap::new(),
        };
        self.effects.push(effect.clone());
        effect
    }

    pub fn remove_effect(&mut self, index: usize) -> Option<TrackItemEffect> {
        if index < self.effects.len() {
            Some(self.effects.remove(index))
        } else { None }
    }

    fn default_params(effect_type: EffectType) -> std::collections::HashMap<String, EffectValue> {
        use std::collections::HashMap;
        let mut params = HashMap::new();
        match effect_type {
            EffectType::Transform => {
                params.insert("position".into(), EffectValue::Vec2(0.0, 0.0));
                params.insert("scale".into(), EffectValue::Vec2(1.0, 1.0));
                params.insert("rotation".into(), EffectValue::Float(0.0));
                params.insert("anchor".into(), EffectValue::Vec2(0.5, 0.5));
            }
            EffectType::Crop => {
                params.insert("left".into(), EffectValue::Float(0.0));
                params.insert("right".into(), EffectValue::Float(0.0));
                params.insert("top".into(), EffectValue::Float(0.0));
                params.insert("bottom".into(), EffectValue::Float(0.0));
            }
            EffectType::Opacity => {
                params.insert("opacity".into(), EffectValue::Float(1.0));
            }
            EffectType::BlendMode => {
                params.insert("mode".into(), EffectValue::Int(BlendMode::Normal as i64));
            }
            EffectType::Speed => {
                params.insert("speed".into(), EffectValue::Float(1.0));
                params.insert("reverse".into(), EffectValue::Bool(false));
                params.insert("ripple".into(), EffectValue::Bool(false));
            }
            EffectType::Volume => {
                params.insert("volume".into(), EffectValue::Float(1.0));
                params.insert("pan".into(), EffectValue::Float(0.0));
            }
            EffectType::LiftGammaGain => {
                params.insert("lift".into(), EffectValue::Vec3(0.0, 0.0, 0.0));
                params.insert("gamma".into(), EffectValue::Vec3(1.0, 1.0, 1.0));
                params.insert("gain".into(), EffectValue::Vec3(1.0, 1.0, 1.0));
            }
            EffectType::ContrastSaturation => {
                params.insert("contrast".into(), EffectValue::Float(1.0));
                params.insert("saturation".into(), EffectValue::Float(1.0));
                params.insert("pivot".into(), EffectValue::Float(0.5));
            }
            EffectType::ColorWheels => {
                params.insert("shadows".into(), EffectValue::Vec3(0.0, 0.0, 0.0));
                params.insert("midtones".into(), EffectValue::Vec3(0.0, 0.0, 0.0));
                params.insert("highlights".into(), EffectValue::Vec3(0.0, 0.0, 0.0));
                params.insert("offset".into(), EffectValue::Vec3(0.0, 0.0, 0.0));
            }
            _ => {}
        }
        params
    }
}

/// Video track
#[derive(Debug, Clone)]
pub struct VideoTrack {
    pub id: TrackId,
    pub index: u32,
    pub name: String,
    pub enabled: bool,
    pub locked: bool,
    pub solo: bool,
    pub items: BTreeMap<Frame, TrackItemKey>, // Sorted by timeline position
    pub height: f32,
    pub color: TrackColor,
}

impl VideoTrack {
    pub fn new(id: TrackId, index: u32, name: String) -> Self {
        Self {
            id,
            index,
            name,
            enabled: true,
            locked: false,
            solo: false,
            items: BTreeMap::new(),
            height: 60.0,
            color: TrackColor::Video,
        }
    }

    pub fn add_item(&mut self, item: TrackItem) -> TrackItemKey {
        let key = item.key;
        self.items.insert(item.timeline_range.start, key);
        key
    }

    pub fn remove_item(&mut self, key: TrackItemKey) -> Option<TrackItem> {
        // Need to find by key - would need reverse lookup
        None
    }

    pub fn item_at_frame(&self, frame: Frame) -> Option<&TrackItemKey> {
        // Find item that contains this frame
        self.items.range(..=frame).next_back()
            .map(|(_, key)| key)
    }

    pub fn items_in_range(&self, range: FrameRange) -> Vec<&TrackItemKey> {
        self.items.range(range.start..range.end)
            .map(|(_, key)| key)
            .collect()
    }
}

/// Audio track
#[derive(Debug, Clone)]
pub struct AudioTrack {
    pub id: TrackId,
    pub index: u32,
    pub name: String,
    pub enabled: bool,
    pub locked: bool,
    pub solo: bool,
    pub muted: bool,
    pub volume: f32,
    pub pan: f32,
    pub items: BTreeMap<Frame, TrackItemKey>,
    pub height: f32,
    pub color: TrackColor,
    pub bus: Option<TrackId>, // Submix bus
    pub automation: TrackAutomation,
}

#[derive(Debug, Clone, Default)]
pub struct TrackAutomation {
    pub volume: Vec<Keyframe>,
    pub pan: Vec<Keyframe>,
    pub mute: Vec<Keyframe>,
    pub solo: Vec<Keyframe>,
}

impl AudioTrack {
    pub fn new(id: TrackId, index: u32, name: String) -> Self {
        Self {
            id,
            index,
            name,
            enabled: true,
            locked: false,
            solo: false,
            muted: false,
            volume: 1.0,
            pan: 0.0,
            items: BTreeMap::new(),
            height: 40.0,
            color: TrackColor::Audio,
            bus: None,
            automation: TrackAutomation::default(),
        }
    }
}

/// Subtitle track
#[derive(Debug, Clone)]
pub struct SubtitleTrack {
    pub id: TrackId,
    pub index: u32,
    pub name: String,
    pub enabled: bool,
    pub locked: bool,
    pub items: BTreeMap<Frame, TrackItemKey>,
    pub height: f32,
    pub color: TrackColor,
}

impl SubtitleTrack {
    pub fn new(id: TrackId, index: u32, name: String) -> Self {
        Self {
            id,
            index,
            name,
            enabled: true,
            locked: false,
            items: BTreeMap::new(),
            height: 30.0,
            color: TrackColor::Subtitle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackColor {
    Video,
    Audio,
    Subtitle,
    Bus,
}

/// Track container - holds all tracks
#[derive(Debug, Clone)]
pub struct TrackContainer {
    pub video_tracks: Vec<VideoTrack>,
    pub audio_tracks: Vec<AudioTrack>,
    pub subtitle_tracks: Vec<SubtitleTrack>,
    pub next_video_index: u32,
    pub next_audio_index: u32,
    pub next_subtitle_index: u32,
}

impl TrackContainer {
    pub fn new(video_count: u32, audio_count: u32) -> Self {
        let mut container = Self {
            video_tracks: Vec::new(),
            audio_tracks: Vec::new(),
            subtitle_tracks: Vec::new(),
            next_video_index: 0,
            next_audio_index: 0,
            next_subtitle_index: 0,
        };
        
        for i in 0..video_count {
            container.add_video_track(format!("V{}", i + 1));
        }
        for i in 0..audio_count {
            container.add_audio_track(format!("A{}", i + 1));
        }
        
        container
    }

    pub fn add_video_track(&mut self, name: String) -> TrackId {
        let id = TrackId::generate();
        let track = VideoTrack::new(id, self.next_video_index, name);
        self.video_tracks.push(track);
        self.next_video_index += 1;
        id
    }

    pub fn add_audio_track(&mut self, name: String) -> TrackId {
        let id = TrackId::generate();
        let track = AudioTrack::new(id, self.next_audio_index, name);
        self.audio_tracks.push(track);
        self.next_audio_index += 1;
        id
    }

    pub fn add_subtitle_track(&mut self, name: String) -> TrackId {
        let id = TrackId::generate();
        let track = SubtitleTrack::new(id, self.next_subtitle_index, name);
        self.subtitle_tracks.push(track);
        self.next_subtitle_index += 1;
        id
    }

    pub fn remove_track(&mut self, track_id: TrackId) -> bool {
        if let Some(pos) = self.video_tracks.iter().position(|t| t.id == track_id) {
            self.video_tracks.remove(pos);
            self.reindex_video();
            return true;
        }
        if let Some(pos) = self.audio_tracks.iter().position(|t| t.id == track_id) {
            self.audio_tracks.remove(pos);
            self.reindex_audio();
            return true;
        }
        if let Some(pos) = self.subtitle_tracks.iter().position(|t| t.id == track_id) {
            self.subtitle_tracks.remove(pos);
            self.reindex_subtitle();
            return true;
        }
        false
    }

    fn reindex_video(&mut self) {
        for (i, track) in self.video_tracks.iter_mut().enumerate() {
            track.index = i as u32;
        }
        self.next_video_index = self.video_tracks.len() as u32;
    }

    fn reindex_audio(&mut self) {
        for (i, track) in self.audio_tracks.iter_mut().enumerate() {
            track.index = i as u32;
        }
        self.next_audio_index = self.audio_tracks.len() as u32;
    }

    fn reindex_subtitle(&mut self) {
        for (i, track) in self.subtitle_tracks.iter_mut().enumerate() {
            track.index = i as u32;
        }
        self.next_subtitle_index = self.subtitle_tracks.len() as u32;
    }

    pub fn get_video_track(&self, track_id: TrackId) -> Option<&VideoTrack> {
        self.video_tracks.iter().find(|t| t.id == track_id)
    }

    pub fn get_audio_track(&self, track_id: TrackId) -> Option<&AudioTrack> {
        self.audio_tracks.iter().find(|t| t.id == track_id)
    }

    pub fn get_subtitle_track(&self, track_id: TrackId) -> Option<&SubtitleTrack> {
        self.subtitle_tracks.iter().find(|t| t.id == track_id)
    }

    pub fn get_video_track_mut(&mut self, track_id: TrackId) -> Option<&mut VideoTrack> {
        self.video_tracks.iter_mut().find(|t| t.id == track_id)
    }

    pub fn get_audio_track_mut(&mut self, track_id: TrackId) -> Option<&mut AudioTrack> {
        self.audio_tracks.iter_mut().find(|t| t.id == track_id)
    }

    pub fn get_subtitle_track_mut(&mut self, track_id: TrackId) -> Option<&mut SubtitleTrack> {
        self.subtitle_tracks.iter_mut().find(|t| t.id == track_id)
    }

    pub fn total_height(&self) -> f32 {
        self.video_tracks.iter().map(|t| t.height).sum::<f32>()
        + self.audio_tracks.iter().map(|t| t.height).sum::<f32>()
        + self.subtitle_tracks.iter().map(|t| t.height).sum::<f32>()
    }
}