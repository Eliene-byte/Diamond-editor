//! Thumbnail generation for media files

use crate::core::error::{MediaError, Result};
use crate::core::id::{ClipId, ThumbnailId};
use crate::core::time::Frame;
use crate::core::types::{PixelFormat, Resolution, Thumbnail};
use ffmpeg_next as ffmpeg;
use image::{DynamicImage, ImageBuffer, Rgba};
use std::path::Path;
use std::sync::Arc;

/// Thumbnail generator
pub struct ThumbnailGenerator {
    size: (u32, u32),
    format: ThumbnailFormat,
    quality: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailFormat {
    Jpeg,
    Png,
    WebP,
}

impl Default for ThumbnailGenerator {
    fn default() -> Self {
        Self {
            size: (256, 144), // 16:9
            format: ThumbnailFormat::Jpeg,
            quality: 80,
        }
    }
}

impl ThumbnailGenerator {
    pub fn new(size: (u32, u32), format: ThumbnailFormat, quality: u8) -> Self {
        Self {
            size,
            format,
            quality,
        }
    }

    pub fn generate(&self, path: &Path, frame: Frame) -> Result<Thumbnail> {
        ffmpeg::init()?;

        let mut ictx =
            ffmpeg::format::input(&path).map_err(|e| MediaError::Decoder(e.to_string()))?;

        let video_stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(MediaError::NoVideoStream)?;

        let mut decoder = video_stream
            .codec()
            .decoder()
            .video()
            .map_err(|e| MediaError::Decoder(e.to_string()))?;

        // Seek to frame
        let time_base = video_stream.time_base();
        let pts = (frame.0 as i64 * time_base.den) / time_base.num as i64;
        ictx.seek(pts, ..pts).ok();
        decoder.flush();

        // Read frames until we reach target
        let mut frame_data = ffmpeg::frame::Video::empty();
        let mut packet = ffmpeg::Packet::empty();
        let mut current_frame = Frame::ZERO;

        while ictx.read_packet(&mut packet).is_ok() {
            if packet.stream() != video_stream.index() {
                continue;
            }

            decoder.send_packet(&packet).ok();
            while decoder.receive_frame(&mut frame_data).is_ok() {
                if current_frame == frame {
                    break;
                }
                current_frame.0 += 1;
            }
            if current_frame == frame {
                break;
            }
        }

        if current_frame != frame {
            return Err(MediaError::FrameNotAvailable(frame.0.to_string()).into());
        }

        // Convert to RGB
        let rgb_frame = self.convert_to_rgb(&frame_data)?;

        // Resize
        let thumbnail = self.resize(&rgb_frame)?;

        // Encode
        let data = self.encode(&thumbnail)?;

        Ok(Thumbnail {
            frame,
            data,
            width: self.size.0 as u16,
            height: self.size.1 as u16,
        })
    }

    pub fn generate_multiple(&self, path: &Path, frames: &[Frame]) -> Result<Vec<Thumbnail>> {
        let mut results = Vec::with_capacity(frames.len());

        ffmpeg::init()?;
        let mut ictx =
            ffmpeg::format::input(&path).map_err(|e| MediaError::Decoder(e.to_string()))?;

        let video_stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(MediaError::NoVideoStream)?;

        let mut decoder = video_stream
            .codec()
            .decoder()
            .video()
            .map_err(|e| MediaError::Decoder(e.to_string()))?;

        let time_base = video_stream.time_base();
        let mut frame_data = ffmpeg::frame::Video::empty();
        let mut packet = ffmpeg::Packet::empty();
        let mut current_frame = Frame::ZERO;
        let mut frame_idx = 0;

        while frame_idx < frames.len() && ictx.read_packet(&mut packet).is_ok() {
            if packet.stream() != video_stream.index() {
                continue;
            }

            decoder.send_packet(&packet).ok();
            while decoder.receive_frame(&mut frame_data).is_ok() {
                if current_frame == frames[frame_idx] {
                    let rgb = self.convert_to_rgb(&frame_data)?;
                    let thumb = self.resize(&rgb)?;
                    let data = self.encode(&thumb)?;

                    results.push(Thumbnail {
                        frame: current_frame,
                        data,
                        width: self.size.0 as u16,
                        height: self.size.1 as u16,
                    });

                    frame_idx += 1;
                    if frame_idx >= frames.len() {
                        break;
                    }
                }
                current_frame.0 += 1;
            }
        }

        Ok(results)
    }

    pub fn generate_sprite(
        &self,
        path: &Path,
        interval: Frame,
        max_frames: usize,
    ) -> Result<ThumbnailSprite> {
        ffmpeg::init()?;
        let mut ictx =
            ffmpeg::format::input(&path).map_err(|e| MediaError::Decoder(e.to_string()))?;

        let video_stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(MediaError::NoVideoStream)?;

        let duration = video_stream.duration().unwrap_or(0);
        let total_frames = (duration / interval.0).min(max_frames as i64) as usize;

        let mut frames = Vec::with_capacity(total_frames);
        for i in 0..total_frames {
            frames.push(Frame(i as i64 * interval.0));
        }

        let thumbnails = self.generate_multiple(path, &frames)?;

        // Create sprite sheet
        let cols = (total_frames as f32).sqrt().ceil() as u32;
        let rows = (total_frames as f32 / cols as f32).ceil() as u32;

        let sprite_width = cols * self.size.0;
        let sprite_height = rows * self.size.1;

        let mut sprite = ImageBuffer::new(sprite_width, sprite_height);

        for (i, thumb) in thumbnails.iter().enumerate() {
            let img = self.decode_image(&thumb.data)?;
            let col = (i % cols as usize) as u32;
            let row = (i / cols as usize) as u32;
            let x = col * self.size.0;
            let y = row * self.size.1;
            image::imageops::overlay(&mut sprite, &img, x, y);
        }

        let sprite_data = self.encode_image(&sprite)?;

        Ok(ThumbnailSprite {
            thumbnails,
            sprite_data,
            sprite_width: sprite_width as u16,
            sprite_height: sprite_height as u16,
            cols: cols as u16,
            rows: rows as u16,
            tile_width: self.size.0 as u16,
            tile_height: self.size.1 as u16,
        })
    }

    fn convert_to_rgb(&self, frame: &ffmpeg::frame::Video) -> Result<DynamicImage> {
        let width = frame.width();
        let height = frame.height();

        // Use swscale to convert to RGBA
        let mut converter = ffmpeg::software::scaling::Context::get(
            frame.format(),
            width,
            height,
            ffmpeg::format::Pixel::RGBA,
            width,
            height,
            ffmpeg::software::scaling::Flags::BILINEAR,
        )
        .map_err(|e| MediaError::Decoder(e.to_string()))?;

        let mut rgb_frame = ffmpeg::frame::Video::empty();
        rgb_frame.set_format(ffmpeg::format::Pixel::RGBA);
        rgb_frame.set_width(width);
        rgb_frame.set_height(height);
        rgb_frame.allocate();

        converter
            .run(frame, &mut rgb_frame)
            .map_err(|e| MediaError::Decoder(e.to_string()))?;

        let data = rgb_frame.data(0);
        let stride = rgb_frame.stride(0);

        let mut img = ImageBuffer::new(width, height);
        for y in 0..height {
            let src_off = (y * stride) as usize;
            let dst_off = (y * width * 4) as usize;
            img.as_mut_slice()[dst_off..dst_off + (width * 4) as usize]
                .copy_from_slice(&data[src_off..src_off + (width * 4) as usize]);
        }

        Ok(DynamicImage::ImageRgba8(img))
    }

    fn resize(&self, img: &DynamicImage) -> Result<DynamicImage> {
        let resized = image::imageops::resize(
            img,
            self.size.0,
            self.size.1,
            image::imageops::FilterType::Lanczos3,
        );
        Ok(resized)
    }

    fn encode(&self, img: &DynamicImage) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        match self.format {
            ThumbnailFormat::Jpeg => {
                let mut encoder =
                    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, self.quality);
                encoder
                    .encode_image(img)
                    .map_err(|e| MediaError::ThumbnailFailed(e.to_string()))?;
            }
            ThumbnailFormat::Png => {
                let encoder = image::codecs::png::PngEncoder::new(&mut buf);
                encoder
                    .write_image(img.as_bytes(), img.width(), img.height(), img.color())
                    .map_err(|e| MediaError::ThumbnailFailed(e.to_string()))?;
            }
            ThumbnailFormat::WebP => {
                let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
                encoder
                    .encode_image(img)
                    .map_err(|e| MediaError::ThumbnailFailed(e.to_string()))?;
            }
        }
        Ok(buf)
    }

    fn encode_image(&self, img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        match self.format {
            ThumbnailFormat::Jpeg => {
                let mut encoder =
                    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, self.quality);
                encoder
                    .encode_image(&DynamicImage::ImageRgba8(img.clone()))
                    .map_err(|e| MediaError::ThumbnailFailed(e.to_string()))?;
            }
            ThumbnailFormat::Png => {
                let encoder = image::codecs::png::PngEncoder::new(&mut buf);
                encoder
                    .write_image(
                        img.as_raw(),
                        img.width(),
                        img.height(),
                        image::ExtendedColorType::Rgba8,
                    )
                    .map_err(|e| MediaError::ThumbnailFailed(e.to_string()))?;
            }
            ThumbnailFormat::WebP => {
                let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
                encoder
                    .encode_image(&DynamicImage::ImageRgba8(img.clone()))
                    .map_err(|e| MediaError::ThumbnailFailed(e.to_string()))?;
            }
        }
        Ok(buf)
    }

    fn decode_image(&self, data: &[u8]) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let img = image::load_from_memory(data)
            .map_err(|e| MediaError::ThumbnailFailed(e.to_string()))?;
        Ok(img.to_rgba8())
    }
}

/// Thumbnail sprite sheet for fast scrubbing
#[derive(Debug, Clone)]
pub struct ThumbnailSprite {
    pub thumbnails: Vec<Thumbnail>,
    pub sprite_data: Vec<u8>,
    pub sprite_width: u16,
    pub sprite_height: u16,
    pub cols: u16,
    pub rows: u16,
    pub tile_width: u16,
    pub tile_height: u16,
}

impl ThumbnailSprite {
    pub fn get_tile_rect(&self, index: usize) -> Option<(u32, u32, u32, u32)> {
        if index >= self.thumbnails.len() {
            return None;
        }
        let col = (index % self.cols as usize) as u32;
        let row = (index / self.cols as usize) as u32;
        Some((
            col * self.tile_width as u32,
            row * self.tile_height as u32,
            self.tile_width as u32,
            self.tile_height as u32,
        ))
    }
}

/// Thumbnail cache for fast UI
pub struct ThumbnailCache {
    cache: std::collections::HashMap<(ClipId, Frame), Arc<Thumbnail>>,
    sprite_cache: std::collections::HashMap<ClipId, Arc<ThumbnailSprite>>,
    max_entries: usize,
    lru: std::collections::VecDeque<(ClipId, Frame)>,
}

impl ThumbnailCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: std::collections::HashMap::new(),
            sprite_cache: std::collections::HashMap::new(),
            max_entries,
            lru: std::collections::VecDeque::new(),
        }
    }

    pub fn get(&mut self, clip_id: ClipId, frame: Frame) -> Option<Arc<Thumbnail>> {
        let key = (clip_id, frame);
        if let Some(thumb) = self.cache.get(&key) {
            // Update LRU
            self.lru.retain(|k| k != &key);
            self.lru.push_back(key);
            Some(thumb.clone())
        } else {
            None
        }
    }

    pub fn put(&mut self, clip_id: ClipId, frame: Frame, thumb: Thumbnail) {
        let key = (clip_id, frame);

        if self.cache.len() >= self.max_entries {
            if let Some(old_key) = self.lru.pop_front() {
                self.cache.remove(&old_key);
            }
        }

        self.cache.insert(key, Arc::new(thumb));
        self.lru.push_back(key);
    }

    pub fn get_sprite(&self, clip_id: ClipId) -> Option<Arc<ThumbnailSprite>> {
        self.sprite_cache.get(&clip_id).cloned()
    }

    pub fn put_sprite(&mut self, clip_id: ClipId, sprite: ThumbnailSprite) {
        self.sprite_cache.insert(clip_id, Arc::new(sprite));
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.sprite_cache.clear();
        self.lru.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }
}
