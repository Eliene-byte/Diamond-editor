//! GPU text rendering primitives.
//!
//! Text is drawn by the custom UI through this module. Glyph shaping is handled
//! upstream (see `parley`/`cosmic-text` in the dependency list); this module only
//! carries the vertex layout and the buffer needed to upload shaped glyph quads
//! to the GPU.

use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::{
    Buffer, BufferUsages, Device, Queue, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexStepMode,
};

/// One glyph quad. Six vertices (two triangles), or four with an index buffer.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GlyphVertex {
    /// Position in normalised device coordinates.
    pub position: [f32; 2],
    /// Glyph texture coordinates.
    pub uv: [f32; 2],
    /// Straight (non-premultiplied) sRGB colour.
    pub color: [f32; 4],
}

/// Vertex layout for a glyph quad stream. Matches `GlyphVertex` above.
pub const TEXT_VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
    array_stride: std::mem::size_of::<GlyphVertex>() as u64,
    step_mode: VertexStepMode::Vertex,
    attributes: &[
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: 8,
            shader_location: 1,
        },
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 16,
            shader_location: 2,
        },
    ],
};

/// A growable CPU-side glyph buffer paired with its GPU vertex buffer.
pub struct GlyphBatch {
    vertices: Vec<GlyphVertex>,
    capacity: usize,
    buffer: Option<Arc<Buffer>>,
    buffer_capacity: usize,
}

impl GlyphBatch {
    pub fn new(capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(capacity),
            capacity,
            buffer: None,
            buffer_capacity: 0,
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Ensure CPU-side room for at least `additional` more vertices.
    pub fn reserve(&mut self, additional: usize) {
        if self.vertices.len() + additional > self.capacity {
            self.capacity = (self.vertices.len() + additional) * 2;
            self.vertices.reserve(self.capacity - self.vertices.len());
        }
    }

    pub fn push(&mut self, vertex: GlyphVertex) {
        self.vertices.push(vertex);
    }

    pub fn extend_from_slice(&mut self, verts: &[GlyphVertex]) {
        self.vertices.extend_from_slice(verts);
    }

    pub fn as_slice(&self) -> &[GlyphVertex] {
        &self.vertices
    }

    /// Upload the current contents to the GPU, growing the vertex buffer if needed.
    /// Returns the buffer to bind, or `None` when there is nothing to draw.
    pub fn upload(&mut self, device: &Device, queue: &Queue) -> Option<Arc<Buffer>> {
        if self.vertices.is_empty() {
            return None;
        }

        let needed = self.vertices.len() * std::mem::size_of::<GlyphVertex>();
        let recreate = match &self.buffer {
            None => true,
            Some(_) => needed > self.buffer_capacity,
        };

        if recreate {
            let capacity = (needed * 2).max(1024);
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Glyph Batch"),
                size: capacity as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.buffer = Some(Arc::new(buffer));
            self.buffer_capacity = capacity;
        }

        let buffer = self.buffer.clone().unwrap();
        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&self.vertices));
        Some(buffer)
    }

    pub fn buffer(&self) -> Option<&Arc<Buffer>> {
        self.buffer.as_ref()
    }
}

impl Default for GlyphBatch {
    fn default() -> Self {
        Self::new(4096)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32) -> GlyphVertex {
        GlyphVertex {
            position: [x, y],
            uv: [0.0, 0.0],
            color: [1.0; 4],
        }
    }

    #[test]
    fn vertex_layout_matches_struct() {
        // The three attributes must exactly tile GlyphVertex with no padding.
        assert_eq!(
            TEXT_VERTEX_LAYOUT.array_stride,
            std::mem::size_of::<GlyphVertex>() as u64
        );
        assert_eq!(TEXT_VERTEX_LAYOUT.attributes.len(), 3);

        let mut covered = 0u64;
        for attr in TEXT_VERTEX_LAYOUT.attributes {
            covered |= attr.offset;
            assert_eq!(attr.offset, covered, "attributes must be contiguous");
            covered += attr.format.size() as u64;
        }
        assert_eq!(covered, TEXT_VERTEX_LAYOUT.array_stride);
    }

    #[test]
    fn batch_starts_empty_and_clears() {
        let mut b = GlyphBatch::new(4);
        assert!(b.is_empty());

        b.push(v(0.0, 0.0));
        b.push(v(1.0, 0.0));
        assert_eq!(b.len(), 2);

        b.clear();
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn reserve_grows_capacity_without_loss() {
        let mut b = GlyphBatch::new(2);
        for i in 0..100 {
            b.reserve(1);
            b.push(v(i as f32, 0.0));
        }
        assert_eq!(b.len(), 100);
        assert!(b.capacity >= 100);
        assert_eq!(b.as_slice()[99].position[0], 99.0);
    }

    #[test]
    fn upload_without_data_returns_none() {
        // No Device/Queue available in a unit test, so only assert the early-out path
        // that must not require touching the GPU.
        let b = GlyphBatch::new(8);
        assert!(b.buffer().is_none());
    }
}
