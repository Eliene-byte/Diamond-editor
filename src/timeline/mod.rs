//! Timeline editing engine - the heart of the edit page

pub mod clip;
pub mod edit;
pub mod markers;
pub mod playhead;
pub mod selection;
pub mod track;
pub mod transitions;
pub mod undo;

// Re-exports. Note: `Selection` and `Marker` are defined in both clip.rs and
// selection.rs / markers.rs, so they are intentionally NOT glob-exported here.
// Consumers should use the fully qualified paths:
//   - `crate::timeline::clip::Selection`  (timeline's internal selection state)
//   - `crate::timeline::selection::Selection` / `...SelectionTool`
//   - `crate::timeline::clip::Marker`      (timeline's marker record)
//   - `crate::timeline::markers::Marker`    (marker manager's marker type)
// Likewise `TrackItemEffect` is defined in both track.rs and clip.rs; track.rs is
// the canonical definition and clip.rs re-exports a compatible alias internally.

pub use clip::{Timeline, TimelineClip, TimelineSettings};
pub use edit::{EditMode, EditState, Editor};
pub use markers::{ChapterExport, Marker as ManagedMarker, MarkerManager, MarkerUI};
pub use playhead::{PlaybackEvent, Playhead, PlayheadController};
pub use selection::{
    DragResult, MarqueeRect, SelectionModifiers, SelectionNavigation, SelectionTool,
};
pub use track::{
    AudioTrack, BlendMode, SubtitleTrack, TrackAutomation, TrackColor, TrackContainer, TrackItem,
    TrackItemColor, TrackItemEffect as TrackEffect, TrackItemKey, TrackItemMetadata, VideoTrack,
};
pub use transitions::{
    Transition, TransitionAlignment, TransitionManager, TransitionRenderer, TransitionType,
};
pub use undo::{Command, EffectData, ItemData, MarkerData, Snapshot, StateCommand, UndoManager};
