//! Media pool - central media management

use crate::core::id::{ClipId, BinId, MediaId, ProjectId};
use crate::core::types::{ClipMetadata, MediaType, ClipTags, ColorLabel, ClipFlags};
use crate::core::time::Frame;
use crate::core::error::{Result, MediaError, ProjectError};
use crate::media::metadata::MediaMetadata;
use crate::media::cache::MediaCache;
use crate::media::proxy::ProxyManager;
use crate::media::thumbnail::ThumbnailCache;
use crate::media::waveform::WaveformCache;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use slotmap::SlotMap;

slotmap::new_key_type! { pub struct ClipKey; }
slotmap::new_key_type! { pub struct BinKey; }

/// Clip in the media pool
#[derive(Debug, Clone)]
pub struct PoolClip {
    pub id: ClipId,
    pub key: ClipKey,
    pub bin: BinId,
    pub metadata: ClipMetadata,
    pub media_metadata: Option<MediaMetadata>,
    pub proxy: Option<crate::media::proxy::ProxyMetadata>,
    pub thumbnails: Vec<crate::media::thumbnail::Thumbnail>,
    pub waveform: Option<crate::media::waveform::Waveform>,
    pub status: ClipStatus,
    pub offline_path: Option<PathBuf>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipStatus {
    Online,
    Offline,
    Relinking,
    ProxyOnly,
    Error(String),
}

/// Bin (folder) in the media pool
#[derive(Debug, Clone)]
pub struct PoolBin {
    pub id: BinId,
    pub key: BinKey,
    pub parent: Option<BinId>,
    pub name: String,
    pub clips: Vec<ClipId>,
    pub sub_bins: Vec<BinId>,
    pub expanded: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Media pool - central repository for all media
pub struct MediaPool {
    clips: RwLock<SlotMap<ClipKey, PoolClip>>,
    bins: RwLock<SlotMap<BinKey, PoolBin>>,
    clip_id_to_key: RwLock<HashMap<ClipId, ClipKey>>,
    bin_id_to_key: RwLock<HashMap<BinId, BinKey>>,
    root_bin: BinId,
    project_id: ProjectId,
    cache: Arc<MediaCache>,
    proxy_manager: Arc<ProxyManager>,
    thumbnail_cache: Arc<ThumbnailCache>,
    waveform_cache: Arc<WaveformCache>,
}

impl MediaPool {
    pub fn new(project_id: ProjectId, cache: Arc<MediaCache>) -> Self {
        let mut bins = SlotMap::with_key();
        let root_key = bins.insert(PoolBin {
            id: BinId::generate(),
            key: BinKey::default(),
            parent: None,
            name: "Master".into(),
            clips: Vec::new(),
            sub_bins: Vec::new(),
            expanded: true,
            created_at: chrono::Utc::now(),
        });
        let root_bin = bins[root_key].id;
        
        let mut bin_id_to_key = HashMap::new();
        bin_id_to_key.insert(root_bin, root_key);
        
        Self {
            clips: RwLock::new(SlotMap::with_key()),
            bins: RwLock::new(bins),
            clip_id_to_key: RwLock::new(HashMap::new()),
            bin_id_to_key: RwLock::new(bin_id_to_key),
            root_bin,
            project_id,
            cache,
            proxy_manager: Arc::new(ProxyManager::new()),
            thumbnail_cache: Arc::new(ThumbnailCache::new(1000)),
            waveform_cache: Arc::new(WaveformCache::new(200)),
        }
    }

    pub async fn add_clip(&self, metadata: ClipMetadata) -> Result<ClipId> {
        let clip_id = metadata.id;
        
        // Check for duplicate
        if self.clip_id_to_key.read().contains_key(&clip_id) {
            return Err(ProjectError::DuplicateName(metadata.tags.name).into());
        }
        
        // Extract full media metadata
        let media_metadata = MediaMetadata::from_file(&PathBuf::from(&metadata.file_path)).ok();
        
        let clip = PoolClip {
            id: clip_id,
            key: ClipKey::default(),
            bin: self.root_bin,
            metadata: metadata.clone(),
            media_metadata,
            proxy: None,
            thumbnails: Vec::new(),
            waveform: None,
            status: ClipStatus::Online,
            offline_path: None,
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
        };
        
        let mut clips = self.clips.write();
        let key = clips.insert(clip);
        clips[key].key = key;
        
        self.clip_id_to_key.write().insert(clip_id, key);
        
        // Add to root bin
        let mut bins = self.bins.write();
        if let Some(bin) = bins.get_mut(self.bin_id_to_key.read()[&self.root_bin]) {
            bin.clips.push(clip_id);
        }
        
        // Generate thumbnail asynchronously
        self.generate_thumbnails_async(clip_id).await;
        
        // Generate waveform asynchronously
        self.generate_waveform_async(clip_id).await;
        
        Ok(clip_id)
    }

    pub fn get_clip(&self, clip_id: ClipId) -> Option<PoolClip> {
        let key = self.clip_id_to_key.read().get(&clip_id).copied()?;
        self.clips.read().get(key).cloned()
    }

    pub fn get_clip_mut(&self, clip_id: ClipId) -> Option<PoolClip> {
        let key = self.clip_id_to_key.read().get(&clip_id).copied()?;
        self.clips.write().get(key).cloned()
    }

    pub fn update_clip(&self, clip_id: ClipId, f: impl FnOnce(&mut PoolClip)) -> bool {
        if let Some(key) = self.clip_id_to_key.read().get(&clip_id).copied() {
            if let Some(clip) = self.clips.write().get_mut(key) {
                f(clip);
                clip.modified_at = chrono::Utc::now();
                return true;
            }
        }
        false
    }

    pub fn remove_clip(&self, clip_id: ClipId) -> Option<PoolClip> {
        let key = self.clip_id_to_key.write().remove(&clip_id)?;
        let clip = self.clips.write().remove(key)?;
        
        // Remove from bin
        if let Some(bin_key) = self.bin_id_to_key.read().get(&clip.bin) {
            if let Some(bin) = self.bins.write().get_mut(*bin_key) {
                bin.clips.retain(|id| *id != clip_id);
            }
        }
        
        // Clean caches
        self.cache.evict_clip_frames(clip_id);
        self.proxy_manager.remove(clip_id);
        self.thumbnail_cache.clear(); // Would need clip-specific clear
        self.waveform_cache.clear();
        
        Some(clip)
    }

    pub fn move_clip(&self, clip_id: ClipId, target_bin: BinId) -> bool {
        let clip_key = match self.clip_id_to_key.read().get(&clip_id).copied() {
            Some(k) => k,
            None => return false,
        };
        // Collect the slot keys first: holding a clip borrow while taking a
        // mutable bin borrow would deadlock the pools against each other.
        let old_bin = {
            let clips = self.clips.read();
            clips.get(clip_key).map(|c| c.bin)
        };

        if let Some(old_bin) = old_bin {
            if let Some(old_key) = self.bin_id_to_key.read().get(&old_bin).copied() {
                if let Some(bin) = self.bins.write().get_mut(old_key) {
                    bin.clips.retain(|id| *id != clip_id);
                }
            }
        }

        if let Some(new_key) = self.bin_id_to_key.read().get(&target_bin).copied() {
            if let Some(bin) = self.bins.write().get_mut(new_key) {
                bin.clips.push(clip_id);
            }
        }

        let mut clips = self.clips.write();
        match clips.get_mut(clip_key) {
            Some(clip) => {
                clip.bin = target_bin;
                clip.modified_at = chrono::Utc::now();
                true
            }
            None => false,
        }
    }

    /// The pool's implicit root bin. Bins below it form the media-pool tree.
    pub fn root_bin(&self) -> BinId {
        self.root_bin
    }

    pub fn list_clips(&self, bin_id: Option<BinId>) -> Vec<ClipId> {
        let bin_id = bin_id.unwrap_or(self.root_bin);
        let bin_key = self.bin_id_to_key.read().get(&bin_id).copied();
        match bin_key {
            Some(k) => self.bins.read().get(k).map(|b| b.clips.clone()).unwrap_or_default(),
            None => Vec::new(),
        }
    }

    pub fn list_all_clips(&self) -> Vec<ClipId> {
        self.clips.read().values().map(|c| c.id).collect()
    }

    // Bin operations
    pub fn create_bin(&self, name: String, parent: Option<BinId>) -> Result<BinId> {
        let parent = parent.unwrap_or(self.root_bin);
        
        let mut bins = self.bins.write();
        let key = bins.insert(PoolBin {
            id: BinId::generate(),
            key: BinKey::default(),
            parent: Some(parent),
            name,
            clips: Vec::new(),
            sub_bins: Vec::new(),
            expanded: false,
            created_at: chrono::Utc::now(),
        });
        bins[key].key = key;
        let bin_id = bins[key].id;
        
        self.bin_id_to_key.write().insert(bin_id, key);
        
        // Add to parent
        if let Some(parent_key) = self.bin_id_to_key.read().get(&parent) {
            if let Some(parent_bin) = bins.get_mut(*parent_key) {
                parent_bin.sub_bins.push(bin_id);
            }
        }
        
        Ok(bin_id)
    }

    pub fn rename_bin(&self, bin_id: BinId, name: String) -> bool {
        if let Some(key) = self.bin_id_to_key.read().get(&bin_id).copied() {
            if let Some(bin) = self.bins.write().get_mut(key) {
                bin.name = name;
                return true;
            }
        }
        false
    }

    pub fn delete_bin(&self, bin_id: BinId, move_clips_to: Option<BinId>) -> bool {
        let key = match self.bin_id_to_key.write().remove(&bin_id) {
            Some(k) => k,
            None => return false,
        };
        let bin = match self.bins.write().remove(key) {
            Some(b) => b,
            None => return false,
        };
        
        // Move clips
        let target = move_clips_to.unwrap_or(self.root_bin);
        for clip_id in bin.clips {
            self.move_clip(clip_id, target);
        }
        
        // Recursively delete sub-bins
        for sub_bin in bin.sub_bins {
            self.delete_bin(sub_bin, Some(target));
        }
        
        // Remove from parent
        if let Some(parent) = bin.parent {
            if let Some(parent_key) = self.bin_id_to_key.read().get(&parent) {
                if let Some(parent_bin) = self.bins.write().get_mut(*parent_key) {
                    parent_bin.sub_bins.retain(|id| *id != bin_id);
                }
            }
        }
        
        true
    }

    pub fn get_bin(&self, bin_id: BinId) -> Option<PoolBin> {
        let key = self.bin_id_to_key.read().get(&bin_id).copied()?;
        self.bins.read().get(key).cloned()
    }

    pub fn list_bins(&self, parent: Option<BinId>) -> Vec<BinId> {
        let parent = parent.unwrap_or(self.root_bin);
        match self.bin_id_to_key.read().get(&parent).copied() {
            Some(key) => self.bins.read().get(key).map(|b| b.sub_bins.clone()).unwrap_or_default(),
            None => Vec::new(),
        }
    }

    pub fn bin_path(&self, bin_id: BinId) -> Vec<BinId> {
        let mut path = Vec::new();
        let mut current = Some(bin_id);
        while let Some(id) = current {
            path.push(id);
            current = self.get_bin(id).and_then(|b| b.parent);
        }
        path.reverse();
        path
    }

    // Search
    pub fn search(&self, query: &str) -> Vec<ClipId> {
        let query = query.to_lowercase();
        self.clips.read().values()
            .filter(|c| {
                c.metadata.tags.name.to_lowercase().contains(&query)
                || c.metadata.file_path.to_lowercase().contains(&query)
                || c.metadata.tags.description.to_lowercase().contains(&query)
            })
            .map(|c| c.id)
            .collect()
    }

    pub fn filter_by_type(&self, media_type: MediaType) -> Vec<ClipId> {
        self.clips.read().values()
            .filter(|c| c.metadata.media_type == media_type)
            .map(|c| c.id)
            .collect()
    }

    pub fn filter_by_status(&self, status: ClipStatus) -> Vec<ClipId> {
        self.clips.read().values()
            .filter(|c| c.status == status)
            .map(|c| c.id)
            .collect()
    }

    // Proxy management
    pub fn set_proxy(&self, clip_id: ClipId, proxy: crate::media::proxy::ProxyMetadata) {
        if let Some(key) = self.clip_id_to_key.read().get(&clip_id).copied() {
            if let Some(clip) = self.clips.write().get_mut(key) {
                clip.proxy = Some(proxy.clone());
                clip.status = ClipStatus::ProxyOnly;
            }
        }
        self.proxy_manager.add(proxy);
    }

    pub fn get_proxy(&self, clip_id: ClipId) -> Option<crate::media::proxy::ProxyMetadata> {
        self.proxy_manager.get(clip_id)
    }

    pub fn enable_proxies(&self, enable: bool) {
        self.proxy_manager.set_active(enable);
    }

    // Thumbnail generation
    async fn generate_thumbnails_async(&self, clip_id: ClipId) {
        let clip = self.get_clip(clip_id);
        if clip.is_none() { return; }
        
        let path = clip.as_ref().unwrap().metadata.file_path.clone();
        let duration = clip.as_ref().unwrap().metadata.duration;
        
        // Generate at multiple positions
        let frames = vec![
            Frame(0),
            Frame(duration.0 / 4),
            Frame(duration.0 / 2),
            Frame(duration.0 * 3 / 4),
        ];
        
        // This would be done in a background task
        // For now, just generate first frame
        if let Ok(thumb) = crate::media::thumbnail::ThumbnailGenerator::default().generate(&PathBuf::from(&path), Frame(0)) {
            self.update_clip(clip_id, |c| c.thumbnails.push(thumb.clone()));
            self.thumbnail_cache.put(clip_id, Frame(0), thumb);
        }
    }

    async fn generate_waveform_async(&self, clip_id: ClipId) {
        let clip = self.get_clip(clip_id);
        if clip.is_none() { return; }
        
        let path = clip.as_ref().unwrap().metadata.file_path.clone();
        
        if let Ok(waveform) = crate::media::waveform::WaveformGenerator::default().generate(&PathBuf::from(&path)) {
            self.update_clip(clip_id, |c| c.waveform = Some(waveform.clone()));
            self.waveform_cache.put(clip_id, waveform);
        }
    }

    // Relinking
    pub async fn relink_clip(&self, clip_id: ClipId, new_path: PathBuf) -> Result<()> {
        let metadata = MediaMetadata::from_file(&new_path)?;
        let new_clip_metadata = metadata.to_clip_metadata(clip_id);

        if !self.update_clip(clip_id, |c| {
            c.metadata = new_clip_metadata;
            c.metadata.file_path = new_path.to_string_lossy().to_string();
            c.status = ClipStatus::Online;
            c.offline_path = None;
            c.media_metadata = Some(metadata);
        }) {
            return Err(MediaError::NotFound("Clip not found".into()).into());
        }

        // Regenerate thumbnails and waveform
        self.generate_thumbnails_async(clip_id).await;
        self.generate_waveform_async(clip_id).await;

        Ok(())
    }

    pub fn mark_offline(&self, clip_id: ClipId, original_path: PathBuf) {
        self.update_clip(clip_id, |c| {
            c.status = ClipStatus::Offline;
            c.offline_path = Some(original_path);
        });
    }

    // Statistics
    pub fn stats(&self) -> PoolStats {
        let clips = self.clips.read();
        let bins = self.bins.read();
        
        let mut online = 0;
        let mut offline = 0;
        let mut proxy_only = 0;
        let mut total_size = 0u64;
        let mut total_duration = Frame::ZERO;
        
        for clip in clips.values() {
            match clip.status {
                ClipStatus::Online => online += 1,
                ClipStatus::Offline => offline += 1,
                ClipStatus::ProxyOnly => proxy_only += 1,
                _ => {}
            }
            total_size += clip.metadata.file_size();
            total_duration = total_duration + clip.metadata.duration;
        }
        
        PoolStats {
            total_clips: clips.len(),
            total_bins: bins.len(),
            online_clips: online,
            offline_clips: offline,
            proxy_only_clips: proxy_only,
            total_size_bytes: total_size,
            total_duration,
        }
    }
}

impl PoolClip {
    fn file_size(&self) -> u64 {
        std::fs::metadata(&self.metadata.file_path).map(|m| m.len()).unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_clips: usize,
    pub total_bins: usize,
    pub online_clips: usize,
    pub offline_clips: usize,
    pub proxy_only_clips: usize,
    pub total_size_bytes: u64,
    pub total_duration: Frame,
}

/// Media pool events for UI updates
#[derive(Debug, Clone)]
pub enum PoolEvent {
    ClipAdded(ClipId),
    ClipRemoved(ClipId),
    ClipUpdated(ClipId),
    ClipMoved { clip_id: ClipId, from_bin: BinId, to_bin: BinId },
    ClipStatusChanged(ClipId, ClipStatus),
    BinAdded(BinId),
    BinRemoved(BinId),
    BinRenamed(BinId),
    BinMoved { bin_id: BinId, from_parent: BinId, to_parent: BinId },
}

/// Pool observer for reactive UI
pub struct PoolObserver {
    sender: crossbeam::channel::Sender<PoolEvent>,
}

impl PoolObserver {
    pub fn new() -> (Self, crossbeam::channel::Receiver<PoolEvent>) {
        let (tx, rx) = crossbeam::channel::unbounded();
        (Self { sender: tx }, rx)
    }

    pub fn emit(&self, event: PoolEvent) {
        self.sender.send(event).ok();
    }
}