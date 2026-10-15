//! GPU-accelerated playback engine

use crate::core::color::PixelFormat;
use crate::core::time::Frame;
use crate::gpu::color::ColorPipeline;
use crate::gpu::renderer::{CompositeLayer, GpuRenderer, RenderCommand};
use crate::gpu::texture::{GpuTexture, TextureManager};
use crate::media::decoder::{VideoDecoder, VideoFrame};
use crate::timeline::clip::Timeline;
use crate::timeline::track::TrackItem;
use crossbeam::channel::{unbounded, Receiver, Sender};
use parking_lot::RwLock;
use std::sync::Arc;

/// Frame ready for display
#[derive(Clone)]
pub struct DisplayFrame {
    pub texture: GpuTexture,
    pub frame: Frame,
    pub timecode: crate::core::time::Timecode,
    pub metadata: FrameMetadata,
}

#[derive(Debug, Clone)]
pub struct FrameMetadata {
    pub color_space: crate::core::color::ColorSpace,
    pub pixel_format: PixelFormat,
    pub has_alpha: bool,
    pub hdr: bool,
}

/// Playback engine - handles frame decoding, caching, and rendering
pub struct PlaybackEngine {
    renderer: Arc<RwLock<GpuRenderer>>,
    texture_manager: RwLock<TextureManager>,
    color_pipeline: Arc<RwLock<ColorPipeline>>,

    // Frame cache
    frame_cache: FrameCache,

    // Decoders per clip
    decoders: std::collections::HashMap<crate::core::id::ClipId, VideoDecoder>,

    // Current timeline
    timeline: Option<Arc<RwLock<Timeline>>>,

    // Playback state
    playhead: Frame,
    playback_state: crate::core::types::PlaybackState,
    loop_range: Option<(Frame, Frame)>,
    speed: f32,

    // Frame delivery
    frame_tx: Sender<DisplayFrame>,
    frame_rx: Receiver<DisplayFrame>,

    // Background decode
    decode_tx: Sender<DecodeRequest>,
    decode_rx: Receiver<DecodeRequest>,
}

#[derive(Debug)]
struct DecodeRequest {
    clip_id: crate::core::id::ClipId,
    frame: Frame,
    respond_to: Sender<DisplayFrame>,
}

struct FrameCache {
    frames: std::collections::HashMap<Frame, DisplayFrame>,
    max_size: usize,
    lru: std::collections::VecDeque<Frame>,
}

impl FrameCache {
    fn new(max_size: usize) -> Self {
        Self {
            frames: std::collections::HashMap::new(),
            max_size,
            lru: std::collections::VecDeque::new(),
        }
    }

    fn get(&mut self, frame: Frame) -> Option<DisplayFrame> {
        if let Some(display_frame) = self.frames.get(&frame) {
            // Update LRU
            self.lru.retain(|f| *f != frame);
            self.lru.push_back(frame);
            Some(display_frame.clone())
        } else {
            None
        }
    }

    fn put(&mut self, frame: Frame, display_frame: DisplayFrame) {
        if self.frames.len() >= self.max_size {
            if let Some(oldest) = self.lru.pop_front() {
                self.frames.remove(&oldest);
            }
        }
        self.frames.insert(frame, display_frame);
        self.lru.push_back(frame);
    }
}

impl PlaybackEngine {
    pub fn new(renderer: Arc<RwLock<GpuRenderer>>) -> Self {
        let (device, queue) = {
            let r = renderer.read();
            (r.device().clone(), r.queue().clone())
        };
        let texture_manager = RwLock::new(TextureManager::new(device.clone(), queue.clone()));
        let color_pipeline = Arc::new(RwLock::new(ColorPipeline::new(device, queue)));

        let (frame_tx, frame_rx) = unbounded();
        let (decode_tx, decode_rx) = unbounded();

        Self {
            renderer,
            texture_manager,
            color_pipeline,
            frame_cache: FrameCache::new(60),
            decoders: std::collections::HashMap::new(),
            timeline: None,
            playhead: Frame::ZERO,
            playback_state: crate::core::types::PlaybackState::Stopped,
            loop_range: None,
            speed: 1.0,
            frame_tx,
            frame_rx,
            decode_tx,
            decode_rx,
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline);
    }

    pub fn add_clip_decoder(&mut self, clip_id: crate::core::id::ClipId, decoder: VideoDecoder) {
        self.decoders.insert(clip_id, decoder);
    }

    pub fn request_frame(&self, frame: Frame) {
        self.decode_tx
            .send(DecodeRequest {
                clip_id: crate::core::id::ClipId::nil(), // Would determine from timeline
                frame,
                respond_to: self.frame_tx.clone(), // Simplified
            })
            .ok();
    }

    pub fn next_frame(&mut self) -> Option<DisplayFrame> {
        self.frame_rx.try_recv().ok()
    }

    pub fn render_timeline_frame(
        &mut self,
        timeline: &Timeline,
        frame: Frame,
    ) -> Result<DisplayFrame, Box<dyn std::error::Error>> {
        // Check cache
        if let Some(cached) = self.frame_cache.get(frame) {
            return Ok(cached);
        }

        // Find visible items at this frame
        let items = timeline.items_at_frame(frame);
        if items.is_empty() {
            // Return blank frame
            let blank = self
                .texture_manager
                .write()
                .create_render_target(1920, 1080);
            return Ok(DisplayFrame {
                texture: blank,
                frame,
                timecode: crate::core::time::Timecode::from_frame(
                    frame,
                    timeline.settings.frame_rate,
                ),
                metadata: FrameMetadata {
                    color_space: crate::core::color::ColorSpace::Rec709,
                    pixel_format: PixelFormat::Rgba16F,
                    has_alpha: false,
                    hdr: false,
                },
            });
        }

        // Composite layers
        let mut layers = Vec::new();
        for item in items {
            if !item.enabled {
                continue;
            }

            // Decode frame from source
            let source_frame = item.frame_at_timeline(frame).unwrap_or(Frame::ZERO);
            if let Some(decoder) = self.decoders.get_mut(&item.clip_id.unwrap()) {
                if let Ok(Some(video_frame)) = decoder.decode_frame(source_frame) {
                    let texture = self.upload_video_frame(&video_frame)?;

                    layers.push(CompositeLayer {
                        texture,
                        rect: [0.0, 0.0, 1.0, 1.0],
                        opacity: item.opacity,
                        blend_mode: match item.blend_mode {
                            crate::timeline::track::BlendMode::Normal => {
                                crate::gpu::renderer::BlendMode::Normal
                            }
                            crate::timeline::track::BlendMode::Add => {
                                crate::gpu::renderer::BlendMode::Add
                            }
                            crate::timeline::track::BlendMode::Screen => {
                                crate::gpu::renderer::BlendMode::Screen
                            }
                            crate::timeline::track::BlendMode::Multiply => {
                                crate::gpu::renderer::BlendMode::Multiply
                            }
                            crate::timeline::track::BlendMode::Overlay => {
                                crate::gpu::renderer::BlendMode::Overlay
                            }
                            _ => crate::gpu::renderer::BlendMode::Normal,
                        },
                        transform: [
                            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
                            0.0, 1.0,
                        ],
                    });
                }
            }
        }

        // Render composite
        let output = self
            .texture_manager
            .write()
            .create_render_target(1920, 1080);

        // Submit composite command
        self.renderer.write().submit(RenderCommand::Composite {
            layers,
            output: output.clone(),
        });

        let display_frame = DisplayFrame {
            texture: output,
            frame,
            timecode: crate::core::time::Timecode::from_frame(frame, timeline.settings.frame_rate),
            metadata: FrameMetadata {
                color_space: timeline.settings.color_space,
                pixel_format: timeline.settings.pixel_format,
                has_alpha: false,
                hdr: false,
            },
        };

        self.frame_cache.put(frame, display_frame.clone());
        Ok(display_frame)
    }

    fn upload_video_frame(
        &self,
        frame: &VideoFrame,
    ) -> Result<GpuTexture, Box<dyn std::error::Error>> {
        // Upload frame data to GPU
        let texture = self.texture_manager.write().upload(
            &frame.data.plane(0),
            frame.width,
            frame.height,
            frame.format,
        );
        Ok(texture)
    }

    pub fn play(&mut self) {
        self.playback_state = crate::core::types::PlaybackState::Playing;
    }
    pub fn pause(&mut self) {
        self.playback_state = crate::core::types::PlaybackState::Paused;
    }
    pub fn stop(&mut self) {
        self.playback_state = crate::core::types::PlaybackState::Stopped;
        self.playhead = Frame::ZERO;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(-16.0, 16.0);
    }
    pub fn set_loop(&mut self, start: Frame, end: Frame) {
        self.loop_range = Some((start, end));
    }
    pub fn clear_loop(&mut self) {
        self.loop_range = None;
    }
}
