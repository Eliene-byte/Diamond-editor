//! Audio system

pub mod dsp;
pub mod engine;
pub mod mixer;

// Explicit re-exports. Only public items are listed: `engine::AudioTrack`,
// `engine::AudioBus`, `engine::AudioEffect` and `dsp::EQBand` are crate-internal.

pub use dsp::{Biquad, DelayLine, Compressor, Gate, Limiter, EQ, Analyzer, FilterType};
pub use engine::{AudioEngine, AudioBuffer};
pub use mixer::AudioMixer;