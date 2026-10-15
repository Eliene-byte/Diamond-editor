//! Fallback audio-waveform support for builds **without** the `codec-ffmpeg`
//! feature.
//!
//! Extracting peaks requires an audio decoder, so without one the generator
//! always reports an error. The API mirrors `waveform.rs` so the rest of the
//! crate compiles unchanged.

use crate::core::id::ClipId;
use crate::core::time::Frame;
use crate::core::error::{AppError, MediaError, Result};
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveformLevel {
    Low,
    Medium,
    High,
}

impl WaveformLevel {
    /// Samples represented per bucket at this level, assuming a 48 kHz source.
    pub fn samples_per_bucket(&self) -> usize {
        match self {
            WaveformLevel::Low => 1024,
            WaveformLevel::Medium => 256,
            WaveformLevel::High => 64,
        }
    }
}

/// Min/max amplitude pair for one bucket of audio.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WaveformPeak {
    pub min: f32,
    pub max: f32,
}

impl WaveformPeak {
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    /// Absolute peak amplitude in `0.0..=1.0`.
    pub fn peak(&self) -> f32 {
        self.min.abs().max(self.max.abs()).min(1.0)
    }

    pub fn is_silent(&self) -> bool {
        self.peak() <= f32::EPSILON
    }
}

/// Extracted waveform for a clip.
#[derive(Debug, Clone, Default)]
pub struct Waveform {
    pub clip_id: ClipId,
    pub sample_rate: u32,
    pub channels: u16,
    pub peaks: Vec<WaveformPeak>,
}

impl Waveform {
    /// Number of buckets in the waveform.
    pub fn len(&self) -> usize {
        self.peaks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peaks.is_empty()
    }

    /// Loudest absolute amplitude across all buckets.
    pub fn peak(&self) -> f32 {
        self.peaks.iter().map(|p| p.peak()).fold(0.0, f32::max)
    }

    /// Time range covered by the waveform.
    pub fn duration(&self) -> Frame {
        if self.sample_rate == 0 || self.peaks.is_empty() {
            return Frame(0);
        }
        // 1 bucket == 1/60 s of audio, matching the display resolution.
        Frame((self.peaks.len() as i64 * i64::from(self.sample_rate)) / 60)
    }
}

/// Generates waveforms from audio tracks.
#[derive(Debug, Clone, Copy, Default)]
pub struct WaveformGenerator {
    /// Buckets per second of audio to produce.
    pub resolution: u32,
}

impl WaveformGenerator {
    pub fn new() -> Self {
        Self { resolution: 1024 }
    }

    /// Always fails: decoding audio needs a codec backend.
    pub fn generate(&self, _path: &Path) -> Result<Waveform> {
        Err(AppError::Media(MediaError::UnsupportedFormat(
            "no codec backend compiled in - rebuild with the `codec-ffmpeg` feature".to_string(),
        )))
    }
}

/// Multi-resolution waveform pyramid, used by the timeline at different zooms.
#[derive(Debug, Clone, Default)]
pub struct MultiResolutionWaveform {
    pub clip_id: ClipId,
    pub levels: Vec<(WaveformLevel, Waveform)>,
}

impl MultiResolutionWaveform {
    /// Pick the coarsest level that still has at least `buckets` entries.
    pub fn level_for(&self, buckets: usize) -> Option<&Waveform> {
        self.levels
            .iter()
            .filter(|(_, w)| w.len() >= buckets)
            .min_by_key(|(lvl, _)| lvl.samples_per_bucket())
            .map(|(_, w)| w)
    }
}

/// In-memory cache of extracted waveforms.
#[derive(Debug)]
pub struct WaveformCache {
    max_entries: usize,
    entries: Mutex<Vec<(ClipId, Waveform)>>,
}

impl WaveformCache {
    pub fn new(max_entries: usize) -> Self {
        Self { max_entries, entries: Mutex::new(Vec::new()) }
    }

    pub fn get(&self, clip_id: ClipId) -> Option<Waveform> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.iter().find(|(c, _)| *c == clip_id).map(|(_, w)| w.clone())
    }

    pub fn put(&self, clip_id: ClipId, waveform: Waveform) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|(c, _)| *c != clip_id);
        if self.max_entries > 0 && entries.len() >= self.max_entries {
            entries.remove(0);
        }
        entries.push((clip_id, waveform));
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wave(clip: u64, peaks: usize) -> Waveform {
        Waveform {
            clip_id: ClipId(clip),
            sample_rate: 48_000,
            channels: 2,
            peaks: (0..peaks).map(|i| WaveformPeak::new(-0.5, 0.25 + i as f32 * 0.01)).collect(),
        }
    }

    #[test]
    fn generation_reports_missing_backend() {
        assert!(WaveformGenerator::default().generate(Path::new("a.wav")).is_err());
    }

    #[test]
    fn peak_is_clamped_to_one() {
        assert_eq!(WaveformPeak::new(-2.0, 2.0).peak(), 1.0);
        assert!(WaveformPeak::new(0.0, 0.0).is_silent());
    }

    #[test]
    fn duration_scales_with_sample_rate() {
        assert_eq!(wave(1, 60).duration(), Frame(48_000));
        assert_eq!(Waveform::default().duration(), Frame(0));
    }

    #[test]
    fn cache_evicts_oldest_entry() {
        let cache = WaveformCache::new(1);
        cache.put(ClipId(1), wave(1, 4));
        cache.put(ClipId(2), wave(2, 4));
        assert!(cache.get(ClipId(1)).is_none());
        assert!(cache.get(ClipId(2)).is_some());
    }
}