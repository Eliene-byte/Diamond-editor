//! Application configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::core::types::{Resolution, FrameRate, ColorSpace, PixelFormat};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub general: GeneralConfig,
    pub ui: UiConfig,
    pub media: MediaConfig,
    pub timeline: TimelineConfig,
    pub color: ColorConfig,
    pub audio: AudioConfig,
    pub gpu: GpuConfig,
    pub cache: CacheConfig,
    pub paths: PathsConfig,
    pub shortcuts: ShortcutsConfig,
    pub advanced: AdvancedConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            general: GeneralConfig::default(),
            ui: UiConfig::default(),
            media: MediaConfig::default(),
            timeline: TimelineConfig::default(),
            color: ColorConfig::default(),
            audio: AudioConfig::default(),
            gpu: GpuConfig::default(),
            cache: CacheConfig::default(),
            paths: PathsConfig::default(),
            shortcuts: ShortcutsConfig::default(),
            advanced: AdvancedConfig::default(),
        }
    }
}

const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub language: String,
    pub auto_save_interval_sec: u32,
    pub max_undo_levels: u32,
    pub check_updates: bool,
    pub send_telemetry: bool,
    pub start_page: StartPage,
    pub recent_projects_limit: usize,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            language: "en".into(),
            auto_save_interval_sec: 300,
            max_undo_levels: 100,
            check_updates: true,
            send_telemetry: false,
            start_page: StartPage::ProjectManager,
            recent_projects_limit: 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartPage {
    ProjectManager,
    NewProject,
    LastProject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Theme,
    pub font_size: f32,
    pub font_family: String,
    pub ui_scale: f32,
    pub show_tooltips: bool,
    pub tooltip_delay_ms: u32,
    pub panel_opacity: f32,
    pub accent_color: String,
    pub timeline_thumbnail_size: ThumbnailSize,
    pub show_timecode: bool,
    pub timecode_format: TimecodeFormat,
    pub playback_controls: PlaybackControlsStyle,
    pub scopes_layout: ScopesLayout,
    pub node_graph_style: NodeGraphStyle,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            font_size: 13.0,
            font_family: "Inter".into(),
            ui_scale: 1.0,
            show_tooltips: true,
            tooltip_delay_ms: 500,
            panel_opacity: 0.95,
            accent_color: "#FF6B00".into(), // DaVinci orange
            timeline_thumbnail_size: ThumbnailSize::Medium,
            show_timecode: true,
            timecode_format: TimecodeFormat::Smpte,
            playback_controls: PlaybackControlsStyle::Classic,
            scopes_layout: ScopesLayout::Horizontal,
            node_graph_style: NodeGraphStyle::Classic,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
    DarkHighContrast,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThumbnailSize {
    None,
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimecodeFormat {
    Smpte,
    Frames,
    FeetFrames,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackControlsStyle {
    Classic,
    Minimal,
    TransportOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScopesLayout {
    Horizontal,
    Vertical,
    Grid,
    Single,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeGraphStyle {
    Classic,
    Compact,
    Minimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaConfig {
    pub auto_create_proxies: bool,
    pub proxy_resolution: ProxyResolution,
    pub proxy_codec: String,
    pub generate_thumbnails: bool,
    pub thumbnail_interval_sec: f32,
    pub audio_waveform_resolution: u32,
    pub max_concurrent_imports: usize,
    pub watch_folders: Vec<PathBuf>,
    pub supported_extensions: Vec<String>,
    pub ignore_patterns: Vec<String>,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            auto_create_proxies: false,
            proxy_resolution: ProxyResolution::Half,
            proxy_codec: "prores_ks".into(),
            generate_thumbnails: true,
            thumbnail_interval_sec: 1.0,
            audio_waveform_resolution: 1024,
            max_concurrent_imports: 4,
            watch_folders: Vec::new(),
            supported_extensions: vec![
                "mp4", "mov", "mxf", "avi", "mkv", "webm",
                "prores", "dnxhd", "dnxhr", "r3d", "braw",
                "wav", "aiff", "flac", "mp3", "aac", "ogg",
                "jpg", "jpeg", "png", "tiff", "tif", "exr", "dpx", "cin", "hdr",
                "srt", "ass", "vtt",
            ].into_iter().map(String::from).collect(),
            ignore_patterns: vec![
                ".*".into(), "Thumbs.db".into(), ".DS_Store".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyResolution {
    Quarter,
    Half,
    ThreeQuarter,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineConfig {
    pub default_video_tracks: u32,
    pub default_audio_tracks: u32,
    pub default_resolution: Resolution,
    pub default_frame_rate: FrameRate,
    pub snap_enabled: bool,
    pub snap_threshold_px: f32,
    pub ripple_edit_default: bool,
    pub show_audio_waveforms: bool,
    pub waveform_height: f32,
    pub track_height: TrackHeight,
    pub playhead_style: PlayheadStyle,
    pub time_ruler_format: TimeRulerFormat,
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            default_video_tracks: 4,
            default_audio_tracks: 8,
            default_resolution: Resolution::Uhd4k,
            default_frame_rate: FrameRate::Fps24,
            snap_enabled: true,
            snap_threshold_px: 10.0,
            ripple_edit_default: false,
            show_audio_waveforms: true,
            waveform_height: 40.0,
            track_height: TrackHeight::Medium,
            playhead_style: PlayheadStyle::Line,
            time_ruler_format: TimeRulerFormat::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackHeight {
    Small,
    Medium,
    Large,
    Custom(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayheadStyle {
    Line,
    Triangle,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeRulerFormat {
    Auto,
    Timecode,
    Frames,
    Seconds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub default_color_space: ColorSpace,
    pub default_pixel_format: PixelFormat,
    pub working_gamma: f32,
    pub viewer_lut: Option<String>,
    pub scopes_color_space: ColorSpace,
    pub enable_gpu_grade: bool,
    pub cache_grade_nodes: bool,
    pub lut_directory: Option<PathBuf>,
    pub still_store_limit: usize,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            default_color_space: ColorSpace::Rec709,
            default_pixel_format: PixelFormat::Rgba16F,
            working_gamma: 2.4,
            viewer_lut: None,
            scopes_color_space: ColorSpace::Rec709,
            enable_gpu_grade: true,
            cache_grade_nodes: true,
            lut_directory: None,
            still_store_limit: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub bit_depth: u32,
    pub channels: u16,
    pub buffer_size: u32,
    pub latency_mode: LatencyMode,
    pub master_volume: f32,
    pub metering: MeteringConfig,
    pub auto_mute_on_focus_loss: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            bit_depth: 32,
            channels: 2,
            buffer_size: 512,
            latency_mode: LatencyMode::Low,
            master_volume: 1.0,
            metering: MeteringConfig::default(),
            auto_mute_on_focus_loss: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LatencyMode {
    Low,
    Balanced,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteringConfig {
    pub peak_hold_time_ms: u32,
    pub rms_window_ms: u32,
    pub true_peak: bool,
    pub k_weighted: bool,
}

impl Default for MeteringConfig {
    fn default() -> Self {
        Self {
            peak_hold_time_ms: 2000,
            rms_window_ms: 300,
            true_peak: true,
            k_weighted: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    pub backend: GpuBackend,
    pub adapter_name: Option<String>,
    pub power_preference: PowerPreference,
    pub features: GpuFeatures,
    pub limits: GpuLimits,
    pub vsync: bool,
    pub max_frames_in_flight: u32,
    pub enable_debug_layer: bool,
    pub enable_gpu_validation: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            backend: GpuBackend::Auto,
            adapter_name: None,
            power_preference: PowerPreference::HighPerformance,
            features: GpuFeatures::default(),
            limits: GpuLimits::default(),
            vsync: true,
            max_frames_in_flight: 2,
            enable_debug_layer: false,
            enable_gpu_validation: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuBackend {
    Auto,
    Vulkan,
    Metal,
    Dx12,
    Dx11,
    Gl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerPreference {
    LowPower,
    HighPerformance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuFeatures {
    pub texture_compression_bc: bool,
    pub texture_compression_etc2: bool,
    pub texture_compression_astc: bool,
    pub timestamp_query: bool,
    pub pipeline_statistics: bool,
    pub subgroup: bool,
}

impl Default for GpuFeatures {
    fn default() -> Self {
        Self {
            texture_compression_bc: true,
            texture_compression_etc2: true,
            texture_compression_astc: true,
            timestamp_query: true,
            pipeline_statistics: false,
            subgroup: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuLimits {
    pub max_texture_dimension_2d: u32,
    pub max_texture_dimension_3d: u32,
    pub max_bind_groups: u32,
    pub max_uniform_buffer_size: u32,
    pub max_storage_buffer_size: u32,
}

impl Default for GpuLimits {
    fn default() -> Self {
        Self {
            max_texture_dimension_2d: 16384,
            max_texture_dimension_3d: 2048,
            max_bind_groups: 8,
            max_uniform_buffer_size: 65536,
            max_storage_buffer_size: 1 << 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub cache_dir: Option<PathBuf>,
    pub max_size_gb: f32,
    pub max_age_days: u32,
    pub enable_proxy_cache: bool,
    pub enable_thumbnail_cache: bool,
    pub enable_waveform_cache: bool,
    pub enable_grade_cache: bool,
    pub enable_render_cache: bool,
    pub compression: CacheCompression,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: None,
            max_size_gb: 50.0,
            max_age_days: 30,
            enable_proxy_cache: true,
            enable_thumbnail_cache: true,
            enable_waveform_cache: true,
            enable_grade_cache: true,
            enable_render_cache: true,
            compression: CacheCompression::Lz4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheCompression {
    None,
    Lz4,
    Zstd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub projects_dir: Option<PathBuf>,
    pub exports_dir: Option<PathBuf>,
    pub renders_dir: Option<PathBuf>,
    pub luts_dir: Option<PathBuf>,
    pub presets_dir: Option<PathBuf>,
    pub scripts_dir: Option<PathBuf>,
    pub temp_dir: Option<PathBuf>,
}

impl Default for PathsConfig {
    fn default() -> Self {
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("DaVinciClone");
        Self {
            projects_dir: Some(base.join("Projects")),
            exports_dir: Some(base.join("Exports")),
            renders_dir: Some(base.join("Renders")),
            luts_dir: Some(base.join("LUTs")),
            presets_dir: Some(base.join("Presets")),
            scripts_dir: Some(base.join("Scripts")),
            temp_dir: Some(base.join("Temp")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    pub scheme: ShortcutScheme,
    pub custom: HashMap<String, String>,
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            scheme: ShortcutScheme::DaVinci,
            custom: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShortcutScheme {
    DaVinci,
    Premiere,
    FinalCut,
    Avid,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    pub enable_experimental: bool,
    pub log_level: LogLevel,
    pub max_threads: usize,
    pub memory_limit_gb: Option<f32>,
    pub network_rendering: bool,
    pub script_engine: ScriptEngine,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            enable_experimental: false,
            log_level: LogLevel::Info,
            max_threads: num_cpus::get(),
            memory_limit_gb: None,
            network_rendering: false,
            script_engine: ScriptEngine::Lua,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptEngine {
    None,
    Lua,
    Python,
}

/// Config manager
pub struct ConfigManager {
    config: AppConfig,
    path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path()?;
        let config = if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            let mut config: AppConfig = toml::from_str(&data)?;
            config = migrate_config(config);
            config
        } else {
            AppConfig::default()
        };
        Ok(Self { config, path })
    }

    fn config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut path = dirs::config_dir().ok_or("No config directory")?;
        path.push("DaVinciClone");
        std::fs::create_dir_all(&path)?;
        path.push("config.toml");
        Ok(path)
    }

    pub fn get(&self) -> &AppConfig { &self.config }
    pub fn get_mut(&mut self) -> &mut AppConfig { &mut self.config }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let data = toml::to_string_pretty(&self.config)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }

    pub fn reset(&mut self) { self.config = AppConfig::default(); }
}

fn migrate_config(mut config: AppConfig) -> AppConfig {
    if config.version < CONFIG_VERSION {
        // Add migration logic here
        config.version = CONFIG_VERSION;
    }
    config
}