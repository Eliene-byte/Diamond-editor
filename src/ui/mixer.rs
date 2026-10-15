//! Audio mixer widget
use crate::core::math::{Point2, Rect};

use crate::core::id::{AudioBusId, TrackId};
use crate::timeline::clip::Timeline;
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::ui::widgets::*;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AudioMixerWidget {
    timeline: Option<Arc<RwLock<Timeline>>>,
    channels: Vec<MixerChannel>,
    master_channel: MixerChannel,
    bus_channels: Vec<MixerChannel>,
    rect: Rect,
}

#[derive(Debug, Clone)]
struct MixerChannel {
    id: TrackId,
    name: String,
    volume: f32,
    pan: f32,
    mute: bool,
    solo: bool,
    input: String,
    output: String,
    eq_enabled: bool,
    dynamics_enabled: bool,
    meter_left: f32,
    meter_right: f32,
    peak_left: f32,
    peak_right: f32,
}

impl AudioMixerWidget {
    pub fn new() -> Self {
        Self {
            timeline: None,
            channels: Vec::new(),
            master_channel: MixerChannel {
                id: TrackId::nil(),
                name: "MASTER".into(),
                volume: 1.0,
                pan: 0.0,
                mute: false,
                solo: false,
                input: "Bus 1".into(),
                output: "Out 1-2".into(),
                eq_enabled: false,
                dynamics_enabled: false,
                meter_left: 0.0,
                meter_right: 0.0,
                peak_left: 0.0,
                peak_right: 0.0,
            },
            bus_channels: Vec::new(),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    pub fn set_timeline(&mut self, timeline: Arc<RwLock<Timeline>>) {
        self.timeline = Some(timeline.clone());
        self.rebuild_channels();
    }

    fn rebuild_channels(&mut self) {
        self.channels.clear();
        if let Some(timeline) = &self.timeline {
            if let Ok(tl) = timeline.read() {
                for track in &tl.tracks.audio_tracks {
                    self.channels.push(MixerChannel {
                        id: track.id,
                        name: track.name.clone(),
                        volume: track.volume,
                        pan: track.pan,
                        mute: track.muted,
                        solo: track.solo,
                        input: "In 1-2".into(),
                        output: "Bus 1".into(),
                        eq_enabled: false,
                        dynamics_enabled: false,
                        meter_left: 0.0,
                        meter_right: 0.0,
                        peak_left: 0.0,
                        peak_right: 0.0,
                    });
                }
            }
        }
    }
}

impl AudioMixerWidget {
    fn draw_channel(
        &self,
        ctx: &DrawContext,
        channel: &MixerChannel,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        let channel_rect = Rect::new(x, y, w, h);

        // Channel strip background
        let bg = if channel.solo {
            [1.0, 0.5, 0.0, 0.2]
        } else if channel.mute {
            [0.5, 0.5, 0.5, 0.5]
        } else {
            ctx.theme.bg_secondary
        };
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: bg,
                rect: [
                    channel_rect.x,
                    channel_rect.y,
                    channel_rect.w,
                    channel_rect.h,
                ],
            });

        // Meters
        let meter_w = 12.0;
        let meter_h = h - 100.0;
        let meter_x = x + (w - meter_w * 2.0 - 4.0) * 0.5;
        let meter_y = y + 10.0;

        // Left meter
        self.draw_meter(
            ctx,
            meter_x,
            meter_y,
            meter_w,
            meter_h,
            channel.meter_left,
            channel.peak_left,
        );
        // Right meter
        self.draw_meter(
            ctx,
            meter_x + meter_w + 4.0,
            meter_y,
            meter_w,
            meter_h,
            channel.meter_right,
            channel.peak_right,
        );

        // Volume fader
        let fader_x = x + w * 0.5 - 10.0;
        let fader_y = y + h - 80.0;
        self.draw_fader(
            ctx,
            fader_x,
            fader_y,
            20.0,
            60.0,
            channel.volume,
            ctx.theme.accent,
        );

        // Pan knob
        let pan_x = x + w * 0.5;
        let pan_y = y + h - 30.0;
        self.draw_knob(
            ctx,
            pan_x,
            pan_y,
            15.0,
            channel.pan,
            ctx.theme.text_secondary,
        );

        // Mute/Solo buttons
        let btn_y = y + h - 50.0;
        let mute_color = if channel.mute {
            ctx.theme.error
        } else {
            ctx.theme.text_disabled
        };
        let solo_color = if channel.solo {
            ctx.theme.accent
        } else {
            ctx.theme.text_disabled
        };

        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: mute_color,
                rect: [x + 5.0, btn_y, 20.0, 20.0],
            });
        ctx.text_renderer.draw_text(
            "M",
            Point2::new(x + 10.0, btn_y + 15.0),
            ctx.theme.font_size_xs,
            ctx.theme.text_primary,
        );

        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: solo_color,
                rect: [x + 30.0, btn_y, 20.0, 20.0],
            });
        ctx.text_renderer.draw_text(
            "S",
            Point2::new(x + 35.0, btn_y + 15.0),
            ctx.theme.font_size_xs,
            ctx.theme.text_primary,
        );

        // Channel name
        ctx.text_renderer.draw_text(
            &channel.name,
            Point2::new(x + w * 0.5, y + h - 10.0),
            ctx.theme.font_size_xs,
            ctx.theme.text_primary,
        );
    }

    fn draw_meter(&self, ctx: &DrawContext, x: f32, y: f32, w: f32, h: f32, level: f32, peak: f32) {
        // Background
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: [0.0, 0.0, 0.0, 0.8],
                rect: [x, y, w, h],
            });

        // Level
        let level_h = h * level.clamp(0.0, 1.0);
        let color = if level > 0.9 {
            ctx.theme.error
        } else if level > 0.7 {
            ctx.theme.warning
        } else {
            ctx.theme.success
        };
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color,
                rect: [x, y + h - level_h, w, level_h],
            });

        // Peak hold
        let peak_y = y + h - h * peak.clamp(0.0, 1.0);
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.text_primary,
                rect: [x, peak_y - 1.0, w, 2.0],
            });
    }

    fn draw_fader(
        &self,
        ctx: &DrawContext,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        value: f32,
        color: [f32; 4],
    ) {
        // Track
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: [0.2, 0.2, 0.2, 1.0],
                rect: [x, y, w, h],
            });

        // Fill
        let fill_h = h * value.clamp(0.0, 1.0);
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color,
                rect: [x, y + h - fill_h, w, fill_h],
            });

        // Handle
        let handle_y = y + h - fill_h - 5.0;
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: [1.0, 1.0, 1.0, 1.0],
                rect: [x - 5.0, handle_y, w + 10.0, 10.0],
            });
    }

    fn draw_knob(&self, ctx: &DrawContext, x: f32, y: f32, r: f32, value: f32, color: [f32; 4]) {
        // Knob background
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: [0.2, 0.2, 0.2, 1.0],
                rect: [x - r, y - r, r * 2.0, r * 2.0],
            });

        // Indicator line
        let angle = (value * 270.0 - 135.0) * std::f32::consts::PI / 180.0;
        let ix = x + (r - 3.0) * angle.cos();
        let iy = y + (r - 3.0) * angle.sin();
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color,
                rect: [ix - 1.0, iy - 1.0, 2.0, 2.0],
            });
    }

    fn draw_meter_bridge(&self, ctx: &DrawContext, rect: Rect) {
        let bridge_h = 40.0;
        let bridge_y = rect.y + rect.h - bridge_h - 10.0;

        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_tertiary,
                rect: [rect.x, bridge_y, rect.w, bridge_h],
            });

        // Master meters
        let meter_w = 200.0;
        let meter_h = 24.0;
        let meter_x = rect.x + rect.w - meter_w - 20.0;
        let meter_y = bridge_y + (bridge_h - meter_h) * 0.5;

        self.draw_meter(
            ctx,
            meter_x,
            meter_y,
            meter_w / 2.0,
            meter_h,
            self.master_channel.meter_left,
            self.master_channel.peak_left,
        );
        self.draw_meter(
            ctx,
            meter_x + meter_w / 2.0,
            meter_y,
            meter_w / 2.0,
            meter_h,
            self.master_channel.meter_right,
            self.master_channel.peak_right,
        );

        ctx.text_renderer.draw_text(
            "MASTER",
            Point2::new(meter_x - 60.0, meter_y + 16.0),
            ctx.theme.font_size_sm,
            ctx.theme.text_primary,
        );
    }
}

impl Widget for AudioMixerWidget {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        let channel_width = 80.0;
        let width = (self.channels.len() + 1) as f32 * channel_width + 100.0; // + master
        (width, 600.0)
    }

    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) {
        self.rect = rect;
    }

    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        // Background
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_primary,
                rect: [rect.x, rect.y, rect.w, rect.h],
            });

        let channel_width = 80.0;
        let channel_height = rect.h - 100.0;
        let start_x = rect.x + 10.0;
        let start_y = rect.y + 10.0;

        // Draw audio channels
        for (i, channel) in self.channels.iter().enumerate() {
            self.draw_channel(
                ctx,
                channel,
                start_x + i as f32 * channel_width,
                start_y,
                channel_width,
                channel_height,
            );
        }

        // Master channel
        let master_x = start_x + self.channels.len() as f32 * channel_width + 10.0;
        self.draw_channel(
            ctx,
            &self.master_channel,
            master_x,
            start_y,
            100.0,
            channel_height,
        );

        // Meter bridge at top
        self.draw_meter_bridge(ctx, rect);
    }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        match event {
            InputEvent::MouseDown(pos, _) => {
                // Check fader clicks
                for (i, channel) in self.channels.iter().enumerate() {
                    let channel_width = 80.0;
                    let x = self.rect.x + 10.0 + i as f32 * channel_width;
                    let fader_x = x + channel_width * 0.5 - 10.0;
                    let fader_y = self.rect.y + self.rect.h - 80.0;

                    if pos.x >= fader_x
                        && pos.x <= fader_x + 20.0
                        && pos.y >= fader_y
                        && pos.y <= fader_y + 60.0
                    {
                        // Start fader drag
                        return true;
                    }
                }
                false
            }
            InputEvent::MouseMove(pos) => {
                // Fader drag
                false
            }
            _ => false,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
