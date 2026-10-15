//! Core types and architecture for DaVinci Clone
//! Minimal, zero-cost abstractions for video editing

pub mod color;
pub mod config;
pub mod error;
pub mod id;
pub mod math;
pub mod time;
pub mod types;

// `ObjectId` is declared in both `types` and `id`. `id` is the canonical home
// (it sits next to the `typed_id!` macro), so `types::ObjectId` is aliased here
// rather than glob-exported twice, which would make `crate::core::*` ambiguous.

pub use color::{
    BlurShape, Cdl, ColorSpace, ColorTransform, Curve, CurveInterp, GradeCache, GradeNode, GradeOp,
    Lut1D, Lut3D, PixelFormat, Primaries, ScopesData, TrackPoint, TransferFunction, WindowShape,
    WindowTransform,
};
pub use config::{AppConfig, ConfigManager};
pub use error::{
    AppError, AudioError, CacheError, CodecError, CommandError, ConfigError, GpuError, MediaError,
    PluginError, ProjectError, Result, TimelineError, UiError,
};
pub use id::{
    id_map, AnnotationId, AudioBusId, AudioPluginId, BinId, CacheEntryId, ClipId, CommandId,
    CompoundId, EffectId, ExportPresetId, GradeNodeId, IdAllocator, LutId, MarkerId, MediaId,
    ObjectId, PanelId, PluginId, ProjectId, ProxyId, RenderJobId, ThumbnailId, TimelineId, TrackId,
    TrackItemId, TransitionId, WidgetId, WindowId,
};
pub use math::{Bvh2D, FastRand, Point2, Rect, Transform2D};
pub use time::{Duration, Frame, FrameRange, PlaybackRange, Rational, Timecode, TimelinePosition};
pub use types::{
    ClipFlags, ClipMetadata, ClipTags, ColorLabel, Easing, EditOperation, EffectParams, EffectType,
    EffectValue, ExportCodec, FrameRate, Interpolation, Keyframe, MarkerData, MarkerType,
    MediaType, PlaybackState, ProjectSettings, RenderQuality, Resolution, Thumbnail,
    TimelineSettings as CoreTimelineSettings, TrackType, TrimEdge,
};
