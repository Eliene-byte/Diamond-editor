//! Fallback container probing for builds **without** the `codec-ffmpeg`
//! feature.
//!
//! Reading stream information requires a demuxer, so without one every probe
//! fails. The types mirror `metadata.rs` so dependent code still compiles, and
//! [`MediaMetadata::from_file`] reports a clear error instead of panicking.

use crate::core::id::ClipId;
use crate::core::time::Frame;
use crate::core::error::{AppError, MediaError, Result};
use std::path::Path;

/// Exact-value rational time base, as containers report it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rational {
    pub num: i64,
    pub den: i64,
}

impl Rational {
    pub const fn new(num: i64, den: i64) -> Self {
        Self { num, den }
    }

    pub fn as_f64(&self) -> f64 {
        if self.den == 0 {
            0.0
        } else {
            self.num as f64 / self.den as f64
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct VideoStreamMetadata {
    pub index: i32,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: Rational,
    pub bitrate: u64,
    pub pixel_format: String,
    pub duration: Frame,
}

impl VideoStreamMetadata {
    /// Display aspect ratio, derived from pixel dimensions.
    pub fn display_aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            return 0.0;
        }
        self.width as f32 / self.height as f32
    }
}

#[derive(Debug, Clone, Default)]
pub struct AudioStreamMetadata {
    pub index: i32,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub bitrate: u64,
    pub duration: Frame,
}

impl AudioStreamMetadata {
    pub fn channel_layout(&self) -> String {
        match self.channels {
            1 => "mono".to_string(),
            2 => "stereo".to_string(),
            6 => "5.1".to_string(),
            8 => "7.1".to_string(),
            other => format!("{other} channels"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SubtitleStreamMetadata {
    pub index: i32,
    pub codec: String,
    pub language: String,
    pub forced: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ChapterMetadata {
    pub start: Frame,
    pub end: Frame,
    pub title: String,
}

/// Everything probed from a media file.
#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    pub clip_id: ClipId,
    pub path: String,
    pub container: String,
    pub duration: Frame,
    pub size_bytes: u64,
    pub video_streams: Vec<VideoStreamMetadata>,
    pub audio_streams: Vec<AudioStreamMetadata>,
    pub subtitle_streams: Vec<SubtitleStreamMetadata>,
    pub chapters: Vec<ChapterMetadata>,
    pub creation_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl MediaMetadata {
    /// Always fails: probing a container needs a demuxer.
    pub fn from_file(_path: &Path) -> Result<Self> {
        Err(AppError::Media(MediaError::MetadataFailed(
            "no codec backend compiled in - rebuild with the `codec-ffmpeg` feature".to_string(),
        )))
    }

    pub fn has_video(&self) -> bool {
        !self.video_streams.is_empty()
    }

    pub fn has_audio(&self) -> bool {
        !self.audio_streams.is_empty()
    }

    /// Primary video stream, if any.
    pub fn primary_video(&self) -> Option<&VideoStreamMetadata> {
        self.video_streams.first()
    }

    /// Primary audio stream, if any.
    pub fn primary_audio(&self) -> Option<&AudioStreamMetadata> {
        self.audio_streams.first()
    }

    /// Project a probe result into the lightweight record the timeline stores.
    pub fn to_clip_metadata(&self, clip_id: ClipId) -> crate::core::types::ClipMetadata {
        use crate::core::types::ClipMetadata;

        let name = std::path::Path::new(&self.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string();

        let mut metadata = ClipMetadata::default();
        metadata.name = name;
        metadata.file_path = self.path.clone();
        metadata.duration = self.duration;
        metadata.has_video = self.has_video();
        metadata.has_audio = self.has_audio();
        let _ = clip_id;
        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probing_reports_missing_backend() {
        assert!(MediaMetadata::from_file(Path::new("a.mov")).is_err());
    }

    #[test]
    fn rational_guards_against_zero_denominator() {
        assert_eq!(Rational::new(30000, 1001).as_f64(), 30000.0 / 1001.0);
        assert_eq!(Rational::new(1, 0).as_f64(), 0.0);
    }

    #[test]
    fn stream_accessors_handle_empty_metadata() {
        let m = MediaMetadata::default();
        assert!(!m.has_video());
        assert!(!m.has_audio());
        assert!(m.primary_video().is_none());
    }

    #[test]
    fn audio_channel_layout_is_named() {
        let a = AudioStreamMetadata { channels: 6, ..Default::default() };
        assert_eq!(a.channel_layout(), "5.1");
    }
}