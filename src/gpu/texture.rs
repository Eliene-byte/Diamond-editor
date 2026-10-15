//! GPU texture management

use wgpu::{Device, Queue, Texture, TextureView, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, Extent3d, FilterMode, AddressMode, SamplerDescriptor};
use std::sync::Arc;
use crate::core::color::PixelFormat;

/// A GPU texture handle. Cloning is cheap: the underlying resource is shared
/// through `Arc`, and wgpu keeps it alive until the last clone drops.
#[derive(Debug, Clone)]
pub struct GpuTexture {
    pub texture: Arc<Texture>,
    pub view: Arc<TextureView>,
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_levels: u32,
    pub usage: TextureUsages,
}

impl GpuTexture {
    pub fn new(
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
        usage: TextureUsages,
        label: Option<&str>,
    ) -> Self {
        let desc = TextureDescriptor {
            label,
            size: Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        };
        
        let texture = Arc::new(device.create_texture(&desc));
        let view = Arc::new(texture.create_view(&Default::default()));
        
        Self {
            texture,
            view,
            format,
            width,
            height,
            depth: 1,
            mip_levels: 1,
            usage,
        }
    }

    pub fn create_view(&self) -> Arc<TextureView> {
        Arc::new(self.texture.create_view(&Default::default()))
    }

    pub fn create_mip_view(&self, level: u32) -> Arc<TextureView> {
        Arc::new(self.texture.create_view(&wgpu::TextureViewDescriptor {
            base_mip_level: level,
            mip_level_count: Some(1),
            ..Default::default()
        }))
    }

    pub fn size(&self) -> (u32, u32) { (self.width, self.height) }
    pub fn aspect_ratio(&self) -> f32 { self.width as f32 / self.height as f32 }
}

/// Texture manager
pub struct TextureManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    textures: std::collections::HashMap<String, GpuTexture>,
    samplers: std::collections::HashMap<SamplerKey, Arc<wgpu::Sampler>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SamplerKey {
    pub filter: FilterMode,
    pub address_u: AddressMode,
    pub address_v: AddressMode,
    pub address_w: AddressMode,
    pub compare: Option<wgpu::CompareFunction>,
}

impl Default for SamplerKey {
    fn default() -> Self {
        Self {
            filter: FilterMode::Linear,
            address_u: AddressMode::ClampToEdge,
            address_v: AddressMode::ClampToEdge,
            address_w: AddressMode::ClampToEdge,
            compare: None,
        }
    }
}

impl TextureManager {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            device,
            queue,
            textures: std::collections::HashMap::new(),
            samplers: std::collections::HashMap::new(),
        }
    }

    pub fn create_render_target(&mut self, width: u32, height: u32) -> GpuTexture {
        let key = format!("rt_{}x{}", width, height);
        
        if let Some(tex) = self.textures.get(&key) {
            if tex.width == width && tex.height == height {
                return tex.clone();
            }
        }
        
        let texture = GpuTexture::new(
            &self.device,
            width,
            height,
            wgpu::TextureFormat::Rgba16Float,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC,
            Some(&key),
        );
        
        self.textures.insert(key, texture.clone());
        texture
    }

    pub fn create_storage_texture(&mut self, width: u32, height: u32, format: TextureFormat) -> GpuTexture {
        let key = format!("storage_{}x{}_{:?}", width, height, format);
        
        let texture = GpuTexture::new(
            &self.device,
            width,
            height,
            format,
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC,
            Some(&key),
        );
        
        self.textures.insert(key, texture.clone());
        texture
    }

    pub fn upload(&mut self, data: &[u8], width: u32, height: u32, format: PixelFormat) -> GpuTexture {
        let gpu_format = self.pixel_format_to_gpu(format);
        let key = format!("upload_{}x{}_{:?}", width, height, format);
        
        let texture = GpuTexture::new(
            &self.device,
            width,
            height,
            gpu_format,
            TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            Some(&key),
        );
        
        // Upload data
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.bytes_per_row(width, gpu_format)),
                rows_per_image: Some(height),
            },
            Extent3d { width, height, depth_or_array_layers: 1 },
        );
        
        self.textures.insert(key, texture.clone());
        texture
    }

    pub fn create_from_pixels(&mut self, pixels: &[u8], width: u32, height: u32, format: PixelFormat) -> GpuTexture {
        self.upload(pixels, width, height, format)
    }

    pub fn get_sampler(&mut self, key: SamplerKey) -> Arc<wgpu::Sampler> {
        self.samplers.entry(key.clone()).or_insert_with(|| {
            Arc::new(self.device.create_sampler(&SamplerDescriptor {
                label: Some(&format!("Sampler {:?}", key)),
                address_mode_u: key.address_u,
                address_mode_v: key.address_v,
                address_mode_w: key.address_w,
                mag_filter: key.filter,
                min_filter: key.filter,
                mipmap_filter: key.filter,
                compare: key.compare,
                ..Default::default()
            }))
        }).clone()
    }

    pub fn get_or_create(&mut self, name: &str, creator: impl FnOnce() -> GpuTexture) -> &GpuTexture {
        self.textures.entry(name.to_string()).or_insert_with(creator)
    }

    pub fn remove(&mut self, name: &str) -> Option<GpuTexture> {
        self.textures.remove(name)
    }

    pub fn clear(&mut self) {
        self.textures.clear();
    }

    fn pixel_format_to_gpu(&self, format: PixelFormat) -> TextureFormat {
        match format {
            PixelFormat::Rgb8 => TextureFormat::Rgba8UnormSrgb,
            PixelFormat::Rgba8 => TextureFormat::Rgba8UnormSrgb,
            PixelFormat::Rgb10 => TextureFormat::Rgb10a2Unorm,
            PixelFormat::Rgba10 => TextureFormat::Rgb10a2Unorm,
            PixelFormat::Rgba16F => TextureFormat::Rgba16Float,
            PixelFormat::Rgb16F => TextureFormat::Rgba16Float,
            PixelFormat::Rgba32F => TextureFormat::Rgba32Float,
            PixelFormat::Yuv420_8 => TextureFormat::R8Unorm, // Planar - would need 3 textures
            PixelFormat::Yuv422_8 => TextureFormat::Rg8Unorm,
            PixelFormat::Yuv444_8 => TextureFormat::Rgba8Unorm,
            PixelFormat::Yuv420_10 => TextureFormat::R16Unorm,
            PixelFormat::Yuv422_10 => TextureFormat::Rg16Unorm,
            PixelFormat::Yuv444_10 => TextureFormat::Rgba16Unorm,
            PixelFormat::P010 => TextureFormat::Rg16Unorm,
            _ => TextureFormat::Rgba8UnormSrgb,
        }
    }

    /// 3D LUT texture (used by the color pipeline).
    pub fn create_3d_texture(&mut self, width: u32, height: u32, depth: u32, data: Vec<u8>) -> GpuTexture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("LUT 3D"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: depth },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &data,
            wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
            wgpu::Extent3d { width, height, depth_or_array_layers: depth },
        );
        let view = texture.create_view(&Default::default());
        GpuTexture {
            texture: Arc::new(texture),
            view: Arc::new(view),
            format: wgpu::TextureFormat::Rgba8Unorm,
            width, height, depth, mip_levels: 1,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
        }
    }

    /// 1D LUT texture (used by the color pipeline).
    pub fn create_1d_texture(&mut self, width: u32, data: Vec<u8>) -> GpuTexture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("LUT 1D"),
            size: wgpu::Extent3d { width, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D1,
            format: wgpu::TextureFormat::Rgba16Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &data,
            wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(width * 6), rows_per_image: None },
            wgpu::Extent3d { width, height: 1, depth_or_array_layers: 1 },
        );
        let view = texture.create_view(&Default::default());
        GpuTexture {
            texture: Arc::new(texture),
            view: Arc::new(view),
            format: wgpu::TextureFormat::Rgba16Unorm,
            width, height: 1, depth: 1, mip_levels: 1,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
        }
    }

    fn bytes_per_row(&self, width: u32, format: TextureFormat) -> u32 {
        // `block_copy_size` takes an aspect argument and supersedes the deprecated
        // `block_size()`.
        let block_size = format.block_copy_size(None).unwrap_or(4);
        ((width + 3) / 4) * 4 * block_size // Align to 256 bytes
    }
}

/// Texture pool for reuse
pub struct TexturePool {
    available: std::collections::VecDeque<GpuTexture>,
    in_use: std::collections::HashMap<u64, GpuTexture>,
    device: Arc<Device>,
    format: TextureFormat,
    width: u32,
    height: u32,
    usage: TextureUsages,
    next_id: u64,
}

impl TexturePool {
    pub fn new(
        device: Arc<Device>,
        width: u32,
        height: u32,
        format: TextureFormat,
        usage: TextureUsages,
        initial_count: usize,
    ) -> Self {
        let mut pool = Self {
            available: std::collections::VecDeque::new(),
            in_use: std::collections::HashMap::new(),
            device,
            format,
            width,
            height,
            usage,
            next_id: 0,
        };
        
        for _ in 0..initial_count {
            pool.available.push_back(GpuTexture::new(
                &pool.device,
                width,
                height,
                format,
                usage,
                Some(&format!("pool_{}", pool.next_id)),
            ));
            pool.next_id += 1;
        }
        
        pool
    }

    pub fn acquire(&mut self) -> GpuTexture {
        if let Some(tex) = self.available.pop_front() {
            let id = self.next_id;
            self.next_id += 1;
            self.in_use.insert(id, tex.clone());
            tex
        } else {
            // Pool exhausted - create new
            let tex = GpuTexture::new(
                &self.device,
                self.width,
                self.height,
                self.format,
                self.usage,
                Some(&format!("pool_extra_{}", self.next_id)),
            );
            let id = self.next_id;
            self.next_id += 1;
            self.in_use.insert(id, tex.clone());
            tex
        }
    }

    pub fn release(&mut self, texture: GpuTexture) {
        // Find and remove from in_use
        if let Some((id, _)) = self.in_use.iter().find(|(_, t)| t.texture.as_ref() == texture.texture.as_ref()).map(|(k, v)| (*k, v.clone())) {
            self.in_use.remove(&id);
            self.available.push_back(texture);
        }
    }

    pub fn available_count(&self) -> usize { self.available.len() }
    pub fn in_use_count(&self) -> usize { self.in_use.len() }
}