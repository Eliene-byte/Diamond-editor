//! Proxy generation for smooth editing

use crate::core::types::{Resolution, PixelFormat, FrameRate};
use crate::core::id::{ClipId, ProxyId};
use crate::core::error::{Result, MediaError};
use crate::core::time::Frame;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ffmpeg_next as ffmpeg;

/// Proxy quality presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyPreset {
    /// Quarter resolution, low bitrate - for rough cutting
    Offline,
    /// Half resolution, medium bitrate - standard proxy
    Standard,
    /// 3/4 resolution, high bitrate - for color grading
    HighQuality,
    /// Custom settings
    Custom,
}

impl ProxyPreset {
    pub fn resolution_scale(&self) -> f32 {
        match self {
            Self::Offline => 0.25,
            Self::Standard => 0.5,
            Self::HighQuality => 0.75,
            Self::Custom => 1.0,
        }
    }

    pub fn bitrate_mbps(&self) -> u32 {
        match self {
            Self::Offline => 5,
            Self::Standard => 15,
            Self::HighQuality => 50,
            Self::Custom => 100,
        }
    }

    pub fn codec(&self) -> &'static str {
        match self {
            Self::Offline => "libx264",
            Self::Standard => "prores_ks",
            Self::HighQuality => "prores_ks",
            Self::Custom => "prores_ks",
        }
    }

    pub fn profile(&self) -> &'static str {
        match self {
            Self::Offline => "baseline",
            Self::Standard => "proxy",
            Self::HighQuality => "hq",
            Self::Custom => "4444",
        }
    }
}

/// Proxy configuration
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub preset: ProxyPreset,
    pub resolution: Option<Resolution>,
    pub frame_rate: Option<FrameRate>,
    pub codec: String,
    pub bitrate: u32,
    pub pixel_format: PixelFormat,
    pub output_dir: PathBuf,
    pub suffix: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            preset: ProxyPreset::Standard,
            resolution: None,
            frame_rate: None,
            codec: "prores_ks".into(),
            bitrate: 15_000_000,
            pixel_format: PixelFormat::Yuv422_10,
            output_dir: PathBuf::from("proxies"),
            suffix: "_proxy".into(),
        }
    }
}

/// Proxy metadata
#[derive(Debug, Clone)]
pub struct ProxyMetadata {
    pub id: ProxyId,
    pub clip_id: ClipId,
    pub path: PathBuf,
    pub preset: ProxyPreset,
    pub resolution: Resolution,
    pub frame_rate: FrameRate,
    pub codec: String,
    pub bitrate: u32,
    pub duration: Frame,
    pub file_size: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Proxy generator
pub struct ProxyGenerator {
    config: ProxyConfig,
}

impl ProxyGenerator {
    pub fn new(config: ProxyConfig) -> Self {
        Self { config }
    }

    pub fn generate(&self, clip_id: ClipId, source_path: &Path) -> Result<ProxyMetadata> {
        ffmpeg::init()?;
        
        // Probe source
        let mut ictx = ffmpeg::format::input(&source_path)
            .map_err(|e| MediaError::Decoder(e.to_string()))?;
        
        let video_stream = ictx.streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(MediaError::NoVideoStream)?;
        
        let src_width = video_stream.codec().width();
        let src_height = video_stream.codec().height();
        let src_fps = video_stream.avg_frame_rate();
        let src_duration = video_stream.duration().unwrap_or(0);
        
        // Calculate proxy resolution
        let scale = self.config.preset.resolution_scale();
        let proxy_width = (src_width as f32 * scale).round() as u32;
        let proxy_height = (src_height as f32 * scale).round() as u32;
        
        // Ensure even dimensions
        let proxy_width = proxy_width & !1;
        let proxy_height = proxy_height & !1;
        
        // Output path
        let stem = source_path.file_stem().unwrap().to_string_lossy();
        let ext = match self.config.preset {
            ProxyPreset::Offline => "mp4",
            _ => "mov",
        };
        let proxy_name = format!("{}{}.{}", stem, self.config.suffix, ext);
        let proxy_path = self.config.output_dir.join(&proxy_name);
        
        std::fs::create_dir_all(&self.config.output_dir)?;
        
        // Build filter graph for scaling
        let filter = format!(
            "scale={}:{}:flags=lanczos,format={}",
            proxy_width, proxy_height, self.pixel_format_ffmpeg()
        );
        
        // Open output
        let mut octx = ffmpeg::format::output(&proxy_path)
            .map_err(|e| MediaError::Encoder(e.to_string()))?;
        
        // Add video stream
        let mut encoder = ffmpeg::codec::encoder::find(self.config.codec)
            .ok_or_else(|| MediaError::Encoder(format!("Codec not found: {}", self.config.codec)))?
            .video()
            .map_err(|e| MediaError::Encoder(e.to_string()))?;
        
        encoder.set_width(proxy_width);
        encoder.set_height(proxy_height);
        encoder.set_format(self.pixel_format_ffmpeg());
        encoder.set_bit_rate(self.config.bitrate);
        encoder.set_time_base(ffmpeg::Rational::new(1, src_fps.0 as i32 / src_fps.1 as i32));
        encoder.set_frame_rate(ffmpeg::Rational::new(src_fps.0 as i32, src_fps.1 as i32));
        
        if self.config.codec == "prores_ks" {
            encoder.set_profile(self.config.preset.profile());
        }
        
        let mut encoder = encoder.open()
            .map_err(|e| MediaError::Encoder(e.to_string()))?;
        
        octx.add_stream(encoder)
            .map_err(|e| MediaError::Encoder(e.to_string()))?;
        
        octx.write_header()
            .map_err(|e| MediaError::Encoder(e.to_string()))?;
        
        // Decode and encode
        let mut decoder = video_stream.codec().decoder()
            .video()
            .map_err(|e| MediaError::Decoder(e.to_string()))?;
        
        let mut frame = ffmpeg::frame::Video::empty();
        let mut packet = ffmpeg::Packet::empty();
        let mut filter_graph = ffmpeg::filter::Graph::new();
        
        // Setup filter
        let mut filter_ctx = filter_graph
            .add(&filter, "")
            .map_err(|e| MediaError::Encoder(e.to_string()))?;
        
        filter_graph.validate()
            .map_err(|e| MediaError::Encoder(e.to_string()))?;
        
        let mut frame_count = 0;
        while ictx.read_packet(&mut packet).is_ok() {
            if packet.stream() != video_stream.index() { continue; }
            
            decoder.send_packet(&packet).ok();
            while decoder.receive_frame(&mut frame).is_ok() {
                // Filter
                filter_ctx.src().add(&frame).ok();
                let mut filtered = ffmpeg::frame::Video::empty();
                while filter_ctx.sink().frame(&mut filtered).is_ok() {
                    encoder.send_frame(&filtered).ok();
                    let mut out_packet = ffmpeg::Packet::empty();
                    while encoder.receive_packet(&mut out_packet).is_ok() {
                        out_packet.set_stream(0);
                        octx.write_packet(&out_packet).ok();
                        frame_count += 1;
                    }
                }
            }
        }
        
        // Flush
        decoder.send_packet(ffmpeg::Packet::empty()).ok();
        while decoder.receive_frame(&mut frame).is_ok() {
            filter_ctx.src().add(&frame).ok();
            let mut filtered = ffmpeg::frame::Video::empty();
            while filter_ctx.sink().frame(&mut filtered).is_ok() {
                encoder.send_frame(&filtered).ok();
                let mut out_packet = ffmpeg::Packet::empty();
                while encoder.receive_packet(&mut out_packet).is_ok() {
                    out_packet.set_stream(0);
                    octx.write_packet(&out_packet).ok();
                }
            }
        }
        
        encoder.send_frame(ffmpeg::frame::Video::empty()).ok();
        let mut out_packet = ffmpeg::Packet::empty();
        while encoder.receive_packet(&mut out_packet).is_ok() {
            out_packet.set_stream(0);
            octx.write_packet(&out_packet).ok();
        }
        
        octx.write_trailer().ok();
        
        let file_size = std::fs::metadata(&proxy_path).map(|m| m.len()).unwrap_or(0);
        
        Ok(ProxyMetadata {
            id: ProxyId::generate(),
            clip_id,
            path: proxy_path,
            preset: self.config.preset,
            resolution: Resolution::Custom(proxy_width, proxy_height),
            frame_rate: FrameRate::Custom(crate::core::time::Rational::new(src_fps.0 as i64, src_fps.1 as i64)),
            codec: self.config.codec.clone(),
            bitrate: self.config.bitrate,
            duration: Frame(src_duration),
            file_size,
            created_at: chrono::Utc::now(),
        })
    }

    fn pixel_format_ffmpeg(&self) -> ffmpeg::format::Pixel {
        match self.config.pixel_format {
            crate::core::types::PixelFormat::Yuv422_10 => ffmpeg::format::Pixel::YUV422P10LE,
            crate::core::types::PixelFormat::Yuv422_8 => ffmpeg::format::Pixel::YUV422P,
            crate::core::types::PixelFormat::Yuv444_10 => ffmpeg::format::Pixel::YUV444P10LE,
            _ => ffmpeg::format::Pixel::YUV422P10LE,
        }
    }

    pub async fn generate_async(&self, clip_id: ClipId, source_path: PathBuf) -> Result<ProxyMetadata> {
        tokio::task::spawn_blocking(move || {
            Self::new(self.config.clone()).generate(clip_id, &source_path)
        }).await?
    }
}

/// Proxy manager - handles proxy selection and fallback
pub struct ProxyManager {
    proxies: std::collections::HashMap<ClipId, ProxyMetadata>,
    active: bool,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self { proxies: std::collections::HashMap::new(), active: true }
    }

    pub fn add(&mut self, proxy: ProxyMetadata) {
        self.proxies.insert(proxy.clip_id, proxy);
    }

    pub fn get(&self, clip_id: ClipId) -> Option<&ProxyMetadata> {
        if self.active {
            self.proxies.get(&clip_id)
        } else {
            None
        }
    }

    pub fn remove(&mut self, clip_id: ClipId) -> Option<ProxyMetadata> {
        self.proxies.remove(&clip_id)
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn is_active(&self) -> bool { self.active }

    pub fn list(&self) -> Vec<&ProxyMetadata> {
        self.proxies.values().collect()
    }

    pub fn total_size(&self) -> u64 {
        self.proxies.values().map(|p| p.file_size).sum()
    }
}

/// Smart proxy renderer - automatically switches between proxy and original
pub struct SmartProxyRenderer {
    manager: ProxyManager,
    decoder_cache: std::collections::HashMap<ClipId, Arc<crate::media::decoder::VideoDecoder>>,
}

impl SmartProxyRenderer {
    pub fn new(manager: ProxyManager) -> Self {
        Self { manager, decoder_cache: std::collections::HashMap::new() }
    }

    pub fn decode_frame(&mut self, clip_id: ClipId, frame: Frame) -> Result<Option<crate::media::decoder::VideoFrame>> {
        // Try proxy first if active
        if self.manager.is_active() {
            if let Some(proxy) = self.manager.get(clip_id) {
                if let Some(decoder) = self.get_or_create_decoder(&proxy.path) {
                    if let Ok(Some(frame)) = decoder.decode_frame(frame) {
                        return Ok(Some(frame));
                    }
                }
            }
        }
        
        // Fallback to original - would need original path
        Ok(None)
    }

    fn get_or_create_decoder(&mut self, path: &Path) -> Option<Arc<crate::media::decoder::VideoDecoder>> {
        // Would need clip_id to cache properly
        None
    }
}