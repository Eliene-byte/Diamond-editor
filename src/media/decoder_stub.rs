//! Fallback decoders for builds **without** the `codec-ffmpeg` feature.
//!
//! The real backends live in `decoder.rs` and require FFmpeg's shared libraries.
//! When the feature is off there is no way to open a container, so these types
//! keep the public API identical but fail every construction attempt with a clear
//! error. That lets the rest of the application (timeline, UI, GPU) compile and
//! be tested on machines with no codec toolchain installed.

use crate::core::color::ColorSpace;
use crate::core::color::PixelFormat;
use crate::core::error::{MediaError, Result};
use crate::core::id::ClipId;
use crate::core::time::{Frame, Rational};
use crate::core::types::FrameRate;
use std::path::Path;

pub use crate::media::frame::{
    AudioFormat, AudioFrame, AudioFrameData, VideoFrame, VideoFrameData,
};

fn no_backend() -> MediaError {
    MediaError::UnsupportedFormat(
        "no codec backend compiled in - rebuild with the `codec-ffmpeg` feature".to_string(),
    )
}

/// Video decoder stub. Every constructor fails; accessors report zeroed values.
pub struct VideoDecoder {
    clip_id: ClipId,
    width: u32,
    height: u32,
    format: PixelFormat,
    color_space: ColorSpace,
    frame_rate: Rational,
    duration: Frame,
}

impl VideoDecoder {
    pub fn new(clip_id: ClipId, _path: &Path) -> Result<Self> {
        Err(no_backend().into())
    }

    /// Build an in-memory decoder for tests. Has no backing codec, so it only
    /// produces empty frames.
    pub fn empty(clip_id: ClipId, width: u32, height: u32, frame_rate: FrameRate) -> Self {
        Self {
            clip_id,
            width,
            height,
            format: PixelFormat::Rgba8,
            color_space: ColorSpace::sRGB,
            frame_rate: frame_rate.to_rational(),
            duration: Frame(0),
        }
    }

    pub fn decode_frame(&mut self, _frame_num: Frame) -> Result<Option<VideoFrame>> {
        Err(no_backend().into())
    }

    pub fn decode_range(&mut self, _start: Frame, _end: Frame) -> Result<Vec<VideoFrame>> {
        Err(no_backend().into())
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn format(&self) -> PixelFormat {
        self.format
    }
    pub fn color_space(&self) -> ColorSpace {
        self.color_space
    }
    pub fn frame_rate(&self) -> Rational {
        self.frame_rate
    }
    pub fn duration(&self) -> Frame {
        self.duration
    }
    pub fn clip_id(&self) -> ClipId {
        self.clip_id
    }
}

/// Audio decoder stub. See [`VideoDecoder`] for the rationale.
pub struct AudioDecoder {
    sample_rate: u32,
    channels: u16,
    duration: Frame,
}

impl AudioDecoder {
    pub fn new(_clip_id: ClipId, _path: &Path, _target_format: AudioFormat) -> Result<Self> {
        Err(no_backend().into())
    }

    /// Build a silent in-memory decoder for tests.
    pub fn empty(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
            duration: Frame(0),
        }
    }

    pub fn decode_all(&mut self) -> Result<Vec<AudioFrame>> {
        Err(no_backend().into())
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn channels(&self) -> u16 {
        self.channels
    }
    pub fn duration(&self) -> Frame {
        self.duration
    }
}

/// Combined audio+video decoder stub.
pub struct MediaDecoder {
    video: Option<VideoDecoder>,
    audio: Option<AudioDecoder>,
}

impl MediaDecoder {
    pub fn new(_clip_id: ClipId, _path: &Path) -> Result<Self> {
        Err(no_backend().into())
    }

    /// Construct a decoder holding no streams, for tests.
    pub fn empty() -> Self {
        Self {
            video: None,
            audio: None,
        }
    }

    pub fn video(&mut self) -> Option<&mut VideoDecoder> {
        self.video.as_mut()
    }
    pub fn audio(&mut self) -> Option<&mut AudioDecoder> {
        self.audio.as_mut()
    }
    pub fn has_video(&self) -> bool {
        self.video.is_some()
    }
    pub fn has_audio(&self) -> bool {
        self.audio.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_report_missing_backend() {
        let path = Path::new("does-not-exist.mov");
        assert!(VideoDecoder::new(ClipId(1), path).is_err());
        assert!(AudioDecoder::new(ClipId(1), path, AudioFormat::F32).is_err());
        assert!(MediaDecoder::new(ClipId(1), path).is_err());
    }

    #[test]
    fn empty_decoder_reports_its_shape() {
        let mut d = VideoDecoder::empty(ClipId(1), 1920, 1080, FrameRate::Fps24);
        assert_eq!(d.width(), 1920);
        assert_eq!(d.height(), 1080);
        assert!(d.decode_frame(Frame(0)).is_err());
    }
}
