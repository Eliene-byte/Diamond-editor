//! Media management - import, decode, proxy, thumbnails, waveforms.
//!
//! The FFmpeg-backed implementations in `scanner.rs`, `decoder.rs`, `proxy.rs`,
//! `thumbnail.rs`, `waveform.rs` and `metadata.rs` were written before the crate
//! ever compiled and do not yet match `ffmpeg-next` 6.1. Until they are brought
//! into line, the modules compiled into the library are the `*_stub.rs`
//! fallbacks, which keep the public API identical and report a clear error. This
//! keeps `--no-default-features --features codec-ffmpeg` building and every
//! other feature set green; the real backends can be restored by `path = "..."` in
//! the entries below once their errors are cleared.

pub mod frame;
pub mod cache;
pub mod pool;

#[path = "decoder_stub.rs"]
pub mod decoder;

#[path = "scanner_stub.rs"]
pub mod scanner;

#[path = "proxy_stub.rs"]
pub mod proxy;

#[path = "thumbnail_stub.rs"]
pub mod thumbnail;

#[path = "waveform_stub.rs"]
pub mod waveform;

#[path = "metadata_stub.rs"]
pub mod metadata;

pub use frame::{AudioFormat, AudioFrame, AudioFrameData, VideoFrame, VideoFrameData};
pub use decoder::{AudioDecoder, MediaDecoder, VideoDecoder};
pub use cache::{
    CacheCompression, CacheConfig, CacheQuality, CacheStats, MediaCache, PlaybackDirection,
    PredictiveFrameCache,
};
pub use pool::{ClipStatus, MediaPool, PoolBin, PoolClip, PoolEvent, PoolObserver, PoolStats};

pub use scanner::{
    AudioStreamInfo, BatchImporter, ImportFailure, ImportResult, MediaScanner, QuickProbeInfo,
    ScanEvent, SubtitleStreamInfo, VideoStreamInfo, detect_media_type,
};
pub use proxy::{
    ProxyConfig, ProxyGenerator, ProxyManager, ProxyMetadata, ProxyPreset, SmartProxyRenderer,
};
pub use thumbnail::{ThumbnailCache, ThumbnailFormat, ThumbnailGenerator, ThumbnailSprite};
pub use waveform::{
    MultiResolutionWaveform, Waveform, WaveformCache, WaveformGenerator, WaveformLevel,
    WaveformPeak,
};
pub use metadata::{
    AudioStreamMetadata, ChapterMetadata, MediaMetadata, Rational, SubtitleStreamMetadata,
    VideoStreamMetadata,
};