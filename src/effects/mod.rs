//! Video and audio effects

pub mod video;
pub mod audio;
pub mod transitions;
pub mod generators;

// Explicit re-exports. `Transition`/`TransitionType` are canonical in
// `timeline::transitions` and re-exported through `transitions` here for
// convenience; they are listed once to avoid glob ambiguity.

pub use video::{VideoEffect, EffectStack, VideoEffectRenderer};
pub use audio::{AudioEffect, AudioEffectChain};
pub use generators::{Generator, GeneratorType, GeneratorRegistry};
pub use transitions::{
    TransitionType, Transition, default_transition_duration, min_transition_duration,
    max_transition_duration, clamp_transition_duration, is_audio_transition,
    is_video_transition, transition_label, make_transition, TRANSITION_PALETTE,
};