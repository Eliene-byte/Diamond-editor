//! GPU rendering engine - Vulkan/Metal/DX12 via wgpu

pub mod renderer;
pub mod pipeline;
pub mod texture;
pub mod buffer;
pub mod text;
pub mod compute;
pub mod color;
pub mod scopes;
pub mod playback;

// Explicit re-exports to avoid glob ambiguity.
// Notably: `ColorGradeUniforms` is defined once in `buffer` and re-exported by
// `renderer`; `ScopesUniforms` is defined once in `buffer`. `ComputePipelineEntry`
// (buffer.rs) and `ComputePipeline` (compute.rs) are distinct types by design.

pub use renderer::{
    GpuRenderer, CpuRenderer, RendererBackend, RenderCommand, BlendMode, CompositeLayer,
    ComputeParams, ComputePipelineType, RenderStats, HeadlessRenderer,
};
pub use buffer::{
    BufferManager, UniformBuffer, StorageBuffer, FrameUniforms, InstanceData,
    TransitionPushConstants, ScopesUniforms,
};
pub use texture::{TextureManager, GpuTexture, TexturePool, SamplerKey};
pub use pipeline::{PipelineManager, PipelineKey, RenderPipeline, TransitionPipelineType, ComputeShaderType};
pub use compute::ComputePipeline;
pub use compute::ComputePipelineEntry;
pub use color::ColorPipeline;
pub use scopes::{ScopesRenderer, ScopeMode};
pub use playback::{PlaybackEngine, DisplayFrame, FrameMetadata};