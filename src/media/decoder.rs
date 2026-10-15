//! Video/Audio frame decoding with frame-accurate seeking

use crate::core::color::ColorSpace;
use crate::core::error::{CodecError, MediaError, Result};
use crate::core::id::{ClipId, MediaId};
use crate::core::time::{Frame, Rational, Timecode};
use crate::core::types::{FrameRate, MediaType, PixelFormat};
use ffmpeg_next as ffmpeg;
use parking_lot::RwLock;
use std::path::Path;
use std::sync::Arc;

// Frame containers live in crate::media::frame so they remain available
// regardless of which codec backend is compiled in. They are re-exported
// here to keep the historical media::decoder::{VideoFrame, ..} paths working.
pub use crate::media::frame::{
    AudioFormat, AudioFrame, AudioFrameData, VideoFrame, VideoFrameData,
};

/// Video decoder with frame-accurate seeking
pub struct VideoDecoder {
    clip_id: ClipId,
    path: String,
    ictx: Option<ffmpeg::format::context::Input>,
    video_stream_index: usize,
    codec_ctx: Option<ffmpeg::codec::context::Context>,
    frame_rate: Rational,
    time_base: Rational,
    duration: Frame,
    width: u32,
    height: u32,
    format: PixelFormat,
    color_space: ColorSpace,
    seek_cache: RwLock<SeekCache>,
    thread_count: usize,
}

struct SeekCache {
    last_keyframe: Option<(Frame, i64)>, // (frame, pts)
    keyframes: Vec<(Frame, i64)>,        // Sorted by frame
}

impl VideoDecoder {
    pub fn new(clip_id: ClipId, path: &Path) -> Result<Self> {
        ffmpeg::init()?;

        let mut ictx =
            ffmpeg::format::input(&path).map_err(|e| MediaError::Decoder(e.to_string()))?;

        let video_stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(MediaError::NoVideoStream)?;

        let stream_index = video_stream.index();
        let codec_ctx = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            .map_err(|e| MediaError::Decoder(e.to_string()))?;

        let mut decoder = codec_ctx
            .decoder()
            .video()
            .map_err(|e| MediaError::Decoder(e.to_string()))?;

        // Enable multi-threading
        decoder.set_threading(ffmpeg::threading::Config {
            kind: ffmpeg::threading::Type::Frame,
            count: num_cpus::get(),
        });

        let time_base = video_stream.time_base();
        let frame_rate = video_stream.avg_frame_rate();
        let duration = video_stream.duration().unwrap_or(0);

        let width = decoder.width();
        let height = decoder.height();
        let format = pixel_format_from_ffmpeg(decoder.format());
        let color_space = color_space_from_ffmpeg(decoder.color_space(), decoder.color_range());

        Ok(Self {
            clip_id,
            path: path.to_string_lossy().to_string(),
            ictx: Some(ictx),
            video_stream_index: stream_index,
            codec_ctx: Some(decoder),
            frame_rate: Rational::new(frame_rate.0 as i64, frame_rate.1 as i64),
            time_base: Rational::new(time_base.0 as i64, time_base.1 as i64),
            duration: Frame(duration),
            width,
            height,
            format,
            color_space,
            seek_cache: RwLock::new(SeekCache {
                last_keyframe: None,
                keyframes: Vec::new(),
            }),
            thread_count: num_cpus::get(),
        })
    }

    pub fn decode_frame(&mut self, frame_num: Frame) -> Result<Option<VideoFrame>> {
        let pts = self.frame_to_pts(frame_num);
        self.seek_to_pts(pts)?;

        let mut frame = None;
        while frame.is_none() {
            let mut packet = match self.read_packet()? {
                Some(p) => p,
                None => break,
            };

            if packet.stream() != self.video_stream_index {
                continue;
            }

            self.codec_ctx
                .as_mut()
                .unwrap()
                .send_packet(&packet)
                .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;

            let mut decoded = ffmpeg::frame::Video::empty();
            while self
                .codec_ctx
                .as_mut()
                .unwrap()
                .receive_frame(&mut decoded)
                .is_ok()
            {
                if decoded.pts() == Some(pts) || decoded.pts().is_none() {
                    frame = Some(self.convert_frame(decoded, frame_num)?);
                    break;
                }
            }
        }

        Ok(frame)
    }

    pub fn decode_range(&mut self, start: Frame, end: Frame) -> Result<Vec<VideoFrame>> {
        let mut frames = Vec::with_capacity((end.0 - start.0) as usize);
        self.seek_to_pts(self.frame_to_pts(start))?;

        let mut frame = ffmpeg::frame::Video::empty();
        // Tracks the last frame index seen, so we can stop once decoding passes `end`.
        let mut last_frame = start;
        let mut reached_end = false;

        while let Some(packet) = self.read_packet()? {
            if packet.stream() != self.video_stream_index {
                continue;
            }

            self.codec_ctx
                .as_mut()
                .unwrap()
                .send_packet(&packet)
                .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;

            while self
                .codec_ctx
                .as_mut()
                .unwrap()
                .receive_frame(&mut frame)
                .is_ok()
            {
                let frame_num = self.pts_to_frame(frame.pts().unwrap_or(0));
                if frame_num < start {
                    continue;
                }
                if frame_num >= end {
                    reached_end = true;
                    break;
                }

                frames.push(self.convert_frame(frame, frame_num)?);
                last_frame = frame_num;
                frame = ffmpeg::frame::Video::empty();
            }
            if reached_end || last_frame >= end {
                break;
            }
        }

        Ok(frames)
    }

    fn seek_to_pts(&mut self, pts: i64) -> Result<()> {
        self.ictx
            .as_mut()
            .unwrap()
            .seek(pts, ..pts)
            .map_err(|e| MediaError::SeekFailed(e.to_string()))?;

        if let Some(ctx) = self.codec_ctx.as_mut() {
            ctx.flush();
        }

        Ok(())
    }

    fn read_packet(&mut self) -> Result<Option<ffmpeg::Packet>> {
        let mut packet = ffmpeg::Packet::empty();
        match self.ictx.as_mut().unwrap().read_packet(&mut packet) {
            Ok(()) => Ok(Some(packet)),
            Err(ffmpeg::Error::Eof) => Ok(None),
            Err(e) => Err(MediaError::Decoder(e.to_string()).into()),
        }
    }

    fn convert_frame(&self, frame: ffmpeg::frame::Video, frame_num: Frame) -> Result<VideoFrame> {
        let mut data = VideoFrameData::new(self.format, self.width, self.height);

        // Convert based on format
        // This is simplified - real implementation would use swscale
        copy_frame_data(&frame, &mut data, self.format)?;

        Ok(VideoFrame {
            clip_id: self.clip_id,
            frame: frame_num,
            pts: frame.pts().unwrap_or(0),
            dts: frame.dts().unwrap_or(0),
            width: self.width,
            height: self.height,
            format: self.format,
            color_space: self.color_space,
            data: Arc::new(data),
            keyframe: frame.is_key(),
            duration: Frame(1),
        })
    }

    fn frame_to_pts(&self, frame: Frame) -> i64 {
        (frame.0 as i64 * self.time_base.den * self.frame_rate.num)
            / (self.time_base.num * self.frame_rate.den)
    }

    fn pts_to_frame(&self, pts: i64) -> Frame {
        Frame(
            (pts as i64 * self.time_base.num * self.frame_rate.den)
                / (self.time_base.den * self.frame_rate.num),
        )
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

/// Audio decoder
pub struct AudioDecoder {
    clip_id: ClipId,
    ictx: Option<ffmpeg::format::context::Input>,
    audio_stream_index: usize,
    codec_ctx: Option<ffmpeg::codec::context::Context>,
    sample_rate: u32,
    channels: u16,
    channel_layout: ffmpeg::channel_layout::ChannelLayout,
    format: AudioFormat,
    time_base: Rational,
    duration: Frame,
    resampler: Option<ffmpeg::software::resampling::Context>,
}

impl AudioDecoder {
    pub fn new(clip_id: ClipId, path: &Path, target_format: AudioFormat) -> Result<Self> {
        ffmpeg::init()?;

        let mut ictx =
            ffmpeg::format::input(&path).map_err(|e| MediaError::Decoder(e.to_string()))?;

        let audio_stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Audio)
            .ok_or(MediaError::NoAudioStream)?;

        let stream_index = audio_stream.index();
        let codec_ctx = ffmpeg::codec::context::Context::from_parameters(audio_stream.parameters())
            .map_err(|e| MediaError::Decoder(e.to_string()))?;

        let mut decoder = codec_ctx
            .decoder()
            .audio()
            .map_err(|e| MediaError::Decoder(e.to_string()))?;

        let time_base = audio_stream.time_base();
        let duration = audio_stream.duration().unwrap_or(0);
        let sample_rate = decoder.rate();
        let channels = decoder.channels() as u16;
        let channel_layout = decoder.channel_layout();

        // Setup resampler if needed
        let target_sample_fmt = match target_format {
            AudioFormat::F32 => ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed),
            _ => ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed),
        };

        let resampler = if decoder.format() != target_sample_fmt
            || decoder.rate() != 48000
            || decoder.channel_layout() != ffmpeg::channel_layout::ChannelLayout::STEREO
        {
            Some(
                ffmpeg::software::resampling::Context::get(
                    decoder.format(),
                    decoder.channel_layout(),
                    decoder.rate(),
                    target_sample_fmt,
                    ffmpeg::channel_layout::ChannelLayout::STEREO,
                    48000,
                )
                .ok(),
            )
        } else {
            None
        };

        Ok(Self {
            clip_id,
            ictx: Some(ictx),
            audio_stream_index: stream_index,
            codec_ctx: Some(decoder),
            sample_rate: 48000,
            channels: 2,
            channel_layout: ffmpeg::channel_layout::ChannelLayout::STEREO,
            format: target_format,
            time_base: Rational::new(time_base.0 as i64, time_base.1 as i64),
            duration: Frame(duration),
            resampler,
        })
    }

    pub fn decode_all(&mut self) -> Result<Vec<AudioFrame>> {
        let mut frames = Vec::new();
        let mut frame = ffmpeg::frame::Audio::empty();

        while let Some(packet) = self.read_packet()? {
            if packet.stream() != self.audio_stream_index {
                continue;
            }

            self.codec_ctx
                .as_mut()
                .unwrap()
                .send_packet(&packet)
                .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;

            while self
                .codec_ctx
                .as_mut()
                .unwrap()
                .receive_frame(&mut frame)
                .is_ok()
            {
                frames.push(self.convert_frame(frame)?);
                frame = ffmpeg::frame::Audio::empty();
            }
        }

        // Flush
        self.codec_ctx
            .as_mut()
            .unwrap()
            .send_packet(ffmpeg::Packet::empty())
            .ok();
        while self
            .codec_ctx
            .as_mut()
            .unwrap()
            .receive_frame(&mut frame)
            .is_ok()
        {
            frames.push(self.convert_frame(frame)?);
        }

        Ok(frames)
    }

    fn read_packet(&mut self) -> Result<Option<ffmpeg::Packet>> {
        let mut packet = ffmpeg::Packet::empty();
        match self.ictx.as_mut().unwrap().read_packet(&mut packet) {
            Ok(()) => Ok(Some(packet)),
            Err(ffmpeg::Error::Eof) => Ok(None),
            Err(e) => Err(MediaError::Decoder(e.to_string()).into()),
        }
    }

    fn convert_frame(&self, frame: ffmpeg::frame::Audio) -> Result<AudioFrame> {
        let mut data = AudioFrameData::new(self.channels, frame.samples() as usize);

        // Convert to f32
        let converted = if let Some(ref resampler) = self.resampler {
            let mut out = ffmpeg::frame::Audio::empty();
            resampler
                .run(&frame, &mut out)
                .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;
            out
        } else {
            frame
        };

        // Copy channel data
        for (i, channel) in converted.planes().enumerate() {
            if i >= data.channels.len() {
                break;
            }
            let src: &[f32] = unsafe {
                std::slice::from_raw_parts(channel.as_ptr() as *const f32, converted.samples())
            };
            data.channels[i].copy_from_slice(src);
        }

        Ok(AudioFrame {
            clip_id: self.clip_id,
            frame: self.pts_to_frame(converted.pts().unwrap_or(0)),
            pts: converted.pts().unwrap_or(0),
            sample_rate: self.sample_rate,
            channels: self.channels,
            format: self.format,
            data: Arc::new(data),
            duration: Frame(converted.samples() as i64),
        })
    }

    fn pts_to_frame(&self, pts: i64) -> Frame {
        Frame((pts as i64 * 48000 * self.time_base.num) / (self.time_base.den * 1))
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

/// Unified decoder for any media type
pub struct MediaDecoder {
    video: Option<VideoDecoder>,
    audio: Option<AudioDecoder>,
}

impl MediaDecoder {
    pub fn new(clip_id: ClipId, path: &Path) -> Result<Self> {
        let video = VideoDecoder::new(clip_id, path).ok();
        let audio = AudioDecoder::new(clip_id, path, AudioFormat::F32).ok();

        Ok(Self { video, audio })
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

fn pixel_format_from_ffmpeg(fmt: ffmpeg::format::Pixel) -> PixelFormat {
    match fmt {
        ffmpeg::format::Pixel::RGB24 => PixelFormat::Rgb8,
        ffmpeg::format::Pixel::RGBA => PixelFormat::Rgba8,
        ffmpeg::format::Pixel::YUV420P => PixelFormat::Yuv420_8,
        ffmpeg::format::Pixel::YUV422P => PixelFormat::Yuv422_8,
        ffmpeg::format::Pixel::YUV444P => PixelFormat::Yuv444_8,
        ffmpeg::format::Pixel::YUV420P10LE => PixelFormat::Yuv420_10,
        ffmpeg::format::Pixel::YUV422P10LE => PixelFormat::Yuv422_10,
        ffmpeg::format::Pixel::YUV444P10LE => PixelFormat::Yuv444_10,
        ffmpeg::format::Pixel::P010LE => PixelFormat::P010,
        ffmpeg::format::Pixel::GBRPF32 => PixelFormat::Rgb32F,
        ffmpeg::format::Pixel::GBRAPF32 => PixelFormat::Rgba32F,
        _ => PixelFormat::Rgba8,
    }
}

fn color_space_from_ffmpeg(
    cs: ffmpeg::format::ColorSpace,
    range: ffmpeg::format::ColorRange,
) -> ColorSpace {
    match (cs, range) {
        (ffmpeg::format::ColorSpace::BT709, ffmpeg::format::ColorRange::MPEG) => ColorSpace::Rec709,
        (ffmpeg::format::ColorSpace::BT2020, _) => ColorSpace::Rec2020,
        (ffmpeg::format::ColorSpace::RGB, _) => ColorSpace::Linear,
        _ => ColorSpace::Rec709,
    }
}

fn copy_frame_data(
    src: &ffmpeg::frame::Video,
    dst: &mut VideoFrameData,
    format: PixelFormat,
) -> Result<()> {
    match format {
        PixelFormat::Rgba8 | PixelFormat::Rgb8 | PixelFormat::Rgba16F | PixelFormat::Rgba32F => {
            let src_data = src.data(0);
            let dst_data = dst.plane_mut(0);
            dst_data.copy_from_slice(src_data);
        }
        PixelFormat::Yuv420_8 | PixelFormat::Yuv422_8 | PixelFormat::Yuv444_8 => {
            for i in 0..3 {
                let src_data = src.data(i);
                let dst_data = dst.plane_mut(i);
                let src_stride = src.stride(i);
                let dst_stride = dst.stride(i);
                let h = if i == 0 {
                    src.height()
                } else {
                    src.height() / 2
                };

                for y in 0..h {
                    let src_off = y * src_stride;
                    let dst_off = y * dst_stride;
                    let w = if i == 0 { src.width() } else { src.width() / 2 };
                    dst_data[dst_off..dst_off + w].copy_from_slice(&src_data[src_off..src_off + w]);
                }
            }
        }
        _ => {
            // Default copy
            let src_data = src.data(0);
            let dst_data = dst.plane_mut(0);
            let min_len = src_data.len().min(dst_data.len());
            dst_data[..min_len].copy_from_slice(&src_data[..min_len]);
        }
    }
    Ok(())
}
