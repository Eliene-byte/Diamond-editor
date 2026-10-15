//! GPU rendering engine - Vulkan/Metal/DX12 via wgpu

pub mod buffer;
pub mod color;
pub mod compute;
pub mod pipeline;
pub mod playback;
pub mod renderer;
pub mod scopes;
pub mod text;
pub mod texture;

// Explicit re-exports to avoid glob ambiguity.
// Notably: `ColorGradeUniforms` is defined once in `buffer` and re-exported by
// `renderer`; `ScopesUniforms` is defined once in `buffer`. `ComputePipelineEntry`
// (buffer.rs) and `ComputePipeline` (compute.rs) are distinct types by design.

pub use buffer::{
    BufferManager, FrameUniforms, InstanceData, ScopesUniforms, StorageBuffer,
    TransitionPushConstants, UniformBuffer,
};
pub use color::ColorPipeline;
pub use compute::ComputePipeline;
pub use compute::ComputePipelineEntry;
pub use pipeline::{
    ComputeShaderType, PipelineKey, PipelineManager, RenderPipelineBundle, TransitionPipelineType,
};
pub use playback::{DisplayFrame, FrameMetadata, PlaybackEngine};
pub use renderer::{
    BlendMode, CompositeLayer, ComputeParams, ComputePipelineType, CpuRenderer, GpuRenderer,
    HeadlessRenderer, RenderCommand, RenderStats, RendererBackend,
};
pub use scopes::{ScopeMode, ScopesRenderer};
pub use texture::{GpuTexture, SamplerKey, TextureManager, TexturePool};
