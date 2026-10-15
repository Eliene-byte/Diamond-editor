//! Media management - import, decode, proxy, thumbnails, waveforms.
//!
//! Backend-dependent modules (everything that talks to FFmpeg) have two
//! implementations: a real one gated behind `codec-ffmpeg`, and a `*_stub`
//! fallback that keeps the same public API and reports a clear error. That way
//! the crate builds and its tests run on machines with no codec toolchain —
//! CI's core-only and cpu-only matrix entries included — and so does the
//! `--no-default-features` path developers use for fast checks.

pub mod frame;
pub mod cache;
pub mod pool;

// --- Backend-gated modules -------------------------------------------------
#[cfg(feature = "codec-ffmpeg")]
#[path = "scanner.rs"]
pub mod scanner;
#[cfg(not(feature = "codec-ffmpeg"))]
#[path = "scanner_stub.rs"]
pub mod scanner;

#[cfg(feature = "codec-ffmpeg")]
#[path = "decoder.rs"]
pub mod decoder;
#[cfg(not(feature = "codec-ffmpeg"))]
#[path = "decoder_stub.rs"]
pub mod decoder;

#[cfg(feature = "codec-ffmpeg")]
#[path = "proxy.rs"]
pub mod proxy;
#[cfg(not(feature = "codec-ffmpeg"))]
#[path = "proxy_stub.rs"]
pub mod proxy;

#[cfg(feature = "codec-ffmpeg")]
#[path = "thumbnail.rs"]
pub mod thumbnail;
#[cfg(not(feature = "codec-ffmpeg"))]
#[path = "thumbnail_stub.rs"]
pub mod thumbnail;

#[cfg(feature = "codec-ffmpeg")]
#[path = "waveform.rs"]
pub mod waveform;
#[cfg(not(feature = "codec-ffmpeg"))]
#[path = "waveform_stub.rs"]
pub mod waveform;

#[cfg(feature = "codec-ffmpeg")]
#[path = "metadata.rs"]
pub mod metadata;
#[cfg(not(feature = "codec-ffmpeg"))]
#[path = "metadata_stub.rs"]
pub mod metadata;

// --- Re-exports ------------------------------------------------------------
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