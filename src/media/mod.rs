//! Media management - import, decode, proxy, thumbnails, waveforms

pub mod scanner;
pub mod decoder;
pub mod proxy;
pub mod thumbnail;
pub mod waveform;
pub mod metadata;
pub mod cache;
pub mod pool;

// Explicit re-exports to avoid glob ambiguity.

pub use scanner::{MediaScanner, ScanEvent, QuickProbeInfo, VideoStreamInfo, AudioStreamInfo, SubtitleStreamInfo, BatchImporter, ImportResult, ImportFailure, detect_media_type};
pub use decoder::{VideoFrame, VideoFrameData, AudioFrame, AudioFrameData, AudioFormat, VideoDecoder, AudioDecoder, MediaDecoder};
pub use proxy::{ProxyPreset, ProxyConfig, ProxyMetadata, ProxyGenerator, ProxyManager, SmartProxyRenderer};
pub use thumbnail::{ThumbnailGenerator, ThumbnailFormat, ThumbnailSprite, ThumbnailCache};
pub use waveform::{Waveform, WaveformPeak, WaveformGenerator, MultiResolutionWaveform, WaveformLevel, WaveformCache};
pub use metadata::{MediaMetadata, VideoStreamMetadata, AudioStreamMetadata, SubtitleStreamMetadata, ChapterMetadata};
pub use cache::{MediaCache, CacheConfig, CacheCompression, CacheQuality, CacheStats, PredictiveFrameCache, PlaybackDirection};
pub use pool::{MediaPool, PoolClip, PoolBin, ClipStatus, PoolStats, PoolEvent, PoolObserver};