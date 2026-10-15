//! Fundamental types for video editing

use crate::core::color::{ColorSpace, PixelFormat};
use crate::core::id::{BinId, ClipId, EffectId, MarkerId, TrackId, TrackItemId};
use crate::core::time::{Frame, Rational, Timecode};
use std::fmt;

/// Unique identifier for any timeline object
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub u64);

impl ObjectId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
    pub const fn nil() -> Self {
        Self(0)
    }
    pub fn next(&mut self) -> Self {
        self.0 += 1;
        Self(self.0)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "obj_{:016x}", self.0)
    }
}

/// Media type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MediaType {
    #[default]
    Video,
    Audio,
    Image,
    Subtitle,
    Compound,  // Nested timeline
    Generator, // Solid, text, noise, etc.
}

/// Video resolution preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Resolution {
    // Standard
    Ntsc,
    Pal,
    // HD
    Hd720,
    Hd1080,
    // 2K
    Dci2k,
    // 4K
    Uhd4k,
    Dci4k,
    // 5K
    Uhd5k,
    // 6K
    Uhd6k,
    // 8K
    Uhd8k,
    Dci8k,
    // Custom
    Custom(u32, u32),
}

impl Resolution {
    pub const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Ntsc => (720, 486),
            Self::Pal => (720, 576),
            Self::Hd720 => (1280, 720),
            Self::Hd1080 => (1920, 1080),
            Self::Dci2k => (2048, 1080),
            Self::Uhd4k => (3840, 2160),
            Self::Dci4k => (4096, 2160),
            Self::Uhd5k => (5120, 2880),
            Self::Uhd6k => (6144, 3456),
            Self::Uhd8k => (7680, 4320),
            Self::Dci8k => (8192, 4320),
            Self::Custom(w, h) => (w, h),
        }
    }

    pub const fn aspect_ratio(self) -> f32 {
        let (w, h) = self.dimensions();
        w as f32 / h as f32
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Ntsc => "NTSC",
            Self::Pal => "PAL",
            Self::Hd720 => "720p",
            Self::Hd1080 => "1080p",
            Self::Dci2k => "2K DCI",
            Self::Uhd4k => "4K UHD",
            Self::Dci4k => "4K DCI",
            Self::Uhd5k => "5K UHD",
            Self::Uhd6k => "6K UHD",
            Self::Uhd8k => "8K UHD",
            Self::Dci8k => "8K DCI",
            Self::Custom(_, _) => "Custom",
        }
    }
}

/// Frame rate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FrameRate {
    Fps23_976,
    Fps24,
    Fps25,
    Fps29_97,
    Fps30,
    Fps47_952,
    Fps48,
    Fps50,
    Fps59_94,
    Fps60,
    Fps72,
    Fps100,
    Fps119_88,
    Fps120,
    Custom(Rational),
}

impl FrameRate {
    pub const fn to_rational(self) -> Rational {
        match self {
            Self::Fps23_976 => Rational::new(24000, 1001),
            Self::Fps24 => Rational::new(24, 1),
            Self::Fps25 => Rational::new(25, 1),
            Self::Fps29_97 => Rational::new(30000, 1001),
            Self::Fps30 => Rational::new(30, 1),
            Self::Fps47_952 => Rational::new(48000, 1001),
            Self::Fps48 => Rational::new(48, 1),
            Self::Fps50 => Rational::new(50, 1),
            Self::Fps59_94 => Rational::new(60000, 1001),
            Self::Fps60 => Rational::new(60, 1),
            Self::Fps72 => Rational::new(72, 1),
            Self::Fps100 => Rational::new(100, 1),
            Self::Fps119_88 => Rational::new(120000, 1001),
            Self::Fps120 => Rational::new(120, 1),
            Self::Custom(r) => r,
        }
    }

    pub const fn fps_f32(self) -> f32 {
        let r = self.to_rational();
        r.num as f32 / r.den as f32
    }
}

/// Project settings - immutable once created
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSettings {
    pub name: String,
    pub resolution: Resolution,
    pub frame_rate: FrameRate,
    pub color_space: ColorSpace,
    pub pixel_format: PixelFormat,
    pub sample_rate: u32,
    pub bit_depth: u8,
    pub timeline_count: u32,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            name: "Untitled Project".into(),
            resolution: Resolution::Uhd4k,
            frame_rate: FrameRate::Fps24,
            color_space: ColorSpace::Rec709,
            pixel_format: PixelFormat::Rgba16F,
            sample_rate: 48000,
            bit_depth: 32,
            timeline_count: 0,
        }
    }
}

/// Timeline settings
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineSettings {
    pub name: String,
    pub resolution: Resolution,
    pub frame_rate: FrameRate,
    pub color_space: ColorSpace,
    pub pixel_format: PixelFormat,
    pub sample_rate: u32,
    pub start_timecode: Timecode,
    pub video_tracks: u32,
    pub audio_tracks: u32,
}

impl Default for TimelineSettings {
    fn default() -> Self {
        Self {
            name: "Timeline 1".into(),
            resolution: Resolution::Uhd4k,
            frame_rate: FrameRate::Fps24,
            color_space: ColorSpace::Rec709,
            pixel_format: PixelFormat::Rgba16F,
            sample_rate: 48000,
            start_timecode: Timecode::zero(FrameRate::Fps24),
            video_tracks: 4,
            audio_tracks: 8,
        }
    }
}

/// Clip metadata from media file
#[derive(Debug, Clone, Default)]
pub struct ClipMetadata {
    pub id: ClipId,
    /// Display name, normally the file stem.
    pub name: String,
    pub file_path: String,
    pub media_type: MediaType,
    pub duration: Frame,
    pub resolution: Option<Resolution>,
    pub frame_rate: Option<FrameRate>,
    pub pixel_format: Option<PixelFormat>,
    pub color_space: Option<ColorSpace>,
    pub audio_channels: Option<u16>,
    pub audio_sample_rate: Option<u32>,
    pub codec_name: Option<String>,
    pub bit_rate: Option<u64>,
    /// Size of the source file in bytes.
    pub size_bytes: u64,
    pub has_video: bool,
    pub has_audio: bool,
    pub tags: ClipTags,
    pub thumbnails: Vec<Thumbnail>,
}

impl ClipMetadata {
    /// Size of the source file in bytes, or 0 when unknown.
    pub fn file_size(&self) -> u64 {
        self.size_bytes
    }
}

/// User-editable clip tags
#[derive(Debug, Clone, Default)]
pub struct ClipTags {
    pub name: String,
    pub description: String,
    pub color_label: Option<ColorLabel>,
    pub rating: u8, // 0-5
    pub flags: ClipFlags,
    pub custom: std::collections::HashMap<String, String>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ClipFlags: u32 {
        const NONE = 0;
        const OFFLINE = 1 << 0;
        const PROXY = 1 << 1;
        const LOCKED = 1 << 2;
        const HIDDEN = 1 << 3;
        const AUDIO_ONLY = 1 << 4;
        const VIDEO_ONLY = 1 << 5;
        const ALPHA = 1 << 6;
        const HDR = 1 << 7;
        const STEREO = 1 << 8;
        const VR360 = 1 << 9;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorLabel {
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
pub struct Thumbnail {
    pub frame: Frame,
    pub data: Vec<u8>, // Compressed JPEG/PNG
    pub width: u16,
    pub height: u16,
}

/// Edit operation types for undo/redo
#[derive(Debug, Clone, PartialEq)]
pub enum EditOperation {
    // Clip operations
    AddClip {
        clip_id: ClipId,
        track: TrackId,
        position: Frame,
    },
    RemoveClip {
        item_id: TrackItemId,
    },
    MoveClip {
        item_id: TrackItemId,
        from: Frame,
        to: Frame,
        track: TrackId,
    },
    TrimClip {
        item_id: TrackItemId,
        edge: TrimEdge,
        old: Frame,
        new: Frame,
    },
    SplitClip {
        item_id: TrackItemId,
        position: Frame,
    },
    JoinClips {
        item_id_a: TrackItemId,
        item_id_b: TrackItemId,
    },

    // Track operations
    AddTrack {
        track_id: TrackId,
        track_type: TrackType,
        index: u32,
    },
    RemoveTrack {
        track_id: TrackId,
    },
    ReorderTrack {
        track_id: TrackId,
        new_index: u32,
    },
    ToggleTrackEnable {
        track_id: TrackId,
        enabled: bool,
    },
    ToggleTrackLock {
        track_id: TrackId,
        locked: bool,
    },
    ToggleTrackSolo {
        track_id: TrackId,
        solo: bool,
    },

    // Effect operations
    AddEffect {
        effect_id: EffectId,
        clip_id: ClipId,
        index: u32,
    },
    RemoveEffect {
        effect_id: EffectId,
    },
    ReorderEffect {
        effect_id: EffectId,
        new_index: u32,
    },
    UpdateEffectParams {
        effect_id: EffectId,
        params: EffectParams,
    },

    // Marker operations
    AddMarker {
        marker_id: MarkerId,
        position: Frame,
        data: MarkerData,
    },
    RemoveMarker {
        marker_id: MarkerId,
    },
    UpdateMarker {
        marker_id: MarkerId,
        data: MarkerData,
    },

    // Project operations
    UpdateProjectSettings {
        old: ProjectSettings,
        new: ProjectSettings,
    },
    UpdateTimelineSettings {
        old: TimelineSettings,
        new: TimelineSettings,
    },

    // Selection
    SetSelection {
        objects: Vec<ObjectId>,
    },
    ClearSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimEdge {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackType {
    Video,
    Audio,
    Subtitle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectParams {
    pub effect_type: EffectType,
    pub parameters: std::collections::HashMap<String, EffectValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectType {
    // Video
    Transform,
    Crop,
    Opacity,
    BlendMode,
    Speed,
    Stabilize,
    // Color (basic for edit page)
    LiftGammaGain,
    ContrastSaturation,
    ColorWheels,
    Curves,
    Lut3d,
    // Transitions
    CrossDissolve,
    DipToColor,
    Push,
    Slide,
    Wipe,
    // Audio
    Volume,
    Pan,
    Eq,
    Dynamics,
    Gate,
    Limiter,
    Delay,
    Reverb,
    Pitch,
    Filter,
    NoiseReduction,
    // Generators
    SolidColor,
    Text,
    Timecode,
    BarsAndTone,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectValue {
    Bool(bool),
    Int(i64),
    Float(f32),
    String(String),
    Vec2(f32, f32),
    Vec3(f32, f32, f32),
    Vec4(f32, f32, f32, f32),
    Color(f32, f32, f32, f32),
    Keyframes(Vec<Keyframe>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe {
    pub frame: Frame,
    pub value: EffectValue,
    pub interpolation: Interpolation,
    pub easing: Easing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    Linear,
    Bezier,
    Constant,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    None,
    EaseIn,
    EaseOut,
    EaseInOut,
    Custom(f32, f32, f32, f32), // cubic-bezier
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarkerData {
    pub name: String,
    pub note: String,
    pub color: ColorLabel,
    pub duration: Option<Frame>, // For range markers
    pub marker_type: MarkerType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerType {
    Chapter,
    WebLink,
    Note,
    Todo,
    Error,
    Warning,
    SyncPoint,
}

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Scrubbing,
    Looping,
}

/// Render quality preset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderQuality {
    Draft,
    Preview,
    High,
    Best,
    Custom,
}

/// Export codec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportCodec {
    H264,
    H265,
    ProRes422,
    ProRes422HQ,
    ProRes4444,
    DNxHD,
    DNxHR,
    VP9,
    AV1,
    EXR,
    DPX,
    TIFF,
    PNG,
    WAV,
    AAC,
    MP3,
    FLAC,
}
