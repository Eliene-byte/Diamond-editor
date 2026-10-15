//! Decoded frame containers.
//!
//! These types are the crate's own representation of a decoded video or audio
//! frame and deliberately do **not** depend on any codec backend, so they stay
//! available in every feature configuration. The backends that produce them
//! live in [`crate::media::decoder`] and are gated behind `codec-ffmpeg`.

use crate::core::color::ColorSpace;
use crate::core::color::PixelFormat;
use crate::core::id::ClipId;
use crate::core::time::Frame;
use std::sync::Arc;

/// Decoded video frame
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub clip_id: ClipId,
    pub frame: Frame,
    pub pts: i64,
    pub dts: i64,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub color_space: ColorSpace,
    pub data: Arc<VideoFrameData>,
    pub keyframe: bool,
    pub duration: Frame,
}

/// Raw planar pixel data backing a [`VideoFrame`].
#[derive(Debug)]
pub struct VideoFrameData {
    pub planes: [Vec<u8>; 4],
    pub strides: [usize; 4],
    pub format: PixelFormat,
}

impl VideoFrameData {
    /// Allocate zeroed plane buffers sized for `format` at `width` x `height`.
    pub fn new(format: PixelFormat, width: u32, height: u32) -> Self {
        let (plane_count, planes, strides) = match format {
            PixelFormat::Rgba8 | PixelFormat::Rgb8 => {
                let stride = (width * 4) as usize;
                let size = stride * height as usize;
                (
                    1,
                    [vec![0; size], Vec::new(), Vec::new(), Vec::new()],
                    [stride, 0, 0, 0],
                )
            }
            PixelFormat::Rgba16F => {
                let stride = (width * 8) as usize;
                let size = stride * height as usize;
                (
                    1,
                    [vec![0; size], Vec::new(), Vec::new(), Vec::new()],
                    [stride, 0, 0, 0],
                )
            }
            PixelFormat::Rgba32F => {
                let stride = (width * 16) as usize;
                let size = stride * height as usize;
                (
                    1,
                    [vec![0; size], Vec::new(), Vec::new(), Vec::new()],
                    [stride, 0, 0, 0],
                )
            }
            PixelFormat::Yuv420_8 | PixelFormat::Yuv420_10 | PixelFormat::Yuv420_12 => {
                let y_stride = width as usize;
                let uv_stride = (width / 2) as usize;
                let y_size = y_stride * height as usize;
                let uv_size = uv_stride * (height / 2) as usize;
                (
                    3,
                    [
                        vec![0; y_size],
                        vec![0; uv_size],
                        vec![0; uv_size],
                        Vec::new(),
                    ],
                    [y_stride, uv_stride, uv_stride, 0],
                )
            }
            PixelFormat::Yuv422_8 | PixelFormat::Yuv422_10 | PixelFormat::Yuv422_12 => {
                let y_stride = width as usize;
                let uv_stride = (width / 2) as usize;
                let y_size = y_stride * height as usize;
                let uv_size = uv_stride * height as usize;
                (
                    3,
                    [
                        vec![0; y_size],
                        vec![0; uv_size],
                        vec![0; uv_size],
                        Vec::new(),
                    ],
                    [y_stride, uv_stride, uv_stride, 0],
                )
            }
            PixelFormat::Yuv444_8 | PixelFormat::Yuv444_10 | PixelFormat::Yuv444_12 => {
                let stride = width as usize;
                let size = stride * height as usize;
                (
                    3,
                    [vec![0; size], vec![0; size], vec![0; size], Vec::new()],
                    [stride, stride, stride, 0],
                )
            }
            PixelFormat::P010 => {
                let y_stride = width as usize;
                let uv_stride = (width / 2) as usize;
                let y_size = y_stride * height as usize;
                let uv_size = uv_stride * (height / 2) as usize * 2; // 10-bit packed
                (
                    2,
                    [vec![0; y_size], vec![0; uv_size], Vec::new(), Vec::new()],
                    [y_stride, uv_stride, 0, 0],
                )
            }
            _ => {
                let stride = (width * 4) as usize;
                let size = stride * height as usize;
                (
                    1,
                    [vec![0; size], Vec::new(), Vec::new(), Vec::new()],
                    [stride, 0, 0, 0],
                )
            }
        };
        Self {
            planes,
            strides,
            format,
        }
    }

    pub fn plane(&self, i: usize) -> &[u8] {
        &self.planes[i]
    }

    pub fn plane_mut(&mut self, i: usize) -> &mut [u8] {
        &mut self.planes[i]
    }

    pub fn stride(&self, i: usize) -> usize {
        self.strides[i]
    }

    /// Number of planes that actually carry data for this format.
    pub fn plane_count(&self) -> usize {
        self.strides.iter().filter(|s| **s > 0).count()
    }
}

/// Decoded audio frame
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub clip_id: ClipId,
    pub frame: Frame,
    pub pts: i64,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: AudioFormat,
    pub data: Arc<AudioFrameData>,
    pub duration: Frame,
}

/// Sample encodings the audio subsystem can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    U8,
    I16,
    I32,
    F32,
    F64,
}

impl AudioFormat {
    /// Bytes per sample for a single channel.
    pub fn bytes_per_sample(&self) -> usize {
        match self {
            AudioFormat::U8 => 1,
            AudioFormat::I16 => 2,
            AudioFormat::I32 | AudioFormat::F32 => 4,
            AudioFormat::F64 => 8,
        }
    }

    /// The format the mixer works in internally.
    pub fn to_float(&self) -> AudioFormat {
        AudioFormat::F32
    }
}

/// Interleaved-by-channel sample data, normalised to `f32`.
#[derive(Debug)]
pub struct AudioFrameData {
    pub channels: Vec<Vec<f32>>,
    pub samples: usize,
}

impl AudioFrameData {
    pub fn new(channels: u16, samples: usize) -> Self {
        Self {
            channels: vec![vec![0.0; samples]; channels as usize],
            samples,
        }
    }

    /// A single channel, or an empty slice when the index is out of range.
    pub fn channel(&self, index: usize) -> &[f32] {
        self.channels
            .get(index)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Zero every sample while keeping the channel layout.
    pub fn silence(&mut self) {
        for channel in &mut self.channels {
            channel.iter_mut().for_each(|s| *s = 0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba8_uses_a_single_plane() {
        let data = VideoFrameData::new(PixelFormat::Rgba8, 16, 8);
        assert_eq!(data.plane_count(), 1);
        assert_eq!(data.stride(0), 16 * 4);
        assert_eq!(data.plane(0).len(), 16 * 4 * 8);
    }

    #[test]
    fn yuv420_uses_three_planes_with_halved_chroma() {
        let data = VideoFrameData::new(PixelFormat::Yuv420_8, 16, 8);
        assert_eq!(data.plane_count(), 3);
        assert_eq!(data.stride(0), 16);
        assert_eq!(data.stride(1), 8);
        assert_eq!(data.plane(1).len(), 8 * 4);
    }

    #[test]
    fn audio_format_sizes_match_variants() {
        assert_eq!(AudioFormat::U8.bytes_per_sample(), 1);
        assert_eq!(AudioFormat::I16.bytes_per_sample(), 2);
        assert_eq!(AudioFormat::F32.bytes_per_sample(), 4);
        assert_eq!(AudioFormat::F64.bytes_per_sample(), 8);
    }

    #[test]
    fn silence_preserves_layout() {
        let mut data = AudioFrameData::new(2, 4);
        data.channels[0][0] = 1.0;
        data.silence();
        assert_eq!(data.channels.len(), 2);
        assert_eq!(data.samples, 4);
        assert!(data.channels.iter().all(|c| c.iter().all(|s| *s == 0.0)));
        assert!(data.channel(9).is_empty(), "out-of-range channel is empty");
    }
}
