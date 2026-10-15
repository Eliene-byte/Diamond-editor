//! Core types and architecture for DaVinci Clone
//! Minimal, zero-cost abstractions for video editing

pub mod types;
pub mod time;
pub mod color;
pub mod math;
pub mod id;
pub mod error;
pub mod config;

// `ObjectId` is declared in both `types` and `id`. `id` is the canonical home
// (it sits next to the `typed_id!` macro), so `types::ObjectId` is aliased here
// rather than glob-exported twice, which would make `crate::core::*` ambiguous.

pub use types::{
    MediaType, Resolution, FrameRate, ProjectSettings, TimelineSettings as CoreTimelineSettings,
    ClipMetadata, ClipTags, ClipFlags, ColorLabel, Thumbnail, EditOperation, TrimEdge,
    TrackType, EffectParams, EffectType, EffectValue, Keyframe, Interpolation, Easing,
    MarkerData, MarkerType, PlaybackState, RenderQuality, ExportCodec,
};
pub use time::{Frame, Rational, Timecode, Duration, FrameRange, PlaybackRange, TimelinePosition};
pub use color::{
    ColorSpace, PixelFormat, Primaries, TransferFunction, ColorTransform, Lut1D, Lut3D, Cdl,
    GradeNode, GradeOp, Curve, CurveInterp, WindowShape, WindowTransform, TrackPoint,
    GradeCache, ScopesData, BlurShape,
};
pub use math::{Point2, Rect, Transform2D, Bvh2D, FastRand};
pub use id::{
    ProjectId, TimelineId, BinId, ClipId, MediaId, ProxyId, ThumbnailId, TrackId,
    TrackItemId, TransitionId, EffectId, GradeNodeId, LutId, PluginId, MarkerId,
    AnnotationId, AudioBusId, AudioPluginId, RenderJobId, ExportPresetId, WindowId,
    PanelId, WidgetId, CacheEntryId, CommandId, ObjectId, CompoundId, IdAllocator,
    id_map,
};
pub use error::{AppError, Result, MediaError, TimelineError, ProjectError, GpuError,
    CodecError, AudioError, UiError, CommandError, ConfigError, CacheError, PluginError};
pub use config::{AppConfig, ConfigManager};