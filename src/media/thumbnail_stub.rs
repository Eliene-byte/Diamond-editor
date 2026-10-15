//! Fallback thumbnail support for builds **without** the `codec-ffmpeg` feature.
//!
//! Generating a still from a video requires a decoder, so without one the cache
//! is inert. The API mirrors `thumbnail.rs` exactly so the rest of the crate
//! compiles unchanged, and generation reports a clear error.

use crate::core::id::ClipId;
use crate::core::time::Frame;
use crate::core::error::{AppError, MediaError, Result};
use std::path::Path;
use std::sync::Mutex;

pub use crate::core::types::Thumbnail;

/// Pixel encodings a thumbnail can be stored in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailFormat {
    Rgba8,
    Rgb8,
    Jpeg,
    Png,
}

/// A grid of thumbnails packed into a single texture.
#[derive(Debug, Clone, Default)]
pub struct ThumbnailSprite {
    pub clip_id: ClipId,
    pub columns: u32,
    pub rows: u32,
    pub data: Vec<u8>,
}

impl ThumbnailSprite {
    /// Number of cells in the sprite sheet.
    pub fn capacity(&self) -> usize {
        self.columns as usize * self.rows as usize
    }
}

/// Generates single-frame stills from a video.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThumbnailGenerator {
    /// Frames between samples when generating a contact sheet.
    pub sample_interval: Frame,
}

impl ThumbnailGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Always fails: decoding a frame needs a codec backend.
    pub fn generate(&self, _path: &Path, _frame: Frame) -> Result<Thumbnail> {
        Err(AppError::Media(MediaError::ThumbnailFailed(
            "no codec backend compiled in - rebuild with the `codec-ffmpeg` feature".to_string(),
        )))
    }

    /// Always fails, see [`ThumbnailGenerator::generate`].
    pub fn generate_sprite(
        &self,
        _path: &Path,
        _start: Frame,
        _count: usize,
    ) -> Result<ThumbnailSprite> {
        Err(AppError::Media(MediaError::ThumbnailFailed(
            "no codec backend compiled in - rebuild with the `codec-ffmpeg` feature".to_string(),
        )))
    }
}

/// In-memory LRU-ish cache of generated thumbnails.
///
/// The real backend bounds this by byte budget; the stub keeps an equivalent
/// entry count so eviction behaviour is observable in tests.
#[derive(Debug)]
pub struct ThumbnailCache {
    max_entries: usize,
    entries: Mutex<Vec<(ClipId, Frame, Thumbnail)>>,
}

impl ThumbnailCache {
    pub fn new(max_entries: usize) -> Self {
        Self { max_entries, entries: Mutex::new(Vec::new()) }
    }

    pub fn get(&self, clip_id: ClipId, frame: Frame) -> Option<Thumbnail> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries
            .iter()
            .find(|(c, f, _)| *c == clip_id && *f == frame)
            .map(|(_, _, t)| t.clone())
    }

    pub fn put(&self, clip_id: ClipId, frame: Frame, thumb: Thumbnail) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|(c, f, _)| !(*c == clip_id && *f == frame));
        if self.max_entries > 0 && entries.len() >= self.max_entries {
            entries.remove(0);
        }
        entries.push((clip_id, frame, thumb));
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

    fn thumb(frame: i64) -> Thumbnail {
        Thumbnail {
            frame: Frame(frame),
            data: Vec::new(),
            width: 160,
            height: 90,
        }
    }

    #[test]
    fn generation_reports_missing_backend() {
        assert!(ThumbnailGenerator::default().generate(Path::new("a.mov"), Frame(0)).is_err());
    }

    #[test]
    fn cache_round_trips_and_evicts() {
        let cache = ThumbnailCache::new(2);
        cache.put(ClipId(1), Frame(0), thumb(0));
        assert!(cache.get(ClipId(1), Frame(0)).is_some());
        assert!(cache.get(ClipId(2), Frame(0)).is_none());

        cache.put(ClipId(2), Frame(0), thumb(0));
        cache.put(ClipId(3), Frame(0), thumb(0));
        assert_eq!(cache.len(), 2, "cache must not exceed max_entries");
        assert!(cache.get(ClipId(1), Frame(0)).is_none(), "oldest entry evicted");
    }

    #[test]
    fn clear_empties_the_cache() {
        let cache = ThumbnailCache::new(4);
        cache.put(ClipId(1), Frame(0), thumb(0));
        cache.clear();
        assert!(cache.is_empty());
    }
}