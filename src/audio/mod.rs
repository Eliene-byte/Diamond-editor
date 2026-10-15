//! Audio system.
//!
//! `dsp` is pure Rust and always available. `engine` reaches the sound card via
//! the [`device`] facade, which compiles to a no-op without the `audio-cpal`
//! feature, so headless builds and CI keep working.

pub mod dsp;
pub mod device;
pub mod engine;
pub mod mixer;

pub use dsp::{Analyzer, Biquad, Compressor, DelayLine, EQ, FilterType, Gate, Limiter};
pub use device::{StreamConfig, open_output_stream, Stream};
pub use engine::{AudioBuffer, AudioEngine};
pub use mixer::AudioMixer;