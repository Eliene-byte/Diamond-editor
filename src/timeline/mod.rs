//! Timeline editing engine - the heart of the edit page

pub mod track;
pub mod clip;
pub mod edit;
pub mod playhead;
pub mod selection;
pub mod undo;
pub mod markers;
pub mod transitions;

// Re-exports. Note: `Selection` and `Marker` are defined in both clip.rs and
// selection.rs / markers.rs, so they are intentionally NOT glob-exported here.
// Consumers should use the fully qualified paths:
//   - `crate::timeline::clip::Selection`  (timeline's internal selection state)
//   - `crate::timeline::selection::Selection` / `...SelectionTool`
//   - `crate::timeline::clip::Marker`      (timeline's marker record)
//   - `crate::timeline::markers::Marker`    (marker manager's marker type)
// Likewise `TrackItemEffect` is defined in both track.rs and clip.rs; track.rs is
// the canonical definition and clip.rs re-exports a compatible alias internally.

pub use track::{
    TrackItem, TrackItemKey, TrackItemColor, TrackItemMetadata, TrackItemEffect as TrackEffect,
    BlendMode, VideoTrack, AudioTrack, SubtitleTrack, TrackColor, TrackContainer,
    TrackAutomation,
};
pub use clip::{Timeline, TimelineSettings, TimelineClip};
pub use edit::{EditMode, EditState, Editor};
pub use playhead::{Playhead, PlaybackEvent, PlayheadController};
pub use selection::{SelectionTool, SelectionModifiers, MarqueeRect, DragResult, SelectionNavigation};
pub use undo::{Command, StateCommand, Snapshot, ItemData, EffectData, MarkerData, UndoManager};
pub use markers::{MarkerManager, Marker as ManagedMarker, ChapterExport, MarkerUI};
pub use transitions::{Transition, TransitionType, TransitionAlignment, TransitionManager, TransitionRenderer};