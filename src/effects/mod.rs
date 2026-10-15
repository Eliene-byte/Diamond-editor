//! Video and audio effects

pub mod audio;
pub mod generators;
pub mod transitions;
pub mod video;

// Explicit re-exports. `Transition`/`TransitionType` are canonical in
// `timeline::transitions` and re-exported through `transitions` here for
// convenience; they are listed once to avoid glob ambiguity.

pub use audio::{AudioEffect, AudioEffectChain};
pub use generators::{Generator, GeneratorRegistry, GeneratorType};
pub use transitions::{
    clamp_transition_duration, default_transition_duration, is_audio_transition,
    is_video_transition, make_transition, max_transition_duration, min_transition_duration,
    transition_label, Transition, TransitionType, TRANSITION_PALETTE,
};
pub use video::{EffectStack, VideoEffect, VideoEffectRenderer};
