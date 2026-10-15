//! Fallback proxy-media support for builds **without** the `codec-ffmpeg`
//! feature.
//!
//! Proxy transcoding requires an encoder, so without one nothing can be
//! generated. The API mirrors `proxy.rs` so the rest of the crate compiles
//! unchanged and callers get a clear error rather than a missing type.

use crate::core::id::{ClipId, ProxyId};
use crate::core::time::Frame;
use crate::core::error::{AppError, MediaError, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Quality tiers used when building proxy media.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyPreset {
    Half,
    Quarter,
    Eighth,
    Custom { width: u32, bitrate_kbps: u32 },
}

impl ProxyPreset {
    /// Target width in pixels for this preset.
    pub fn target_width(&self) -> u32 {
        match self {
            ProxyPreset::Half => 960,
            ProxyPreset::Quarter => 480,
            ProxyPreset::Eighth => 240,
            ProxyPreset::Custom { width, .. } => *width,
        }
    }

    /// Target bitrate in kbps for this preset.
    pub fn target_bitrate_kbps(&self) -> u32 {
        match self {
            ProxyPreset::Half => 4000,
            ProxyPreset::Quarter => 1500,
            ProxyPreset::Eighth => 500,
            ProxyPreset::Custom { bitrate_kbps, .. } => *bitrate_kbps,
        }
    }
}

/// Where and how proxies are stored.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub cache_dir: PathBuf,
    pub preset: ProxyPreset,
    pub codec: String,
    pub enabled: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from(".davinci-clone/proxies"),
            preset: ProxyPreset::Quarter,
            codec: "h264".to_string(),
            enabled: false,
        }
    }
}

impl ProxyConfig {
    pub fn new(cache_dir: PathBuf, preset: ProxyPreset) -> Self {
        Self { cache_dir, preset, ..Default::default() }
    }
}

/// Record of a generated proxy for a clip.
#[derive(Debug, Clone)]
pub struct ProxyMetadata {
    pub proxy_id: ProxyId,
    pub clip_id: ClipId,
    pub path: PathBuf,
    pub preset: ProxyPreset,
    pub width: u32,
    pub height: u32,
    pub bitrate_kbps: u32,
    pub duration: Frame,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Transcodes source clips into lightweight proxies.
#[derive(Debug, Clone, Default)]
pub struct ProxyGenerator {
    config: ProxyConfig,
}

impl ProxyGenerator {
    pub fn new(config: ProxyConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &ProxyConfig {
        &self.config
    }

    /// Always fails: transcoding needs an encoder backend.
    pub fn generate(&self, _clip_id: ClipId, _source: &PathBuf) -> Result<ProxyMetadata> {
        Err(AppError::Media(MediaError::ProxyFailed(
            "no codec backend compiled in - rebuild with the `codec-ffmpeg` feature".to_string(),
        )))
    }
}

/// Tracks which clips have proxies and requests them on demand.
#[derive(Debug)]
pub struct ProxyManager {
    config: ProxyConfig,
    active: AtomicBool,
    proxies: Mutex<Vec<ProxyMetadata>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            config: ProxyConfig::default(),
            active: AtomicBool::new(false),
            proxies: Mutex::new(Vec::new()),
        }
    }

    pub fn with_config(config: ProxyConfig) -> Self {
        Self { config, active: AtomicBool::new(false), proxies: Mutex::new(Vec::new()) }
    }

    pub fn config(&self) -> &ProxyConfig {
        &self.config
    }

    /// Whether proxies should be used for playback.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Turn proxy playback on or off.
    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Relaxed);
    }

    /// Record a proxy. Used by the real backend after a successful transcode.
    pub fn add(&self, metadata: ProxyMetadata) {
        let mut proxies = self.proxies.lock().unwrap_or_else(|e| e.into_inner());
        proxies.retain(|p| p.clip_id != metadata.clip_id);
        proxies.push(metadata);
    }

    pub fn get(&self, clip_id: ClipId) -> Option<ProxyMetadata> {
        let proxies = self.proxies.lock().unwrap_or_else(|e| e.into_inner());
        proxies.iter().find(|p| p.clip_id == clip_id).cloned()
    }

    pub fn remove(&self, clip_id: ClipId) -> bool {
        let mut proxies = self.proxies.lock().unwrap_or_else(|e| e.into_inner());
        let before = proxies.len();
        proxies.retain(|p| p.clip_id != clip_id);
        proxies.len() != before
    }

    pub fn clear(&self) {
        self.proxies.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    pub fn len(&self) -> usize {
        self.proxies.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Chooses whether to use a proxy based on playback pressure.
#[derive(Debug, Clone, Copy, Default)]
pub struct SmartProxyRenderer {
    /// Frame time above which playback is considered degraded.
    pub frame_budget_ms: f32,
}

impl SmartProxyRenderer {
    pub fn new() -> Self {
        Self { frame_budget_ms: 16.6 }
    }

    /// True when `frame_time_ms` exceeds the budget and a proxy should be used.
    pub fn should_use_proxy(&self, frame_time_ms: f32) -> bool {
        frame_time_ms > self.frame_budget_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(clip: u64) -> ProxyMetadata {
        ProxyMetadata {
            proxy_id: ProxyId(clip),
            clip_id: ClipId(clip),
            path: PathBuf::from(format!("/tmp/proxy{clip}.mov")),
            preset: ProxyPreset::Quarter,
            width: 480,
            height: 270,
            bitrate_kbps: 1500,
            duration: Frame(240),
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn presets_expose_their_targets() {
        assert_eq!(ProxyPreset::Quarter.target_width(), 480);
        assert_eq!(ProxyPreset::Half.target_bitrate_kbps(), 4000);
        assert_eq!(
            ProxyPreset::Custom { width: 320, bitrate_kbps: 800 }.target_width(),
            320
        );
    }

    #[test]
    fn generation_reports_missing_backend() {
        let gen = ProxyGenerator::new(ProxyConfig::default());
        assert!(gen.generate(ClipId(1), &PathBuf::from("a.mov")).is_err());
    }

    #[test]
    fn manager_deduplicates_by_clip() {
        let m = ProxyManager::new();
        assert!(m.is_empty());

        m.add(meta(1));
        m.add(meta(1));
        assert_eq!(m.len(), 1, "re-adding the same clip must replace, not duplicate");

        assert!(m.get(ClipId(1)).is_some());
        assert!(m.remove(ClipId(1)));
        assert!(m.is_empty());
        assert!(!m.remove(ClipId(42)));
    }

    #[test]
    fn active_flag_round_trips() {
        let m = ProxyManager::new();
        assert!(!m.is_active());
        m.set_active(true);
        assert!(m.is_active());
    }

    #[test]
    fn smart_renderer_switches_on_overrun() {
        let s = SmartProxyRenderer::new();
        assert!(!s.should_use_proxy(8.0));
        assert!(s.should_use_proxy(40.0));
    }
}