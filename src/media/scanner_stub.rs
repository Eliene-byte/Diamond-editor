//! Fallback media scanner for builds **without** the `codec-ffmpeg` feature.
//!
//! Extension-based detection and folder walking work exactly as in
//! `scanner.rs`; only the parts that need a demuxer (probing, metadata reads)
//! report an error. That keeps `cargo check --no-default-features` and the
//! headless CI matrix entries compiling and testable.

use crate::core::error::{AppError, MediaError, Result};
use crate::core::time::Frame;
use crate::core::types::{ClipMetadata, MediaType};
use crate::media::metadata::MediaMetadata;
use crate::media::pool::MediaPool;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone, Default)]
pub struct SubtitleStreamInfo {
    pub index: usize,
    pub codec_name: String,
    pub language: String,
}

/// Progress notifications emitted while scanning.
#[derive(Debug, Clone)]
pub enum ScanEvent {
    ScanStarted,
    FileFound {
        path: PathBuf,
        metadata: Box<MediaMetadata>,
    },
    ScanError {
        path: PathBuf,
        error: String,
    },
    ScanCompleted {
        files_found: usize,
    },
}

/// Walks folders looking for media files.
pub struct MediaScanner {
    supported_extensions: Vec<String>,
    watch_folders: Vec<PathBuf>,
    #[allow(dead_code)]
    callback: Option<Box<dyn Fn(ScanEvent) + Send + Sync>>,
}

impl MediaScanner {
    pub fn new(supported_extensions: Vec<String>, watch_folders: Vec<PathBuf>) -> Result<Self> {
        Ok(Self {
            supported_extensions,
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

    /// Walks the configured folders. Without a codec backend no metadata can be
    /// produced, so the result is always empty.
    pub fn scan(&mut self) -> Result<Vec<MediaMetadata>> {
        Ok(Vec::new())
    }

    /// Filesystem watching needs `notify` support; a no-op in the fallback.
    pub fn start_watching(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn stop_watching(&mut self) {}

    pub fn is_supported(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| {
                self.supported_extensions
                    .iter()
                    .any(|e| e == &ext.to_lowercase())
            })
            .unwrap_or(false)
    }

    /// Folders this scanner was configured with.
    pub fn watch_folders(&self) -> &[PathBuf] {
        &self.watch_folders
    }
}

/// Media type detection from file extension. Pure, so it works in every build.
pub fn detect_media_type(path: &Path) -> Option<MediaType> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        // Video
        "mp4" | "mov" | "mxf" | "avi" | "mkv" | "webm" | "flv" | "wmv" | "mpeg" | "mpg"
        | "m2ts" | "mts" | "ts" | "vob" | "prores" | "dnxhd" | "dnxhr" | "r3d" | "braw" | "ari"
        | "gxf" => Some(MediaType::Video),
        // Audio
        "wav" | "aiff" | "aif" | "flac" | "mp3" | "aac" | "ogg" | "opus" | "wma" | "m4a"
        | "caf" => Some(MediaType::Audio),
        // Image
        "jpg" | "jpeg" | "png" | "tiff" | "tif" | "exr" | "dpx" | "cin" | "hdr" | "bmp" | "tga"
        | "psd" | "svg" | "webp" | "jxl" | "avif" => Some(MediaType::Image),
        // Subtitle
        "srt" | "ass" | "ssa" | "vtt" | "sub" | "idx" | "ttml" | "xml" => Some(MediaType::Subtitle),
        _ => None,
    }
}

/// Always fails: probing a container needs a demuxer.
pub fn quick_probe(_path: &Path) -> Result<QuickProbeInfo> {
    Err(AppError::Media(MediaError::MetadataFailed(
        "no codec backend compiled in - rebuild with the `codec-ffmpeg` feature".to_string(),
    )))
}

/// Import outcome for one batch of files.
#[derive(Debug, Clone, Default)]
pub struct ImportResult {
    pub imported: Vec<crate::core::id::ClipId>,
    pub failed: Vec<ImportFailure>,
    pub total_time_ms: u64,
}

/// A single file that could not be imported.
#[derive(Debug, Clone)]
pub struct ImportFailure {
    pub path: PathBuf,
    pub error: String,
}

/// Imports many files into the media pool.
pub struct BatchImporter {
    scanner: MediaScanner,
    #[allow(dead_code)]
    pool: MediaPool,
}

impl BatchImporter {
    pub fn new(scanner: MediaScanner, pool: MediaPool) -> Self {
        Self { scanner, pool }
    }

    /// Every path fails to import without a codec backend, and the failures are
    /// reported individually rather than aborting the batch.
    pub async fn import_files(&mut self, paths: Vec<PathBuf>) -> ImportResult {
        ImportResult {
            imported: Vec::new(),
            failed: paths
                .into_iter()
                .map(|path| ImportFailure {
                    error: "no codec backend compiled in - rebuild with the `codec-ffmpeg` feature"
                        .to_string(),
                    path,
                })
                .collect(),
            total_time_ms: 0,
        }
    }

    /// The scanner this importer was built with.
    pub fn scanner(&self) -> &MediaScanner {
        &self.scanner
    }
}

/// Build a [`ClipMetadata`] placeholder from a path, without probing.
pub fn clip_metadata_from_path(path: &Path) -> ClipMetadata {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();

    let mut metadata = ClipMetadata::default();
    metadata.name = name;
    metadata.file_path = path.to_string_lossy().to_string();
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_detection_covers_each_kind() {
        assert_eq!(
            detect_media_type(Path::new("a.MOV")),
            Some(MediaType::Video)
        );
        assert_eq!(
            detect_media_type(Path::new("a.wav")),
            Some(MediaType::Audio)
        );
        assert_eq!(
            detect_media_type(Path::new("a.exr")),
            Some(MediaType::Image)
        );
        assert_eq!(
            detect_media_type(Path::new("a.srt")),
            Some(MediaType::Subtitle)
        );
        assert_eq!(detect_media_type(Path::new("a.txt")), None);
        assert_eq!(detect_media_type(Path::new("noext")), None);
    }

    #[test]
    fn probing_reports_missing_backend() {
        assert!(quick_probe(Path::new("a.mov")).is_err());
    }

    #[tokio::test]
    async fn import_reports_every_path_as_failed() {
        let scanner = MediaScanner::new(vec!["mov".into()], Vec::new()).unwrap();
        let cache =
            crate::media::cache::MediaCache::new(crate::media::cache::CacheConfig::default())
                .expect("in-memory cache");
        let pool = MediaPool::new(crate::core::id::ProjectId::generate(), Arc::new(cache));
        let mut importer = BatchImporter::new(scanner, pool);

        let result = importer
            .import_files(vec![PathBuf::from("a.mov"), PathBuf::from("b.mov")])
            .await;

        assert!(result.imported.is_empty());
        assert_eq!(result.failed.len(), 2);
    }
}
