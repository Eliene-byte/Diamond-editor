//! Audio mixer - routing, automation, control surfaces

use crate::audio::engine::{AudioEngine, AudioBuffer};
use crate::core::id::{TrackId, AudioBusId};
use crate::core::time::Frame;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct AudioMixer {
    engine: Arc<RwLock<AudioEngine>>,
    channel_strips: HashMap<TrackId, ChannelStrip>,
    bus_strips: HashMap<AudioBusId, BusStrip>,
    master_strip: MasterStrip,
    automation: AutomationManager,
}

#[derive(Debug, Clone)]
struct ChannelStrip {
    track_id: TrackId,
    name: String,
    volume: f32,
    pan: f32,
    mute: bool,
    solo: bool,
    input: String,
    output: String,
    eq: EQState,
    dynamics: DynamicsState,
    inserts: Vec<InsertSlot>,
    sends: Vec<SendSlot>,
    meter: MeterState,
}

#[derive(Debug, Clone, Default)]
struct EQState {
    enabled: bool,
    bands: [EQBandState; 6],
}

#[derive(Debug, Clone, Default)]
struct EQBandState {
    enabled: bool,
    freq: f32,
    gain: f32,
    q: f32,
    filter_type: super::dsp::FilterType,
}

#[derive(Debug, Clone, Default)]
struct DynamicsState {
    compressor: CompressorState,
    gate: GateState,
    limiter: LimiterState,
}

#[derive(Debug, Clone, Default)]
struct CompressorState {
    enabled: bool,
    threshold: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    knee: f32,
    makeup: f32,
}

#[derive(Debug, Clone, Default)]
struct GateState {
    enabled: bool,
    threshold: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    hold: f32,
}

#[derive(Debug, Clone, Default)]
struct LimiterState {
    enabled: bool,
    threshold: f32,
    release: f32,
}

#[derive(Debug, Clone)]
struct InsertSlot {
    effect_type: String,
    enabled: bool,
    params: HashMap<String, f32>,
}

#[derive(Debug, Clone)]
struct SendSlot {
    bus_id: AudioBusId,
    level: f32,
    pre_fader: bool,
    enabled: bool,
}

#[derive(Debug, Clone, Default)]
struct MeterState {
    peak_left: f32,
    peak_right: f32,
    rms_left: f32,
    rms_right: f32,
    peak_hold_left: f32,
    peak_hold_right: f32,
}

#[derive(Debug, Clone)]
struct BusStrip {
    bus_id: AudioBusId,
    name: String,
    volume: f32,
    mute: bool,
    solo: bool,
    inserts: Vec<InsertSlot>,
    meter: MeterState,
}

#[derive(Debug, Clone)]
struct MasterStrip {
    volume: f32,
    mute: bool,
    dim: bool,
    mono: bool,
    inserts: Vec<InsertSlot>,
    meter: MeterState,
}

#[derive(Debug, Clone)]
struct AutomationManager {
    lanes: HashMap<String, AutomationLane>,
}

#[derive(Debug, Clone)]
struct AutomationLane {
    parameter: String,
    points: Vec<AutomationPoint>,
    mode: AutomationMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutomationMode { Read, Write, Touch, Latch, Off }

#[derive(Debug, Clone)]
struct AutomationPoint {
    frame: Frame,
    value: f32,
    interpolation: super::core::types::Interpolation,
}

impl AudioMixer {
    pub fn new(engine: Arc<RwLock<AudioEngine>>) -> Self {
        Self {
            engine,
            channel_strips: HashMap::new(),
            bus_strips: HashMap::new(),
            master_strip: MasterStrip { volume: 1.0, mute: false, dim: false, mono: false, inserts: Vec::new(), meter: MeterState::default() },
            automation: AutomationManager { lanes: HashMap::new() },
        }
    }

    pub fn add_channel(&mut self, track_id: TrackId, name: String) {
        self.channel_strips.insert(track_id, ChannelStrip {
            track_id,
            name,
            volume: 1.0,
            pan: 0.0,
            mute: false,
            solo: false,
            input: "In 1-2".into(),
            output: "Bus 1".into(),
            eq: EQState::default(),
            dynamics: DynamicsState::default(),
            inserts: Vec::new(),
            sends: Vec::new(),
            meter: MeterState::default(),
        });
    }

    pub fn add_bus(&mut self, bus_id: AudioBusId, name: String) {
        self.bus_strips.insert(bus_id, BusStrip {
            bus_id,
            name,
            volume: 1.0,
            mute: false,
            solo: false,
            inserts: Vec::new(),
            meter: MeterState::default(),
        });
    }

    pub fn set_volume(&mut self, track_id: TrackId, volume: f32) {
        if let Some(strip) = self.channel_strips.get_mut(&track_id) {
            strip.volume = volume.clamp(0.0, 2.0);
        }
    }

    pub fn set_pan(&mut self, track_id: TrackId, pan: f32) {
        if let Some(strip) = self.channel_strips.get_mut(&track_id) {
            strip.pan = pan.clamp(-1.0, 1.0);
        }
    }

    pub fn set_mute(&mut self, track_id: TrackId, mute: bool) {
        if let Some(strip) = self.channel_strips.get_mut(&track_id) {
            strip.mute = mute;
        }
    }

    pub fn set_solo(&mut self, track_id: TrackId, solo: bool) {
        if let Some(strip) = self.channel_strips.get_mut(&track_id) {
            strip.solo = solo;
        }
    }

    pub fn add_send(&mut self, track_id: TrackId, bus_id: AudioBusId, level: f32, pre_fader: bool) {
        if let Some(strip) = self.channel_strips.get_mut(&track_id) {
            strip.sends.push(SendSlot {
                bus_id,
                level: level.clamp(0.0, 1.0),
                pre_fader,
                enabled: true,
            });
        }
    }

    pub fn set_eq(&mut self, track_id: TrackId, band: usize, freq: f32, gain: f32, q: f32, filter_type: super::dsp::FilterType) {
        if let Some(strip) = self.channel_strips.get_mut(&track_id) {
            if band < strip.eq.bands.len() {
                strip.eq.bands[band] = EQBandState {
                    enabled: true,
                    freq: freq.clamp(20.0, 20000.0),
                    gain: gain.clamp(-24.0, 24.0),
                    q: q.clamp(0.1, 10.0),
                    filter_type,
                };
            }
        }
    }

    pub fn set_compressor(&mut self, track_id: TrackId, threshold: f32, ratio: f32, attack: f32, release: f32, knee: f32, makeup: f32) {
        if let Some(strip) = self.channel_strips.get_mut(&track_id) {
            strip.dynamics.compressor = CompressorState {
                enabled: true,
                threshold: threshold.clamp(-60.0, 0.0),
                ratio: ratio.clamp(1.0, 100.0),
                attack: attack.clamp(0.1, 500.0),
                release: release.clamp(5.0, 5000.0),
                knee: knee.clamp(0.0, 24.0),
                makeup: makeup.clamp(-12.0, 24.0),
            };
        }
    }

    pub fn write_automation(&mut self, parameter: String, frame: Frame, value: f32) {
        self.automation.lanes.entry(parameter).or_insert_with(|| AutomationLane {
            parameter: String::new(),
            points: Vec::new(),
            mode: AutomationMode::Write,
        }).points.push(AutomationPoint {
            frame,
            value,
            interpolation: super::core::types::Interpolation::Linear,
        });
    }

    pub fn read_automation(&self, parameter: &str, frame: Frame) -> Option<f32> {
        self.automation.lanes.get(parameter).and_then(|lane| {
            lane.points.iter()
                .filter(|p| p.frame <= frame)
                .max_by_key(|p| p.frame)
                .map(|p| p.value)
        })
    }
}