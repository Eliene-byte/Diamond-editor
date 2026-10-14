//! Audio effects - real-time processing

use crate::core::types::{EffectType, EffectParams, EffectValue, Interpolation, Easing, Keyframe};
use crate::core::time::Frame;
use crate::core::id::EffectId;
use crate::audio::dsp::{Biquad, DelayLine, Compressor, Gate, Limiter, EQ, Analyzer};
use std::collections::HashMap;
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

/// Audio effect instance
#[derive(Debug, Clone)]
pub struct AudioEffect {
    pub id: EffectId,
    pub effect_type: EffectType,
    pub enabled: bool,
    pub params: EffectParams,
    pub keyframes: HashMap<String, Vec<Keyframe>>,
    pub mix: f32,
    // DSP state
    dsp_state: Option<AudioDSPState>,
}

#[derive(Debug)]
enum AudioDSPState {
    EQ(EQ),
    Compressor(Compressor),
    Gate(Gate),
    Limiter(Limiter),
    Delay(DelayLine),
    Reverb(ReverbState),
    Pitch(PitchState),
    Filter(Biquad),
}

#[derive(Debug)]
struct ReverbState {
    // Convolution reverb or algorithmic
    impulse_response: Option<Vec<f32>>,
    delay_lines: Vec<DelayLine>,
    mix: f32,
}

#[derive(Debug)]
struct PitchState {
    resampler: Option<SincFixedIn<f32>>,
    ratio: f32,
}

impl AudioEffect {
    pub fn new(effect_type: EffectType, sample_rate: u32) -> Self {
        Self {
            id: EffectId::generate(),
            effect_type,
            enabled: true,
            params: Self::default_params(effect_type),
            keyframes: HashMap::new(),
            mix: 1.0,
            dsp_state: Self::create_dsp_state(effect_type, sample_rate),
        }
    }

    fn default_params(effect_type: EffectType) -> EffectParams {
        let mut params = HashMap::new();
        
        match effect_type {
            EffectType::Volume => {
                params.insert("volume".into(), EffectValue::Float(1.0));
                params.insert("pan".into(), EffectValue::Float(0.0));
            }
            EffectType::Pan => {
                params.insert("pan".into(), EffectValue::Float(0.0));
                params.insert("law".into(), EffectValue::Int(0)); // Linear, Constant Power, etc.
            }
            EffectType::EQ => {
                // Parametric EQ bands
                for i in 0..6 {
                    params.insert(&format!("band{}_freq", i), EffectValue::Float([100.0, 250.0, 1000.0, 4000.0, 8000.0, 16000.0][i]));
                    params.insert(&format!("band{}_gain", i), EffectValue::Float(0.0));
                    params.insert(&format!("band{}_q", i), EffectValue::Float(1.0));
                    params.insert(&format!("band{}_type", i), EffectValue::Int(0)); // Peak, Low Shelf, High Shelf, Low Pass, High Pass
                    params.insert(&format!("band{}_enabled", i), EffectValue::Bool(true));
                }
                params.insert("analyzer".into(), EffectValue::Bool(false));
            }
            EffectType::Dynamics => {
                params.insert("threshold".into(), EffectValue::Float(-24.0));
                params.insert("ratio".into(), EffectValue::Float(4.0));
                params.insert("attack".into(), EffectValue::Float(10.0)); // ms
                params.insert("release".into(), EffectValue::Float(100.0)); // ms
                params.insert("knee".into(), EffectValue::Float(6.0)); // dB
                params.insert("makeup".into(), EffectValue::Float(0.0));
                params.insert("mode".into(), EffectValue::Int(0)); // Compressor, Limiter, Expander, Gate
            }
            EffectType::Delay => {
                params.insert("time".into(), EffectValue::Float(500.0)); // ms
                params.insert("feedback".into(), EffectValue::Float(0.3));
                params.insert("mix".into(), EffectValue::Float(0.5));
                params.insert("sync".into(), EffectValue::Bool(false));
                params.insert("low_cut".into(), EffectValue::Float(20.0));
                params.insert("high_cut".into(), EffectValue::Float(20000.0));
            }
            EffectType::Reverb => {
                params.insert("room_size".into(), EffectValue::Float(0.5));
                params.insert("damping".into(), EffectValue::Float(0.5));
                params.insert("width".into(), EffectValue::Float(1.0));
                params.insert("mix".into(), EffectValue::Float(0.3));
                params.insert("pre_delay".into(), EffectValue::Float(20.0));
                params.insert("low_cut".into(), EffectValue::Float(100.0));
                params.insert("high_cut".into(), EffectValue::Float(8000.0));
            }
            EffectType::Pitch => {
                params.insert("semitones".into(), EffectValue::Float(0.0));
                params.insert("cents".into(), EffectValue::Float(0.0));
                params.insert("formant".into(), EffectValue::Bool(true));
                params.insert("quality".into(), EffectValue::Int(2)); // Low, Medium, High
            }
            EffectType::Filter => {
                params.insert("type".into(), EffectValue::Int(0)); // Low Pass, High Pass, Band Pass, Notch
                params.insert("frequency".into(), EffectValue::Float(1000.0));
                params.insert("resonance".into(), EffectValue::Float(1.0));
                params.insert("slope".into(), EffectValue::Int(0)); // 6dB, 12dB, 24dB, 48dB
            }
            EffectType::NoiseReduction => {
                params.insert("reduction".into(), EffectValue::Float(6.0)); // dB
                params.insert("threshold".into(), EffectValue::Float(-40.0));
                params.insert("sensitivity".into(), EffectValue::Float(6.0));
                params.insert("frequency_smoothing".into(), EffectValue::Float(3.0));
            }
            _ => {}
        }
        
        EffectParams { effect_type, parameters: params }
    }

    fn create_dsp_state(effect_type: EffectType, sample_rate: u32) -> Option<AudioDSPState> {
        match effect_type {
            EffectType::EQ => Some(AudioDSPState::EQ(EQ::new(sample_rate))),
            EffectType::Dynamics => Some(AudioDSPState::Compressor(Compressor::new(sample_rate))),
            EffectType::Gate => Some(AudioDSPState::Gate(Gate::new(sample_rate))),
            EffectType::Limiter => Some(AudioDSPState::Limiter(Limiter::new(sample_rate))),
            EffectType::Delay => Some(AudioDSPState::Delay(DelayLine::new(sample_rate, 2.0))), // 2s max
            EffectType::Reverb => Some(AudioDSPState::Reverb(ReverbState { impulse_response: None, delay_lines: Vec::new(), mix: 0.3 })),
            EffectType::Pitch => Some(AudioDSPState::Pitch(PitchState { resampler: None, ratio: 1.0 })),
            EffectType::Filter => Some(AudioDSPState::Filter(Biquad::new())),
            _ => None,
        }
    }

    pub fn process(&mut self, input: &mut [f32], channels: usize, sample_rate: u32) {
        if !self.enabled || self.mix <= 0.0 { return; }
        
        match self.effect_type {
            EffectType::Volume => self.process_volume(input, channels),
            EffectType::Pan => self.process_pan(input, channels),
            EffectType::EQ => self.process_eq(input, channels, sample_rate),
            EffectType::Dynamics => self.process_dynamics(input, channels, sample_rate),
            EffectType::Delay => self.process_delay(input, channels, sample_rate),
            EffectType::Reverb => self.process_reverb(input, channels, sample_rate),
            EffectType::Pitch => self.process_pitch(input, channels, sample_rate),
            EffectType::Filter => self.process_filter(input, channels, sample_rate),
            EffectType::NoiseReduction => self.process_noise_reduction(input, channels, sample_rate),
            _ => {}
        }
    }

    fn process_volume(&mut self, input: &mut [f32], channels: usize) {
        let volume = self.params.parameters.get("volume").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(1.0);
        let pan = self.params.parameters.get("pan").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(0.0);
        
        let (left_gain, right_gain) = Self::pan_to_gains(pan);
        
        for chunk in input.chunks_exact_mut(channels) {
            if channels >= 2 {
                chunk[0] *= volume * left_gain;
                chunk[1] *= volume * right_gain;
            } else if channels == 1 {
                chunk[0] *= volume;
            }
        }
    }

    fn process_pan(&mut self, input: &mut [f32], channels: usize) {
        if channels < 2 { return; }
        let pan = self.params.parameters.get("pan").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(0.0);
        let (left_gain, right_gain) = Self::pan_to_gains(pan);
        
        for chunk in input.chunks_exact_mut(channels) {
            chunk[0] *= left_gain;
            chunk[1] *= right_gain;
        }
    }

    fn pan_to_gains(pan: f32) -> (f32, f32) {
        // Constant power panning
        let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4; // -45 to +45 degrees
        let left = angle.cos();
        let right = angle.sin();
        (left, right)
    }

    fn process_eq(&mut self, input: &mut [f32], channels: usize, sample_rate: u32) {
        if let Some(AudioDSPState::EQ(eq)) = &mut self.dsp_state {
            // Update EQ parameters from params
            for i in 0..6 {
                if let Some(freq) = self.params.parameters.get(&format!("band{}_freq", i)).and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }) {
                    if let Some(gain) = self.params.parameters.get(&format!("band{}_gain", i)).and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }) {
                        if let Some(q) = self.params.parameters.get(&format!("band{}_q", i)).and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }) {
                            if let Some(enabled) = self.params.parameters.get(&format!("band{}_enabled", i)).and_then(|v| match v { EffectValue::Bool(b) => Some(*b), _ => None }) {
                                eq.set_band(i, freq, gain, q, enabled);
                            }
                        }
                    }
                }
            }
            
            // Process
            for chunk in input.chunks_exact_mut(channels) {
                for ch in 0..channels {
                    chunk[ch] = eq.process(chunk[ch], ch);
                }
            }
        }
    }

    fn process_dynamics(&mut self, input: &mut [f32], channels: usize, sample_rate: u32) {
        if let Some(AudioDSPState::Compressor(comp)) = &mut self.dsp_state {
            let threshold = self.params.parameters.get("threshold").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(-24.0);
            let ratio = self.params.parameters.get("ratio").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(4.0);
            let attack = self.params.parameters.get("attack").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(10.0);
            let release = self.params.parameters.get("release").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(100.0);
            let knee = self.params.parameters.get("knee").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(6.0);
            let makeup = self.params.parameters.get("makeup").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(0.0);
            
            comp.set_params(threshold, ratio, attack, release, knee, makeup, sample_rate);
            
            for chunk in input.chunks_exact_mut(channels) {
                for ch in 0..channels {
                    chunk[ch] = comp.process(chunk[ch], ch);
                }
            }
        }
    }

    fn process_delay(&mut self, input: &mut [f32], channels: usize, sample_rate: u32) {
        if let Some(AudioDSPState::Delay(delay)) = &mut self.dsp_state {
            let time = self.params.parameters.get("time").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(500.0);
            let feedback = self.params.parameters.get("feedback").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(0.3);
            let mix = self.params.parameters.get("mix").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(0.5);
            
            delay.set_time(time, sample_rate);
            delay.set_feedback(feedback);
            
            for chunk in input.chunks_exact_mut(channels) {
                for ch in 0..channels {
                    let dry = chunk[ch];
                    let wet = delay.process(dry, ch);
                    chunk[ch] = dry + wet * mix;
                }
            }
        }
    }

    fn process_reverb(&mut self, input: &mut [f32], channels: usize, sample_rate: u32) {
        // Simplified - would use convolution or algorithmic reverb
    }

    fn process_pitch(&mut self, input: &mut [f32], channels: usize, sample_rate: u32) {
        // Would use rubato for pitch shifting
    }

    fn process_filter(&mut self, input: &mut [f32], channels: usize, sample_rate: u32) {
        if let Some(AudioDSPState::Filter(filter)) = &mut self.dsp_state {
            let freq = self.params.parameters.get("frequency").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(1000.0);
            let resonance = self.params.parameters.get("resonance").and_then(|v| match v { EffectValue::Float(f) => Some(*f), _ => None }).unwrap_or(1.0);
            
            filter.set_lowpass(freq, resonance, sample_rate);
            
            for chunk in input.chunks_exact_mut(channels) {
                for ch in 0..channels {
                    chunk[ch] = filter.process(chunk[ch]);
                }
            }
        }
    }

    fn process_noise_reduction(&mut self, input: &mut [f32], channels: usize, sample_rate: u32) {
        // Spectral noise reduction - simplified
    }
}

/// Audio effect chain
pub struct AudioEffectChain {
    effects: Vec<AudioEffect>,
    sample_rate: u32,
}

impl AudioEffectChain {
    pub fn new(sample_rate: u32) -> Self {
        Self { effects: Vec::new(), sample_rate }
    }

    pub fn add(&mut self, effect: AudioEffect) {
        self.effects.push(effect);
    }

    pub fn remove(&mut self, index: usize) -> Option<AudioEffect> {
        if index < self.effects.len() {
            Some(self.effects.remove(index))
        } else { None }
    }

    pub fn process(&mut self, buffer: &mut [f32], channels: usize) {
        for effect in &mut self.effects {
            effect.process(buffer, channels, self.sample_rate);
        }
    }

    pub fn len(&self) -> usize { self.effects.len() }
}