//! Media file scanning and discovery

use crate::core::types::{MediaType, Resolution, FrameRate, PixelFormat, ColorSpace, ClipMetadata, ClipTags, ClipFlags, ColorLabel, Thumbnail};
use crate::core::id::{ClipId, MediaId};
use crate::core::time::Frame;
use crate::core::error::{Result, MediaError};
use crate::media::metadata::MediaMetadata;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use walkdir::WalkDir;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{RecommendedWatcher, Watcher, RecursiveMode, Event, EventKind};

/// Media scanner for discovering and importing media files
pub struct MediaScanner {
    watcher: Option<RecommendedWatcher>,
    ignore: Gitignore,
    supported_extensions: HashSet<String>,
    watch_folders: Vec<PathBuf>,
    callback: Option<Box<dyn Fn(ScanEvent) + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    FileFound { path: PathBuf, metadata: MediaMetadata },
    FileRemoved { path: PathBuf },
    FileChanged { path: PathBuf, metadata: MediaMetadata },
    ScanStarted,
    ScanCompleted { files_found: usize },
    ScanError { path: PathBuf, error: String },
}

impl MediaScanner {
    pub fn new(supported_extensions: Vec<String>, watch_folders: Vec<PathBuf>) -> Result<Self> {
        let mut ignore_builder = GitignoreBuilder::new(".");
        ignore_builder.add_line(None, ".*")?;
        ignore_builder.add_line(None, "Thumbs.db")?;
        ignore_builder.add_line(None, ".DS_Store")?;
        let ignore = ignore_builder.build()?;

        let extensions: HashSet<String> = supported_extensions.into_iter()
            .map(|e| e.to_lowercase())
            .collect();

        Ok(Self {
            watcher: None,
            ignore,
            supported_extensions: extensions,
            watch_folders,
            callback: None,
        })
    }

    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(ScanEvent) + Send + Sync + 'static,
    {
        self.callback = Some(Box::new(callback));
    }

    pub fn scan(&mut self) -> Result<Vec<MediaMetadata>> {
        self.emit(ScanEvent::ScanStarted);
        let mut results = Vec::new();

        for folder in &self.watch_folders.clone() {
            if let Err(e) = self.scan_folder(folder, &mut results) {
                self.emit(ScanEvent::ScanError { path: folder.clone(), error: e.to_string() });
            }
        }

        self.emit(ScanEvent::ScanCompleted { files_found: results.len() });
        Ok(results)
    }

    fn scan_folder(&self, folder: &Path, results: &mut Vec<MediaMetadata>) -> Result<()> {
        for entry in WalkDir::new(folder).follow_links(true).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() { continue; }
            if self.ignore.matched(path, path.is_dir()).is_ignore() { continue; }

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !self.supported_extensions.contains(&ext.to_lowercase()) { continue; }
            } else { continue; }

            match self.extract_metadata(path) {
                Ok(metadata) => {
                    results.push(metadata.clone());
                    self.emit(ScanEvent::FileFound { path: path.to_path_buf(), metadata });
                }
                Err(e) => {
                    self.emit(ScanEvent::ScanError { path: path.to_path_buf(), error: e.to_string() });
                }
            }
        }
        Ok(())
    }

    fn extract_metadata(&self, path: &Path) -> Result<MediaMetadata> {
        MediaMetadata::from_file(path)
    }

    pub fn start_watching(&mut self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())?;

        for folder in &self.watch_folders {
            watcher.watch(folder, RecursiveMode::Recursive)?;
        }

        self.watcher = Some(watcher);

        // Spawn event handler
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                // Handle events
            }
        });

        Ok(())
    }

    pub fn stop_watching(&mut self) {
        self.watcher = None;
    }

    fn emit(&self, event: ScanEvent) {
        if let Some(cb) = &self.callback {
            cb(event);
        }
    }

    pub fn is_supported(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| self.supported_extensions.contains(&ext.to_lowercase()))
            .unwrap_or(false)
    }
}

/// Media type detection from file extension
pub fn detect_media_type(path: &Path) -> Option<MediaType> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        // Video
        "mp4" | "mov" | "mxf" | "avi" | "mkv" | "webm" | "flv" | "wmv" | "mpeg" | "mpg" | "m2ts" | "mts" | "ts" | "vob" |
        "prores" | "dnxhd" | "dnxhr" | "r3d" | "braw" | "ari" | "mxf" | "gxf" => Some(MediaType::Video),
        // Audio
        "wav" | "aiff" | "aif" | "flac" | "mp3" | "aac" | "ogg" | "opus" | "wma" | "m4a" | "caf" => Some(MediaType::Audio),
        // Image
        "jpg" | "jpeg" | "png" | "tiff" | "tif" | "exr" | "dpx" | "cin" | "hdr" | "bmp" | "tga" | "psd" | "svg" | "webp" | "jxl" | "avif" => Some(MediaType::Image),
        // Subtitle
        "srt" | "ass" | "ssa" | "vtt" | "sub" | "idx" | "ttml" | "xml" => Some(MediaType::Subtitle),
        _ => None,
    }
}

/// Quick probe for basic file info without full decode
pub fn quick_probe(path: &Path) -> Result<QuickProbeInfo> {
    use ffmpeg_next as ffmpeg;
    
    ffmpeg::init()?;
    
    let mut ictx = ffmpeg::format::input(&path)
        .map_err(|e| MediaError::Decoder(e.to_string()))?;
    
    let mut info = QuickProbeInfo {
        path: path.to_path_buf(),
        duration: None,
        video_streams: Vec::new(),
        audio_streams: Vec::new(),
        subtitle_streams: Vec::new(),
        format_name: ictx.format().name().to_string(),
        bit_rate: ictx.bit_rate(),
        size: std::fs::metadata(path).map(|m| m.len()).ok(),
    };

    for stream in ictx.streams() {
        let codec = stream.codec();
        let codec_name = codec.name().to_string();
        let codec_type = codec.medium();

        match codec_type {
            ffmpeg::media::Type::Video => {
                let width = codec.width();
                let height = codec.height();
                let frame_rate = stream.avg_frame_rate();
                let duration = stream.duration();
                let time_base = stream.time_base();
                let pix_fmt = format!("{:?}", codec.format());
                let color_space = codec.color_space();
                let color_range = codec.color_range();

                info.video_streams.push(VideoStreamInfo {
                    index: stream.index(),
                    codec_name,
                    width,
                    height,
                    frame_rate: if frame_rate.0 > 0 && frame_rate.1 > 0 {
                        Some(frame_rate.0 as f32 / frame_rate.1 as f32)
                    } else { None },
                    duration: duration.map(|d| Frame::from_seconds(
                        d as f64 * time_base.0 as f64 / time_base.1 as f64,
                        FrameRate::Fps24
                    )),
                    pixel_format: pix_fmt,
                    color_space: format!("{:?}", color_space),
                    color_range: format!("{:?}", color_range),
                    bit_rate: codec.bit_rate(),
                });
            }
            ffmpeg::media::Type::Audio => {
                let sample_rate = codec.rate();
                let channels = codec.channels();
                let channel_layout = format!("{:?}", codec.channel_layout());
                let sample_fmt = format!("{:?}", codec.format());
                let duration = stream.duration();
                let time_base = stream.time_base();

                info.audio_streams.push(AudioStreamInfo {
                    index: stream.index(),
                    codec_name,
                    sample_rate,
                    channels,
                    channel_layout,
                    sample_format: sample_fmt,
                    duration: duration.map(|d| Frame::from_seconds(
                        d as f64 * time_base.0 as f64 / time_base.1 as f64,
                        FrameRate::Fps24
                    )),
                    bit_rate: codec.bit_rate(),
                });
            }
            ffmpeg::media::Type::Subtitle => {
                info.subtitle_streams.push(SubtitleStreamInfo {
                    index: stream.index(),
                    codec_name,
                    language: stream.language().unwrap_or_default(),
                });
            }
            _ => {}
        }
    }

    // Overall duration
    if let Some(dur) = ictx.duration() {
        info.duration = Some(Frame::from_seconds(
            dur as f64 / ffi::AV_TIME_BASE as f64,
            FrameRate::Fps24
        ));
    }

    Ok(info)
}

#[derive(Debug, Clone)]
pub struct QuickProbeInfo {
    pub path: PathBuf,
    pub duration: Option<Frame>,
    pub video_streams: Vec<VideoStreamInfo>,
    pub audio_streams: Vec<AudioStreamInfo>,
    pub subtitle_streams: Vec<SubtitleStreamInfo>,
    pub format_name: String,
    pub bit_rate: i64,
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct VideoStreamInfo {
    pub index: usize,
    pub codec_name: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: Option<f32>,
    pub duration: Option<Frame>,
    pub pixel_format: String,
    pub color_space: String,
    pub color_range: String,
    pub bit_rate: i64,
}

#[derive(Debug, Clone)]
pub struct AudioStreamInfo {
    pub index: usize,
    pub codec_name: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub channel_layout: String,
    pub sample_format: String,
    pub duration: Option<Frame>,
    pub bit_rate: i64,
}

#[derive(Debug, Clone)]
pub struct SubtitleStreamInfo {
    pub index: usize,
    pub codec_name: String,
    pub language: String,
}

/// Batch import multiple files
pub struct BatchImporter {
    scanner: MediaScanner,
    pool: crate::media::pool::MediaPool,
}

impl BatchImporter {
    pub fn new(scanner: MediaScanner, pool: crate::media::pool::MediaPool) -> Self {
        Self { scanner, pool }
    }

    pub async fn import_files(&mut self, paths: Vec<PathBuf>) -> ImportResult {
        let mut result = ImportResult::default();
        
        for path in paths {
            match self.import_single(&path).await {
                Ok(clip_id) => {
                    result.imported.push(clip_id);
                    result.success_count += 1;
                }
                Err(e) => {
                    result.failed.push(ImportFailure { path: path.clone(), error: e.to_string() });
                    result.failure_count += 1;
                }
            }
        }
        
        result
    }

    async fn import_single(&mut self, path: &Path) -> Result<ClipId> {
        let metadata = self.scanner.extract_metadata(path)?;
        let clip_id = self.pool.add_clip(metadata).await?;
        Ok(clip_id)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImportResult {
    pub imported: Vec<ClipId>,
    pub failed: Vec<ImportFailure>,
    pub success_count: usize,
    pub failure_count: usize,
}

#[derive(Debug, Clone)]
pub struct ImportFailure {
    pub path: PathBuf,
    pub error: String,
}

mod ffi {
    pub const AV_TIME_BASE: i64 = 1000000;
}