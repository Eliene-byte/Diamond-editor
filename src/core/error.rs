//! Error types for the application

use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Media error: {0}")]
    Media(#[from] MediaError),

    #[error("Timeline error: {0}")]
    Timeline(#[from] TimelineError),

    #[error("Project error: {0}")]
    Project(#[from] ProjectError),

    #[error("GPU error: {0}")]
    Gpu(#[from] GpuError),

    #[error("Codec error: {0}")]
    Codec(#[from] CodecError),

    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),

    #[error("UI error: {0}")]
    Ui(#[from] UiError),

    #[error("Command error: {0}")]
    Command(#[from] CommandError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    #[error("Out of memory")]
    OutOfMemory,

    #[error("Cancelled")]
    Cancelled,

    #[error("Timeout")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug)]
pub enum MediaError {
    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Corrupted file: {0}")]
    Corrupted(String),

    #[error("No video stream")]
    NoVideoStream,

    #[error("No audio stream")]
    NoAudioStream,

    #[error("Decoder error: {0}")]
    Decoder(String),

    #[error("Encoder error: {0}")]
    Encoder(String),

    #[error("Seek failed: {0}")]
    SeekFailed(String),

    #[error("Frame not available: {0}")]
    FrameNotAvailable(String),

    #[error("Proxy generation failed: {0}")]
    ProxyFailed(String),

    #[error("Thumbnail generation failed: {0}")]
    ThumbnailFailed(String),

    #[error("Metadata read failed: {0}")]
    MetadataFailed(String),

    #[error("Color space conversion failed: {0}")]
    ColorSpaceFailed(String),
}

#[derive(Error, Debug)]
pub enum TimelineError {
    #[error("Track not found: {0}")]
    TrackNotFound(String),

    #[error("Clip not found: {0}")]
    ClipNotFound(String),

    #[error("Invalid position: {0}")]
    InvalidPosition(String),

    #[error("Overlap not allowed: {0}")]
    OverlapNotAllowed(String),

    #[error("Gap not allowed: {0}")]
    GapNotAllowed(String),

    #[error("Invalid trim: {0}")]
    InvalidTrim(String),

    #[error("Invalid split: {0}")]
    InvalidSplit(String),

    #[error("Cannot join clips: {0}")]
    CannotJoin(String),

    #[error("Track locked: {0}")]
    TrackLocked(String),

    #[error("Track hidden: {0}")]
    TrackHidden(String),

    #[error("Invalid transition: {0}")]
    InvalidTransition(String),

    #[error("Marker not found: {0}")]
    MarkerNotFound(String),

    #[error("Undo stack empty")]
    UndoEmpty,

    #[error("Redo stack empty")]
    RedoEmpty,
}

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("Project not found: {0}")]
    NotFound(String),

    #[error("Project already open: {0}")]
    AlreadyOpen(String),

    #[error("Project save failed: {0}")]
    SaveFailed(String),

    #[error("Project load failed: {0}")]
    LoadFailed(String),

    #[error("Invalid project settings: {0}")]
    InvalidSettings(String),

    #[error("Timeline not found: {0}")]
    TimelineNotFound(String),

    #[error("Bin not found: {0}")]
    BinNotFound(String),

    #[error("Duplicate name: {0}")]
    DuplicateName(String),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),
}

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("No GPU adapter found")]
    NoAdapter,

    #[error("Device creation failed: {0}")]
    DeviceCreation(String),

    #[error("Surface creation failed: {0}")]
    SurfaceCreation(String),

    #[error("Swapchain error: {0}")]
    Swapchain(String),

    #[error("Pipeline creation failed: {0}")]
    PipelineCreation(String),

    #[error("Shader compilation failed: {0}")]
    ShaderCompilation(String),

    #[error("Buffer creation failed: {0}")]
    BufferCreation(String),

    #[error("Texture creation failed: {0}")]
    TextureCreation(String),

    #[error("Bind group error: {0}")]
    BindGroup(String),

    #[error("Command buffer error: {0}")]
    CommandBuffer(String),

    #[error("Out of GPU memory")]
    OutOfMemory,

    #[error("Feature not supported: {0}")]
    UnsupportedFeature(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Device lost")]
    DeviceLost,
}

#[derive(Error, Debug)]
pub enum CodecError {
    #[error("Codec not found: {0}")]
    NotFound(String),

    #[error("Unsupported codec: {0}")]
    Unsupported(String),

    #[error("Encoding failed: {0}")]
    EncodingFailed(String),

    #[error("Decoding failed: {0}")]
    DecodingFailed(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Hardware acceleration not available: {0}")]
    NoHardwareAccel(String),

    #[error("Bitstream error: {0}")]
    Bitstream(String),
}

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Stream creation failed: {0}")]
    StreamCreation(String),

    #[error("Format not supported: {0}")]
    UnsupportedFormat(String),

    #[error("Buffer underrun")]
    BufferUnderrun,

    #[error("Resampling failed: {0}")]
    ResamplingFailed(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Mixing error: {0}")]
    Mixing(String),
}

#[derive(Error, Debug)]
pub enum UiError {
    #[error("Widget not found: {0}")]
    WidgetNotFound(String),

    #[error("Layout error: {0}")]
    Layout(String),

    #[error("Theme error: {0}")]
    Theme(String),

    #[error("Font not found: {0}")]
    FontNotFound(String),

    #[error("Input error: {0}")]
    Input(String),

    #[error("Window error: {0}")]
    Window(String),
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Command not found: {0}")]
    NotFound(String),

    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Undo failed: {0}")]
    UndoFailed(String),

    #[error("Redo failed: {0}")]
    RedoFailed(String),

    #[error("Command not undoable: {0}")]
    NotUndoable(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(String),

    #[error("Config parse failed: {0}")]
    ParseFailed(String),

    #[error("Config write failed: {0}")]
    WriteFailed(String),

    #[error("Invalid value: {0}")]
    InvalidValue(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),
}

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache dir not accessible: {0}")]
    DirNotAccessible(String),

    #[error("Cache entry not found: {0}")]
    NotFound(String),

    #[error("Cache write failed: {0}")]
    WriteFailed(String),

    #[error("Cache read failed: {0}")]
    ReadFailed(String),

    #[error("Cache corrupted: {0}")]
    Corrupted(String),

    #[error("Cache size limit exceeded")]
    SizeLimitExceeded,
}

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin load failed: {0}")]
    LoadFailed(String),

    #[error("Plugin init failed: {0}")]
    InitFailed(String),

    #[error("Plugin API version mismatch: {0}")]
    VersionMismatch(String),

    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Plugin unload failed: {0}")]
    UnloadFailed(String),
}

/// Error context for debugging
pub trait ErrorContext<T> {
    fn context<C: fmt::Display>(self, msg: C) -> Result<T>;
    fn with_context<C: fmt::Display, F: FnOnce() -> C>(self, f: F) -> Result<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> ErrorContext<T> for std::result::Result<T, E> {
    fn context<C: fmt::Display>(self, msg: C) -> Result<T> {
        self.map_err(|e| AppError::Internal(format!("{}: {}", msg, e)))
    }

    fn with_context<C: fmt::Display, F: FnOnce() -> C>(self, f: F) -> Result<T> {
        self.map_err(|e| AppError::Internal(format!("{}: {}", f(), e)))
    }
}