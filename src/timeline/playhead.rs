//! Playhead and playback control

use crate::core::time::{Duration, Frame, FrameRate, Timecode};
use crate::core::types::PlaybackState;
use crossbeam::channel::{unbounded, Receiver, Sender};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;

/// Playback event
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    PositionChanged { frame: Frame, timecode: Timecode },
    StateChanged { state: PlaybackState },
    LoopPointReached { loop_start: Frame, loop_end: Frame },
    EndReached,
    Error { message: String },
}

/// Playhead - controls timeline playback
pub struct Playhead {
    position: Arc<AtomicI64>,
    state: Arc<RwLock<PlaybackState>>,
    frame_rate: FrameRate,
    duration: RwLock<Frame>,
    loop_range: RwLock<Option<(Frame, Frame)>>,
    playback_speed: Arc<RwLock<f32>>,
    audio_scrub: RwLock<bool>,
    event_tx: Sender<PlaybackEvent>,
    event_rx: Receiver<PlaybackEvent>,
    frame_timer: parking_lot::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Playhead {
    pub fn new(frame_rate: FrameRate, duration: Frame) -> Self {
        let (tx, rx) = unbounded();
        Self {
            position: Arc::new(AtomicI64::new(0)),
            state: Arc::new(RwLock::new(PlaybackState::Stopped)),
            frame_rate,
            duration: RwLock::new(duration),
            loop_range: RwLock::new(None),
            playback_speed: Arc::new(RwLock::new(1.0)),
            audio_scrub: RwLock::new(false),
            event_tx: tx,
            event_rx: rx,
            frame_timer: parking_lot::Mutex::new(None),
        }
    }

    pub fn position(&self) -> Frame {
        Frame(self.position.load(Ordering::Relaxed))
    }

    pub fn set_position(&self, frame: Frame) {
        let clamped = frame.0.clamp(0, self.duration().0);
        self.position.store(clamped, Ordering::Relaxed);
        self.emit_position();
    }

    pub fn timecode(&self) -> Timecode {
        Timecode::from_frame(self.position(), self.frame_rate)
    }

    pub fn state(&self) -> PlaybackState {
        *self.state.read()
    }

    pub fn set_state(&self, state: PlaybackState) {
        let mut s = self.state.write();
        *s = state;
        drop(s);
        self.emit_state(state);
    }

    pub fn play(&self) {
        self.set_state(PlaybackState::Playing);
        self.start_frame_timer();
    }

    pub fn pause(&self) {
        self.set_state(PlaybackState::Paused);
        self.stop_frame_timer();
    }

    pub fn stop(&self) {
        self.set_state(PlaybackState::Stopped);
        self.stop_frame_timer();
        self.set_position(Frame::ZERO);
    }

    pub fn toggle(&self) {
        match self.state() {
            PlaybackState::Playing => self.pause(),
            _ => self.play(),
        }
    }

    pub fn set_loop_range(&self, start: Frame, end: Frame) {
        if start < end {
            *self.loop_range.write() = Some((start, end));
        } else {
            *self.loop_range.write() = None;
        }
    }

    pub fn clear_loop(&self) {
        *self.loop_range.write() = None;
    }

    pub fn set_playback_speed(&self, speed: f32) {
        *self.playback_speed.write() = speed.clamp(-16.0, 16.0);
    }

    pub fn playback_speed(&self) -> f32 {
        *self.playback_speed.read()
    }

    pub fn set_audio_scrub(&self, enabled: bool) {
        *self.audio_scrub.write() = enabled;
    }

    pub fn step_forward(&self, frames: i64) {
        let new_pos = (self.position() + Frame(frames))
            .0
            .clamp(0, self.duration().0);
        self.set_position(Frame(new_pos));
    }

    pub fn step_backward(&self, frames: i64) {
        self.step_forward(-frames);
    }

    pub fn next_frame(&self) {
        self.step_forward(1);
    }

    pub fn prev_frame(&self) {
        self.step_forward(-1);
    }

    pub fn go_to_start(&self) {
        self.set_position(Frame::ZERO);
    }

    pub fn go_to_end(&self) {
        let end = self.duration();
        self.set_position(end);
    }

    pub fn go_to_timecode(&self, tc: Timecode) {
        self.set_position(tc.to_frame());
    }

    pub fn set_duration(&self, duration: Frame) {
        *self.duration.write() = duration;
    }

    pub fn duration(&self) -> Frame {
        *self.duration.read()
    }

    pub fn frame_rate(&self) -> FrameRate {
        self.frame_rate
    }

    pub fn event_receiver(&self) -> &Receiver<PlaybackEvent> {
        &self.event_rx
    }

    fn start_frame_timer(&self) {
        if self.frame_timer.lock().is_some() {
            return;
        }

        // Clone the shared handles; the data behind them is only ever read.
        let position = self.position.clone();
        let state = self.state.clone();
        let frame_rate = self.frame_rate;
        let duration = self.duration();
        let loop_range = *self.loop_range.read();
        let playback_speed = self.playback_speed.clone();
        let event_tx = self.event_tx.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(16)); // ~60fps timer

            loop {
                interval.tick().await;

                let current_state = *state.read();
                if current_state != PlaybackState::Playing {
                    break;
                }

                let speed = *playback_speed.read();
                if speed == 0.0 {
                    continue;
                }

                let current = Frame(position.load(Ordering::Relaxed));
                let frame_duration = 1.0 / frame_rate.fps_f32() / speed.abs();
                let frames_to_advance =
                    (speed * frame_rate.fps_f32() * frame_duration).round() as i64;

                let new_pos = (current.0 + frames_to_advance).clamp(0, duration.0);

                // Check loop
                if let Some((loop_start, loop_end)) = loop_range {
                    if new_pos >= loop_end.0 {
                        position.store(loop_start.0, Ordering::Relaxed);
                        let _ = event_tx.send(PlaybackEvent::LoopPointReached {
                            loop_start,
                            loop_end,
                        });
                        continue;
                    }
                    if new_pos < loop_start.0 && speed < 0.0 {
                        position.store(loop_end.0, Ordering::Relaxed);
                        let _ = event_tx.send(PlaybackEvent::LoopPointReached {
                            loop_start,
                            loop_end,
                        });
                        continue;
                    }
                }

                if new_pos >= duration.0 {
                    position.store(duration.0, Ordering::Relaxed);
                    let _ = event_tx.send(PlaybackEvent::EndReached);
                    *state.write() = PlaybackState::Stopped;
                    let _ = event_tx.send(PlaybackEvent::StateChanged {
                        state: PlaybackState::Stopped,
                    });
                    break;
                }

                position.store(new_pos, Ordering::Relaxed);

                let tc = Timecode::from_frame(Frame(new_pos), frame_rate);
                let _ = event_tx.send(PlaybackEvent::PositionChanged {
                    frame: Frame(new_pos),
                    timecode: tc,
                });
            }
        });

        *self.frame_timer.lock() = Some(handle);
    }

    fn stop_frame_timer(&self) {
        if let Some(handle) = self.frame_timer.lock().take() {
            handle.abort();
        }
    }

    fn emit_position(&self) {
        let frame = self.position();
        let tc = self.timecode();
        let _ = self.event_tx.send(PlaybackEvent::PositionChanged {
            frame,
            timecode: tc,
        });
    }

    fn emit_state(&self, state: PlaybackState) {
        let _ = self.event_tx.send(PlaybackEvent::StateChanged { state });
    }
}

/// Playhead controller for UI
pub struct PlayheadController {
    playhead: Arc<Playhead>,
    jog_mode: bool,
    jog_accumulator: f32,
}

impl PlayheadController {
    pub fn new(playhead: Arc<Playhead>) -> Self {
        Self {
            playhead,
            jog_mode: false,
            jog_accumulator: 0.0,
        }
    }

    pub fn jog(&mut self, delta: f32) {
        if self.jog_mode {
            self.jog_accumulator += delta;
            let frames = self.jog_accumulator.round() as i64;
            if frames != 0 {
                self.playhead.step_forward(frames);
                self.jog_accumulator -= frames as f32;
            }
        } else {
            // Shuttle mode - variable speed
            let speed = delta.clamp(-1.0, 1.0) * 8.0;
            self.playhead.set_playback_speed(speed);
            if speed != 0.0 {
                self.playhead.play();
            } else {
                self.playhead.pause();
            }
        }
    }

    pub fn set_jog_mode(&mut self, jog: bool) {
        self.jog_mode = jog;
        self.jog_accumulator = 0.0;
    }

    pub fn shuttle(&mut self, speed: f32) {
        self.playhead.set_playback_speed(speed.clamp(-16.0, 16.0));
        if speed != 0.0 {
            self.playhead.play();
        } else {
            self.playhead.pause();
        }
    }

    pub fn scrub(&mut self, frame: Frame) {
        self.playhead.set_position(frame);
        if *self.playhead.audio_scrub.read() {
            // Trigger audio preview at this frame
        }
    }
}
