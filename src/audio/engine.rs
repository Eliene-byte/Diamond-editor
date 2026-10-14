//! Audio engine - playback, routing, processing

use crate::audio::dsp::{Compressor, Gate, Limiter, EQ, Analyzer, DelayLine, Biquad};
use crate::core::time::Frame;
use cpal::{Stream, StreamConfig, SampleRate, SampleFormat, ChannelCount};
use std::sync::{Arc, Mutex};
use crossbeam::channel::{Sender, Receiver, unbounded};

pub struct AudioEngine {
    stream: Option<Stream>,
    config: StreamConfig,
    sample_rate: u32,
    channels: usize,
    buffer_size: usize,
    
    // Processing chain
    master_compressor: Compressor,
    master_limiter: Limiter,
    master_eq: EQ,
    analyzer: Analyzer,
    
    // Audio buffers
    input_buffer: Arc<Mutex<Vec<f32>>>,
    output_buffer: Arc<Mutex<Vec<f32>>>,
    
    // Playback
    playback_tx: Sender<AudioBuffer>,
    playback_rx: Receiver<AudioBuffer>,
    
    // Routing
    tracks: Vec<AudioTrack>,
    buses: Vec<AudioBus>,
    master_bus: AudioBus,
}

#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub data: Vec<f32>,
    pub channels: usize,
    pub sample_rate: u32,
    pub frame: Frame,
}

#[derive(Debug, Clone)]
struct AudioTrack {
    id: usize,
    name: String,
    volume: f32,
    pan: f32,
    mute: bool,
    solo: bool,
    effects: Vec<AudioEffect>,
    input_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
}

#[derive(Debug, Clone)]
enum AudioEffect {
    EQ(EQ),
    Compressor(Compressor),
    Gate(Gate),
    Limiter(Limiter),
    Delay(DelayLine),
}

#[derive(Debug, Clone)]
struct AudioBus {
    id: usize,
    name: String,
    volume: f32,
    channels: usize,
    input_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
    sends: Vec<BusSend>,
}

#[derive(Debug, Clone)]
struct BusSend {
    bus_id: usize,
    level: f32,
    pre_fader: bool,
}

impl AudioEngine {
    pub fn new(sample_rate: u32, channels: usize, buffer_size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("No output device")?;
        let config = device.default_output_config()?.config();
        
        let (playback_tx, playback_rx) = unbounded();
        
        Ok(Self {
            stream: None,
            config,
            sample_rate,
            channels,
            buffer_size,
            master_compressor: Compressor::new(sample_rate),
            master_limiter: Limiter::new(sample_rate),
            master_eq: EQ::new(sample_rate),
            analyzer: Analyzer::new(sample_rate),
            input_buffer: Arc::new(Mutex::new(vec![0.0; buffer_size * channels])),
            output_buffer: Arc::new(Mutex::new(vec![0.0; buffer_size * channels])),
            playback_tx,
            playback_rx,
            tracks: Vec::new(),
            buses: Vec::new(),
            master_bus: AudioBus { id: 0, name: "Master".into(), volume: 1.0, channels, input_buffer: vec![0.0; buffer_size * channels], output_buffer: vec![0.0; buffer_size * channels], sends: Vec::new() },
        })
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let device = cpal::default_host().default_output_device().ok_or("No output device")?;
        let config = self.config.clone();
        
        let output_buffer = self.output_buffer.clone();
        
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(mut buf) = output_buffer.lock() {
                    let len = data.len().min(buf.len());
                    data[..len].copy_from_slice(&buf[..len]);
                    buf[..len].fill(0.0);
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;
        
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stream = None;
    }

    pub fn process_callback(&mut self, input: &[f32], output: &mut [f32]) {
        // Copy input to buffer
        if let Ok(mut buf) = self.input_buffer.lock() {
            let len = input.len().min(buf.len());
            buf[..len].copy_from_slice(&input[..len]);
        }
        
        // Process audio
        self.process_audio();
        
        // Copy to output
        if let Ok(buf) = self.output_buffer.lock() {
            let len = output.len().min(buf.len());
            output[..len].copy_from_slice(&buf[..len]);
        }
    }

    fn process_audio(&mut self) {
        // Get input buffer
        let input = {
            let buf = self.input_buffer.lock().unwrap();
            buf.clone()
        };
        
        // Mix tracks to buses
        for track in &mut self.tracks {
            if track.mute { continue; }
            
            // Process track effects
            let mut track_output = track.input_buffer.clone();
            
            for effect in &mut track.effects {
                match effect {
                    AudioEffect::EQ(eq) => {
                        for (i, sample) in track_output.iter_mut().enumerate() {
                            *sample = eq.process(*sample, i % self.channels);
                        }
                    }
                    AudioEffect::Compressor(comp) => {
                        for (i, sample) in track_output.iter_mut().enumerate() {
                            *sample = comp.process(*sample, i % self.channels);
                        }
                    }
                    AudioEffect::Gate(gate) => {
                        for (i, sample) in track_output.iter_mut().enumerate() {
                            *sample = gate.process(*sample);
                        }
                    }
                    AudioEffect::Limiter(lim) => {
                        for (i, sample) in track_output.iter_mut().enumerate() {
                            *sample = lim.process(*sample);
                        }
                    }
                    AudioEffect::Delay(delay) => {
                        for (i, sample) in track_output.iter_mut().enumerate() {
                            *sample = delay.process(*sample, i % self.channels);
                        }
                    }
                }
            }
            
            // Apply volume and pan
            for (i, sample) in track_output.iter_mut().enumerate() {
                *sample *= track.volume;
                if self.channels >= 2 {
                    let ch = i % 2;
                    if ch == 0 { *sample *= (1.0 - track.pan).max(0.0); }
                    else { *sample *= (1.0 + track.pan).max(0.0); }
                }
            }
            
            track.output_buffer = track_output;
        }
        
        // Mix to master bus
        let mut master_input = vec![0.0; self.master_bus.input_buffer.len()];
        
        for track in &self.tracks {
            if track.solo || !self.tracks.iter().any(|t| t.solo) {
                for (i, sample) in track.output_buffer.iter().enumerate() {
                    if i < master_input.len() {
                        master_input[i] += sample;
                    }
                }
            }
        }
        
        // Master processing
        for (i, sample) in master_input.iter_mut().enumerate() {
            *sample = self.master_eq.process(*sample, i % self.channels);
            *sample = self.master_compressor.process(*sample, i % self.channels);
            *sample = self.master_limiter.process(*sample);
            
            // Analyze
            let (rms, peak, peak_hold) = self.analyzer.process(*sample);
        }
        
        // Apply master volume
        for sample in &mut master_input {
            *sample *= self.master_bus.volume;
        }
        
        // Copy to output buffer
        if let Ok(mut buf) = self.output_buffer.lock() {
            let len = master_input.len().min(buf.len());
            buf[..len].copy_from_slice(&master_input[..len]);
        }
    }

    pub fn add_track(&mut self, name: String) -> usize {
        let id = self.tracks.len();
        self.tracks.push(AudioTrack {
            id,
            name,
            volume: 1.0,
            pan: 0.0,
            mute: false,
            solo: false,
            effects: Vec::new(),
            input_buffer: vec![0.0; self.buffer_size * self.channels],
            output_buffer: vec![0.0; self.buffer_size * self.channels],
        });
        id
    }

    pub fn get_metering(&self) -> (f32, f32, f32) {
        // Would return actual metering data
        (0.0, 0.0, 0.0)
    }
}