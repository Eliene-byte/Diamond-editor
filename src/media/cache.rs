//! Media cache - frame, proxy, thumbnail, waveform caching

use crate::core::error::{AppError, CacheError, Result};
use crate::core::id::{CacheEntryId, ClipId};
use crate::core::time::Frame;
use crate::core::types::FrameRate;
use crate::media::decoder::VideoFrame;
use crate::media::proxy::ProxyMetadata;
use crate::media::thumbnail::Thumbnail;
use crate::media::waveform::Waveform;
use lru::LruCache;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Unified media cache with multiple tiers
pub struct MediaCache {
    // In-memory caches (LruCache)
    frame_cache: RwLock<LruCache<FrameCacheKey, Arc<VideoFrame>>>,
    thumbnail_cache: RwLock<LruCache<ThumbnailCacheKey, Arc<Thumbnail>>>,
    waveform_cache: RwLock<LruCache<ClipId, Arc<Waveform>>>,

    // Disk cache index
    disk_index: RwLock<DiskCacheIndex>,

    // Configuration
    config: CacheConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FrameCacheKey {
    clip_id: ClipId,
    frame: Frame,
    quality: CacheQuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ThumbnailCacheKey {
    clip_id: ClipId,
    frame: Frame,
    size: (u16, u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheQuality {
    Full,
    Proxy,
    Thumbnail,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_memory_frames: usize,
    pub max_memory_thumbnails: usize,
    pub max_memory_waveforms: usize,
    pub disk_cache_dir: PathBuf,
    pub max_disk_size_gb: f32,
    pub enable_disk_cache: bool,
    pub compression: CacheCompression,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_frames: 200,
            max_memory_thumbnails: 500,
            max_memory_waveforms: 100,
            disk_cache_dir: PathBuf::from("cache"),
            max_disk_size_gb: 20.0,
            enable_disk_cache: true,
            compression: CacheCompression::Lz4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheCompression {
    None,
    Lz4,
    Zstd,
}

#[derive(Debug, Default)]
struct DiskCacheIndex {
    entries: HashMap<CacheEntryId, DiskCacheEntry>,
    total_size: u64,
}

#[derive(Debug, Clone)]
struct DiskCacheEntry {
    id: CacheEntryId,
    key: DiskCacheKey,
    path: PathBuf,
    size: u64,
    created: chrono::DateTime<chrono::Utc>,
    last_access: chrono::DateTime<chrono::Utc>,
    access_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DiskCacheKey {
    Frame {
        clip_id: ClipId,
        frame: Frame,
        quality: CacheQuality,
    },
    Thumbnail {
        clip_id: ClipId,
        frame: Frame,
        size: (u16, u16),
    },
    Waveform {
        clip_id: ClipId,
    },
    Proxy {
        clip_id: ClipId,
    },
}

impl MediaCache {
    pub fn new(config: CacheConfig) -> Result<Self> {
        if config.enable_disk_cache {
            std::fs::create_dir_all(&config.disk_cache_dir)?;
        }

        Ok(Self {
            frame_cache: RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(config.max_memory_frames).unwrap(),
            )),
            thumbnail_cache: RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(config.max_memory_thumbnails).unwrap(),
            )),
            waveform_cache: RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(config.max_memory_waveforms).unwrap(),
            )),
            disk_index: RwLock::new(DiskCacheIndex::default()),
            config,
        })
    }

    // Frame cache
    pub fn get_frame(
        &self,
        clip_id: ClipId,
        frame: Frame,
        quality: CacheQuality,
    ) -> Option<Arc<VideoFrame>> {
        let key = FrameCacheKey {
            clip_id,
            frame,
            quality,
        };
        self.frame_cache.write().get(&key).cloned()
    }

    pub fn put_frame(
        &self,
        clip_id: ClipId,
        frame: Frame,
        quality: CacheQuality,
        video_frame: VideoFrame,
    ) {
        let key = FrameCacheKey {
            clip_id,
            frame,
            quality,
        };
        self.frame_cache.write().put(key, Arc::new(video_frame));
    }

    pub fn evict_frame(&self, clip_id: ClipId, frame: Frame) {
        for quality in [
            CacheQuality::Full,
            CacheQuality::Proxy,
            CacheQuality::Thumbnail,
        ] {
            let key = FrameCacheKey {
                clip_id,
                frame,
                quality,
            };
            self.frame_cache.write().pop(&key);
        }
    }

    pub fn evict_clip_frames(&self, clip_id: ClipId) {
        let mut cache = self.frame_cache.write();
        let keys: Vec<_> = cache
            .iter()
            .filter(|(k, _)| k.clip_id == clip_id)
            .map(|(k, _)| *k)
            .collect();
        for key in keys {
            cache.pop(&key);
        }
    }

    // Thumbnail cache
    pub fn get_thumbnail(
        &self,
        clip_id: ClipId,
        frame: Frame,
        size: (u16, u16),
    ) -> Option<Arc<Thumbnail>> {
        let key = ThumbnailCacheKey {
            clip_id,
            frame,
            size,
        };
        self.thumbnail_cache.write().get(&key).cloned()
    }

    pub fn put_thumbnail(&self, clip_id: ClipId, frame: Frame, size: (u16, u16), thumb: Thumbnail) {
        let key = ThumbnailCacheKey {
            clip_id,
            frame,
            size,
        };
        self.thumbnail_cache.write().put(key, Arc::new(thumb));
    }

    // Waveform cache
    pub fn get_waveform(&self, clip_id: ClipId) -> Option<Arc<Waveform>> {
        self.waveform_cache.write().get(&clip_id).cloned()
    }

    pub fn put_waveform(&self, clip_id: ClipId, waveform: Waveform) {
        self.waveform_cache.write().put(clip_id, Arc::new(waveform));
    }

    // Disk cache
    pub fn cache_to_disk(&self, key: DiskCacheKey, data: &[u8]) -> Result<CacheEntryId> {
        if !self.config.enable_disk_cache {
            return Err(AppError::Cache(CacheError::Disabled));
        }

        let entry_id = CacheEntryId::generate();
        let filename = format!("{}.cache", entry_id.0);
        let path = self.config.disk_cache_dir.join(&filename);

        // Compress if needed
        let compressed = match self.config.compression {
            CacheCompression::None => data.to_vec(),
            CacheCompression::Lz4 => lz4_flex::compress_prepend_size(data),
            CacheCompression::Zstd => zstd::encode_all(data, 3)
                .map_err(|e| AppError::Cache(CacheError::Compress(e.to_string())))?,
        };

        std::fs::write(&path, &compressed)?;

        let entry = DiskCacheEntry {
            id: entry_id,
            key: key.clone(),
            path,
            size: compressed.len() as u64,
            created: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            access_count: 0,
        };

        let mut index = self.disk_index.write();
        index.entries.insert(entry_id, entry);
        index.total_size += compressed.len() as u64;

        self.enforce_disk_limit(&mut index)?;

        Ok(entry_id)
    }

    pub fn load_from_disk(&self, entry_id: CacheEntryId) -> Result<Vec<u8>> {
        let mut index = self.disk_index.write();
        let entry =
            index
                .entries
                .get_mut(&entry_id)
                .ok_or(AppError::Cache(CacheError::NotFound(format!(
                    "{}.cache",
                    entry_id.0
                ))))?;

        let data = std::fs::read(&entry.path)?;

        entry.last_access = chrono::Utc::now();
        entry.access_count += 1;

        // Decompress
        let decompressed = match self.config.compression {
            CacheCompression::None => data,
            CacheCompression::Lz4 => lz4_flex::decompress_size_prepended(&data)
                .map_err(|e| AppError::Cache(CacheError::Decompress(e.to_string())))?,
            CacheCompression::Zstd => zstd::decode_all(&data[..])
                .map_err(|e| AppError::Cache(CacheError::Decompress(e.to_string())))?,
        };

        Ok(decompressed)
    }

    pub fn remove_from_disk(&self, entry_id: CacheEntryId) -> Result<()> {
        let mut index = self.disk_index.write();
        if let Some(entry) = index.entries.remove(&entry_id) {
            index.total_size = index.total_size.saturating_sub(entry.size);
            std::fs::remove_file(entry.path).ok();
        }
        Ok(())
    }

    fn enforce_disk_limit(&self, index: &mut DiskCacheIndex) -> Result<()> {
        let max_bytes = (self.config.max_disk_size_gb * 1024.0 * 1024.0 * 1024.0) as u64;

        while index.total_size > max_bytes {
            // Remove least recently used
            if let Some((_, entry)) = index
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_access)
                .map(|(k, v)| (*k, v.clone()))
            {
                index.total_size = index.total_size.saturating_sub(entry.size);
                index.entries.remove(&entry.id);
                std::fs::remove_file(entry.path).ok();
            } else {
                break;
            }
        }
        Ok(())
    }

    pub fn clear_memory(&self) {
        self.frame_cache.write().clear();
        self.thumbnail_cache.write().clear();
        self.waveform_cache.write().clear();
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            memory_frames: self.frame_cache.read().len(),
            memory_thumbnails: self.thumbnail_cache.read().len(),
            memory_waveforms: self.waveform_cache.read().len(),
            disk_entries: self.disk_index.read().entries.len(),
            disk_size_bytes: self.disk_index.read().total_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub memory_frames: usize,
    pub memory_thumbnails: usize,
    pub memory_waveforms: usize,
    pub disk_entries: usize,
    pub disk_size_bytes: u64,
}

/// Frame cache with predictive loading
pub struct PredictiveFrameCache {
    cache: MediaCache,
    prefetch_range: i64,
    current_playhead: Frame,
    playback_direction: PlaybackDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackDirection {
    Forward,
    Backward,
    Stopped,
}

impl PredictiveFrameCache {
    pub fn new(cache: MediaCache, prefetch_seconds: f32, fps: FrameRate) -> Self {
        let r = fps.to_rational();
        let prefetch_range = (prefetch_seconds * r.num as f32 / r.den as f32) as i64;
        Self {
            cache,
            prefetch_range,
            current_playhead: Frame::ZERO,
            playback_direction: PlaybackDirection::Stopped,
        }
    }

    pub fn set_playhead(&mut self, frame: Frame, direction: PlaybackDirection) {
        self.current_playhead = frame;
        self.playback_direction = direction;
        self.prefetch();
    }

    fn prefetch(&self) {
        match self.playback_direction {
            PlaybackDirection::Forward => {
                for i in 1..=self.prefetch_range {
                    let frame = Frame(self.current_playhead.0 + i);
                    // Trigger async load - would need clip_id
                }
            }
            PlaybackDirection::Backward => {
                for i in 1..=self.prefetch_range {
                    let frame = Frame(self.current_playhead.0 - i);
                    // Trigger async load
                }
            }
            PlaybackDirection::Stopped => {}
        }
    }
}
