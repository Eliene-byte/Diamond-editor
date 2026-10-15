//! GPU buffer management

use wgpu::{Device, Queue, Buffer, BufferDescriptor, BufferUsages, BufferAddress, BufferSize};
// `create_buffer_init` is provided by this extension trait; it must be in scope.
use wgpu::util::DeviceExt;
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};

/// Typed GPU buffer
pub struct GpuBuffer<T: Pod> {
    buffer: Arc<Buffer>,
    size: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Pod> GpuBuffer<T> {
    pub fn new(device: &Device, count: usize, usage: BufferUsages, label: Option<&str>) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label,
            size: (count * std::mem::size_of::<T>()) as BufferAddress,
            usage,
            mapped_at_creation: false,
        });
        
        Self {
            buffer: Arc::new(buffer),
            size: count,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn from_data(device: &Device, data: &[T], usage: BufferUsages, label: Option<&str>) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(data),
            usage,
        });
        
        Self {
            buffer: Arc::new(buffer),
            size: data.len(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn write(&self, queue: &Queue, offset: usize, data: &[T]) {
        queue.write_buffer(&self.buffer, (offset * std::mem::size_of::<T>()) as BufferAddress, bytemuck::cast_slice(data));
    }

    pub fn read(&self, queue: &Queue, offset: usize, count: usize) -> Vec<T> {
        let mut staging = self.buffer.clone(); // Would need actual staging buffer
        // Simplified - real implementation needs staging buffer
        vec![T::zeroed(); count]
    }

    pub fn as_entire_binding(&self) -> wgpu::BindingResource {
        self.buffer.as_entire_binding()
    }

    pub fn slice(&self, offset: usize, count: usize) -> wgpu::BufferBinding {
        wgpu::BufferBinding {
            buffer: &self.buffer,
            offset: (offset * std::mem::size_of::<T>()) as BufferAddress,
            size: BufferSize::new((count * std::mem::size_of::<T>()) as BufferAddress),
        }
    }

    pub fn len(&self) -> usize { self.size }
    pub fn is_empty(&self) -> bool { self.size == 0 }
    pub fn buffer(&self) -> &Arc<Buffer> { &self.buffer }
}

/// Uniform buffer for shaders
pub type UniformBuffer = GpuBuffer<u8>;

impl UniformBuffer {
    pub fn new_uniform<T: Pod>(device: &Device, data: &T, label: Option<&str>) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(data),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        
        Self {
            buffer: Arc::new(buffer),
            size: std::mem::size_of::<T>(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn update<T: Pod>(&self, queue: &Queue, data: &T) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(data));
    }
}

/// Storage buffer for compute shaders
pub type StorageBuffer<T> = GpuBuffer<T>;

/// Buffer manager
pub struct BufferManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    buffers: std::collections::HashMap<String, Arc<Buffer>>,
    uniform_buffers: std::collections::HashMap<String, UniformBuffer>,
}

impl BufferManager {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            device,
            queue,
            buffers: std::collections::HashMap::new(),
            uniform_buffers: std::collections::HashMap::new(),
        }
    }

    pub fn create_uniform<T: Pod>(&mut self, name: &str, data: &T) -> UniformBuffer {
        let buffer = UniformBuffer::new_uniform(&self.device, data, Some(name));
        self.uniform_buffers.insert(name.to_string(), buffer.clone());
        buffer
    }

    pub fn get_uniform(&self, name: &str) -> Option<&UniformBuffer> {
        self.uniform_buffers.get(name)
    }

    pub fn update_uniform<T: Pod>(&self, name: &str, data: &T) -> bool {
        if let Some(buf) = self.uniform_buffers.get(name) {
            buf.update(&self.queue, data);
            true
        } else { false }
    }

    pub fn create_storage<T: Pod>(&mut self, name: &str, count: usize) -> StorageBuffer<T> {
        let buffer = StorageBuffer::new(&self.device, count, BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC, Some(name));
        self.buffers.insert(name.to_string(), buffer.buffer.clone());
        buffer
    }

    pub fn create_vertex<T: Pod>(&mut self, name: &str, data: &[T]) -> StorageBuffer<T> {
        let buffer = StorageBuffer::from_data(&self.device, data, BufferUsages::VERTEX | BufferUsages::COPY_DST, Some(name));
        self.buffers.insert(name.to_string(), buffer.buffer.clone());
        buffer
    }

    pub fn create_index(&mut self, name: &str, data: &[u32]) -> StorageBuffer<u32> {
        let buffer = StorageBuffer::from_data(&self.device, data, BufferUsages::INDEX | BufferUsages::COPY_DST, Some(name));
        self.buffers.insert(name.to_string(), buffer.buffer.clone());
        buffer
    }

    pub fn create_indirect(&mut self, name: &str, count: usize) -> StorageBuffer<u32> {
        let buffer = StorageBuffer::new(&self.device, count, BufferUsages::INDIRECT | BufferUsages::COPY_DST, Some(name));
        self.buffers.insert(name.to_string(), buffer.buffer.clone());
        buffer
    }

    pub fn get_buffer(&self, name: &str) -> Option<&Arc<Buffer>> {
        self.buffers.get(name)
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
        self.uniform_buffers.clear();
    }
}

/// Frame data for rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FrameUniforms {
    pub projection: [f32; 16],
    pub view: [f32; 16],
    pub resolution: [f32; 2],
    pub time: f32,
    pub frame: u32,
    pub frame_rate: f32,
    pub _padding: [f32; 2],
}

/// Instance data for instanced rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct InstanceData {
    pub transform: [f32; 16],
    pub color: [f32; 4],
    pub uv_rect: [f32; 4],
    pub texture_index: u32,
    pub blend_mode: u32,
    pub _padding: [f32; 2],
}

/// Push constants for transitions
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TransitionPushConstants {
    pub progress: f32,
    pub params: [f32; 15],
}

/// Color grade uniforms
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ColorGradeUniforms {
    pub lift: [f32; 4],
    pub gamma: [f32; 4],
    pub gain: [f32; 4],
    pub offset: [f32; 4],
    pub contrast: f32,
    pub pivot: f32,
    pub saturation: f32,
    pub temperature: f32,
    pub tint: f32,
    pub shadows: [f32; 4],
    pub midtones: [f32; 4],
    pub highlights: [f32; 4],
    pub _padding: [f32; 3],
}

/// Scopes uniforms
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ScopesUniforms {
    pub resolution: [f32; 2],
    pub mode: u32, // 0=waveform, 1=parade, 2=vectorscope, 3=histogram
    pub color_space: u32,
    pub gain: f32,
    pub _padding: [f32; 3],
}