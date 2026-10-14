//! Media metadata extraction and management

use crate::core::types::{MediaType, Resolution, FrameRate, PixelFormat, ColorSpace, ClipMetadata, ClipTags, ClipFlags, ColorLabel, Thumbnail};
use crate::core::id::{ClipId, MediaId};
use crate::core::time::Frame;
use crate::core::error::{Result, MediaError};
use std::path::Path;
use std::collections::HashMap;
use ffmpeg_next as ffmpeg;

/// Comprehensive media metadata
#[derive(Debug, Clone)]
pub struct MediaMetadata {
    pub id: MediaId,
    pub clip_id: ClipId,
    pub file_path: PathBuf,
    pub file_name: String,
    pub file_size: u64,
    pub media_type: MediaType,
    pub duration: Frame,
    pub bit_rate: u64,
    pub format_name: String,
    pub format_long_name: String,
    pub video_streams: Vec<VideoStreamMetadata>,
    pub audio_streams: Vec<AudioStreamMetadata>,
    pub subtitle_streams: Vec<SubtitleStreamMetadata>,
    pub chapters: Vec<ChapterMetadata>,
    pub tags: HashMap<String, String>,
    pub creation_time: Option<chrono::DateTime<chrono::Utc>>,
    pub modification_time: chrono::DateTime<chrono::Utc>,
}

impl MediaMetadata {
    pub fn from_file(path: &Path) -> Result<Self> {
        ffmpeg::init()?;
        
        let mut ictx = ffmpeg::format::input(&path)
            .map_err(|e| MediaError::Decoder(e.to_string()))?;
        
        let metadata = Self::from_format_context(&mut ictx, path)?;
        Ok(metadata)
    }

    fn from_format_context(ictx: &mut ffmpeg::format::context::Input, path: &Path) -> Result<Self> {
        let file_meta = std::fs::metadata(path)?;
        let file_size = file_meta.len();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        
        let format_name = ictx.format().name().to_string();
        let format_long_name = ictx.format().long_name().to_string();
        
        let duration = ictx.duration().unwrap_or(0);
        let bit_rate = ictx.bit_rate() as u64;
        
        let creation_time = file_meta.created().ok()
            .and_then(|t| chrono::DateTime::<chrono::Utc>::from(t).into());
        let modification_time = file_meta.modified().ok()
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
            .unwrap_or_else(chrono::Utc::now);
        
        // Extract tags
        let mut tags = HashMap::new();
        for (k, v) in ictx.metadata().iter() {
            tags.insert(k.to_string(), v.to_string());
        }
        
        // Process streams
        let mut video_streams = Vec::new();
        let mut audio_streams = Vec::new();
        let mut subtitle_streams = Vec::new();
        
        for stream in ictx.streams() {
            let codec = stream.codec();
            let stream_tags: HashMap<String, String> = stream.metadata()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            
            match codec.medium() {
                ffmpeg::media::Type::Video => {
                    video_streams.push(VideoStreamMetadata::from_stream(stream)?);
                }
                ffmpeg::media::Type::Audio => {
                    audio_streams.push(AudioStreamMetadata::from_stream(stream)?);
                }
                ffmpeg::media::Type::Subtitle => {
                    subtitle_streams.push(SubtitleStreamMetadata::from_stream(stream)?);
                }
                _ => {}
            }
        }
        
        // Chapters
        let chapters = ictx.chapters()
            .iter()
            .map(ChapterMetadata::from_chapter)
            .collect();
        
        // Determine primary media type
        let media_type = if !video_streams.is_empty() {
            MediaType::Video
        } else if !audio_streams.is_empty() {
            MediaType::Audio
        } else {
            MediaType::Image
        };
        
        Ok(Self {
            id: MediaId::generate(),
            clip_id: ClipId::nil(),
            file_path: path.to_path_buf(),
            file_name,
            file_size,
            media_type,
            duration: Frame(duration),
            bit_rate,
            format_name,
            format_long_name,
            video_streams,
            audio_streams,
            subtitle_streams,
            chapters,
            tags,
            creation_time,
            modification_time,
        })
    }

    pub fn primary_video_stream(&self) -> Option<&VideoStreamMetadata> {
        self.video_streams.first()
    }

    pub fn primary_audio_stream(&self) -> Option<&AudioStreamMetadata> {
        self.audio_streams.first()
    }

    pub fn to_clip_metadata(&self, clip_id: ClipId) -> ClipMetadata {
        let video = self.primary_video_stream();
        let audio = self.primary_audio_stream();
        
        ClipMetadata {
            id: clip_id,
            file_path: self.file_path.to_string_lossy().to_string(),
            media_type: self.media_type,
            duration: self.duration,
            resolution: video.and_then(|v| v.resolution),
            frame_rate: video.and_then(|v| v.frame_rate),
            pixel_format: video.and_then(|v| v.pixel_format),
            color_space: video.and_then(|v| v.color_space),
            audio_channels: audio.map(|a| a.channels as u16),
            audio_sample_rate: audio.map(|a| a.sample_rate),
            codec_name: video.map(|v| v.codec_name.clone()).or_else(|| audio.map(|a| a.codec_name.clone())),
            bit_rate: Some(self.bit_rate),
            tags: ClipTags {
                name: self.file_name.clone(),
                ..Default::default()
            },
            thumbnails: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoStreamMetadata {
    pub index: usize,
    pub codec_name: String,
    pub codec_long_name: String,
    pub profile: String,
    pub level: i32,
    pub width: u32,
    pub height: u32,
    pub resolution: Option<Resolution>,
    pub frame_rate: Option<FrameRate>,
    pub time_base: Rational,
    pub duration: Option<Frame>,
    pub bit_rate: u64,
    pub pixel_format: Option<PixelFormat>,
    pub color_space: Option<ColorSpace>,
    pub color_range: String,
    pub color_primaries: String,
    pub color_transfer: String,
    pub field_order: String,
    pub refs: i32,
    pub is_avc: bool,
    pub is_hevc: bool,
    pub is_pro_res: bool,
    pub is_dnxhd: bool,
    pub tags: HashMap<String, String>,
}

impl VideoStreamMetadata {
    fn from_stream(stream: &ffmpeg::format::Stream) -> Result<Self> {
        let codec = stream.codec();
        let width = codec.width();
        let height = codec.height();
        
        let resolution = match (width, height) {
            (720, 486) => Some(Resolution::Ntsc),
            (720, 576) => Some(Resolution::Pal),
            (1280, 720) => Some(Resolution::Hd720),
            (1920, 1080) => Some(Resolution::Hd1080),
            (2048, 1080) => Some(Resolution::Dci2k),
            (3840, 2160) => Some(Resolution::Uhd4k),
            (4096, 2160) => Some(Resolution::Dci4k),
            (5120, 2880) => Some(Resolution::Uhd5k),
            (6144, 3456) => Some(Resolution::Uhd6k),
            (7680, 4320) => Some(Resolution::Uhd8k),
            (8192, 4320) => Some(Resolution::Dci8k),
            _ => Some(Resolution::Custom(width, height)),
        };
        
        let frame_rate = stream.avg_frame_rate();
        let frame_rate = if frame_rate.0 > 0 && frame_rate.1 > 0 {
            Some(FrameRate::Custom(crate::core::time::Rational::new(frame_rate.0 as i64, frame_rate.1 as i64)))
        } else { None };
        
        let time_base = stream.time_base();
        let duration = stream.duration().map(|d| Frame(d));
        
        let pixel_format = pixel_format_from_ffmpeg(codec.format());
        let color_space = color_space_from_ffmpeg(codec.color_space(), codec.color_range());
        
        let codec_name = codec.name().to_string();
        let is_pro_res = codec_name.contains("prores");
        let is_dnxhd = codec_name.contains("dnxhd") || codec_name.contains("dnxhr");
        
        Ok(Self {
            index: stream.index(),
            codec_name: codec_name.clone(),
            codec_long_name: codec.description().to_string(),
            profile: format!("{:?}", codec.profile()),
            level: codec.level(),
            width,
            height,
            resolution,
            frame_rate,
            time_base: Rational::new(time_base.0 as i64, time_base.1 as i64),
            duration,
            bit_rate: codec.bit_rate() as u64,
            pixel_format: Some(pixel_format),
            color_space: Some(color_space),
            color_range: format!("{:?}", codec.color_range()),
            color_primaries: format!("{:?}", codec.color_primaries()),
            color_transfer: format!("{:?}", codec.color_trc()),
            field_order: format!("{:?}", codec.field_order()),
            refs: codec.refs(),
            is_avc: codec_name == "h264",
            is_hevc: codec_name == "hevc",
            is_pro_res,
            is_dnxhd,
            tags: stream.metadata().iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AudioStreamMetadata {
    pub index: usize,
    pub codec_name: String,
    pub codec_long_name: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub channel_layout: String,
    pub sample_format: String,
    pub bit_rate: u64,
    pub time_base: Rational,
    pub duration: Option<Frame>,
    pub language: String,
    pub tags: HashMap<String, String>,
}

impl AudioStreamMetadata {
    fn from_stream(stream: &ffmpeg::format::Stream) -> Result<Self> {
        let codec = stream.codec();
        Ok(Self {
            index: stream.index(),
            codec_name: codec.name().to_string(),
            codec_long_name: codec.description().to_string(),
            sample_rate: codec.rate(),
            channels: codec.channels(),
            channel_layout: format!("{:?}", codec.channel_layout()),
            sample_format: format!("{:?}", codec.format()),
            bit_rate: codec.bit_rate() as u64,
            time_base: Rational::new(stream.time_base().0 as i64, stream.time_base().1 as i64),
            duration: stream.duration().map(|d| Frame(d)),
            language: stream.language().unwrap_or_default().to_string(),
            tags: stream.metadata().iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SubtitleStreamMetadata {
    pub index: usize,
    pub codec_name: String,
    pub language: String,
    pub is_hearing_impaired: bool,
    pub tags: HashMap<String, String>,
}

impl SubtitleStreamMetadata {
    fn from_stream(stream: &ffmpeg::format::Stream) -> Result<Self> {
        let codec = stream.codec();
        Ok(Self {
            index: stream.index(),
            codec_name: codec.name().to_string(),
            language: stream.language().unwrap_or_default().to_string(),
            is_hearing_impaired: stream.disposition().contains(ffmpeg::format::stream::Disposition::HEARING_IMPAIRED),
            tags: stream.metadata().iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ChapterMetadata {
    pub id: u64,
    pub start: Frame,
    pub end: Frame,
    pub title: String,
    pub tags: HashMap<String, String>,
}

impl ChapterMetadata {
    fn from_chapter(chapter: &ffmpeg::format::Chapter) -> Self {
        Self {
            id: chapter.id(),
            start: Frame(chapter.start()),
            end: Frame(chapter.end()),
            title: chapter.metadata().get("title").unwrap_or(&"").to_string(),
            tags: chapter.metadata().iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    pub num: i64,
    pub den: i64,
}

impl Rational {
    pub const fn new(num: i64, den: i64) -> Self { Self { num, den } }
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

fn color_space_from_ffmpeg(cs: ffmpeg::format::ColorSpace, range: ffmpeg::format::ColorRange) -> ColorSpace {
    match (cs, range) {
        (ffmpeg::format::ColorSpace::BT709, ffmpeg::format::ColorRange::MPEG) => ColorSpace::Rec709,
        (ffmpeg::format::ColorSpace::BT2020, _) => ColorSpace::Rec2020,
        (ffmpeg::format::ColorSpace::RGB, _) => ColorSpace::Linear,
        _ => ColorSpace::Rec709,
    }
}

use std::path::PathBuf;