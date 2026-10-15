//! Audio DSP - filters, dynamics, EQ, etc.

/// Biquad filter - 2nd order IIR filter
#[derive(Debug, Clone)]
pub struct Biquad {
    a0: f32, a1: f32, a2: f32,
    b1: f32, b2: f32,
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl Biquad {
    pub fn new() -> Self {
        Self { a0: 1.0, a1: 0.0, a2: 0.0, b1: 0.0, b2: 0.0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    pub fn set_lowpass(&mut self, freq: f32, q: f32, sample_rate: u32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate as f32;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        
        let b0 = (1.0 - cos_w0) * 0.5;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        
        self.set_coefficients(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    }

    pub fn set_highpass(&mut self, freq: f32, q: f32, sample_rate: u32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate as f32;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        
        let b0 = (1.0 + cos_w0) * 0.5;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        
        self.set_coefficients(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    }

    pub fn set_bandpass(&mut self, freq: f32, q: f32, sample_rate: u32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate as f32;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        
        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        
        self.set_coefficients(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    }

    pub fn set_notch(&mut self, freq: f32, q: f32, sample_rate: u32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate as f32;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        
        let b0 = 1.0;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        
        self.set_coefficients(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    }

    pub fn set_peaking_eq(&mut self, freq: f32, gain_db: f32, q: f32, sample_rate: u32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate as f32;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let alpha = sin_w0 / (2.0 * q);
        
        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;
        
        self.set_coefficients(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    }

    pub fn set_low_shelf(&mut self, freq: f32, gain_db: f32, s: f32, sample_rate: u32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate as f32;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let alpha = sin_w0 / 2.0 * ((a + 1.0/a) * (1.0/s - 1.0) + 2.0).sqrt();
        
        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * alpha.sqrt());
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * alpha.sqrt());
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * alpha.sqrt();
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * alpha.sqrt();
        
        self.set_coefficients(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    }

    pub fn set_high_shelf(&mut self, freq: f32, gain_db: f32, s: f32, sample_rate: u32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate as f32;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let alpha = sin_w0 / 2.0 * ((a + 1.0/a) * (1.0/s - 1.0) + 2.0).sqrt();
        
        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * alpha.sqrt());
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * alpha.sqrt());
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * alpha.sqrt();
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * alpha.sqrt();
        
        self.set_coefficients(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    }

    fn set_coefficients(&mut self, b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) {
        self.a0 = b0;
        self.a1 = b1;
        self.a2 = b2;
        self.b1 = a1;
        self.b2 = a2;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let y = self.a0 * input + self.a1 * self.x1 + self.a2 * self.x2 - self.b1 * self.y1 - self.b2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        (self.process(left), self.process(right))
    }
}

/// Delay line
#[derive(Debug, Clone)]
pub struct DelayLine {
    buffer: Vec<f32>,
    write_pos: usize,
    max_delay: usize,
    feedback: f32,
    mix: f32,
}

impl DelayLine {
    pub fn new(sample_rate: u32, max_delay_sec: f32) -> Self {
        let max_delay = (sample_rate as f32 * max_delay_sec) as usize;
        Self { buffer: vec![0.0; max_delay], write_pos: 0, max_delay, feedback: 0.0, mix: 1.0 }
    }

    pub fn set_time(&mut self, delay_ms: f32, sample_rate: u32) {
        let delay_samples = ((delay_ms / 1000.0) * sample_rate as f32) as usize;
        self.max_delay = delay_samples.min(self.buffer.len());
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.99);
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn process(&mut self, input: f32, channel: usize) -> f32 {
        let read_pos = (self.write_pos + self.buffer.len() - self.max_delay) % self.buffer.len();
        let delayed = self.buffer[read_pos];
        
        let output = input + delayed * self.feedback;
        self.buffer[self.write_pos] = output;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        
        input + delayed * self.mix
    }
}

/// Compressor
#[derive(Debug, Clone)]
pub struct Compressor {
    threshold: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    knee: f32,
    makeup_gain: f32,
    envelope: f32,
    sample_rate: u32,
}

impl Compressor {
    pub fn new(sample_rate: u32) -> Self {
        Self { threshold: -24.0, ratio: 4.0, attack: 0.01, release: 0.1, knee: 6.0, makeup_gain: 0.0, envelope: 0.0, sample_rate }
    }

    pub fn set_params(&mut self, threshold: f32, ratio: f32, attack_ms: f32, release_ms: f32, knee: f32, makeup_db: f32, sample_rate: u32) {
        self.threshold = threshold;
        self.ratio = ratio;
        self.attack = (-1.0 / (attack_ms / 1000.0 * sample_rate as f32)).exp();
        self.release = (-1.0 / (release_ms / 1000.0 * sample_rate as f32)).exp();
        self.knee = knee;
        self.makeup_gain = 10.0_f32.powf(makeup_db / 20.0);
        self.sample_rate = sample_rate;
    }

    pub fn process(&mut self, input: f32, channel: usize) -> f32 {
        let input_db = 20.0 * input.abs().max(1e-10).log10();
        
        // Knee
        let over_threshold = input_db - self.threshold;
        let gain_db = if self.knee > 0.0 && over_threshold.abs() < self.knee {
            // Soft knee
            let x = over_threshold + self.knee;
            x * x / (4.0 * self.knee)
        } else if over_threshold > 0.0 {
            over_threshold
        } else {
            0.0
        };
        
        let gain_reduction = gain_db * (1.0 - 1.0 / self.ratio);
        let target_gain = 10.0_f32.powf(-gain_reduction / 20.0);
        
        // Envelope follower
        if target_gain < self.envelope {
            self.envelope = self.attack * self.envelope + (1.0 - self.attack) * target_gain;
        } else {
            self.envelope = self.release * self.envelope + (1.0 - self.release) * target_gain;
        }
        
        input * self.envelope * self.makeup_gain
    }
}

/// Gate/Expander
#[derive(Debug, Clone)]
pub struct Gate {
    threshold: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    hold: f32,
    envelope: f32,
    hold_timer: f32,
    sample_rate: u32,
}

impl Gate {
    pub fn new(sample_rate: u32) -> Self {
        Self { threshold: -40.0, ratio: 10.0, attack: 0.001, release: 0.1, hold: 0.05, envelope: 0.0, hold_timer: 0.0, sample_rate }
    }

    pub fn set_params(&mut self, threshold: f32, ratio: f32, attack_ms: f32, release_ms: f32, hold_ms: f32, sample_rate: u32) {
        self.threshold = threshold;
        self.ratio = ratio;
        self.attack = (-1.0 / (attack_ms / 1000.0 * sample_rate as f32)).exp();
        self.release = (-1.0 / (release_ms / 1000.0 * sample_rate as f32)).exp();
        self.hold = hold_ms / 1000.0;
        self.sample_rate = sample_rate;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let input_db = 20.0 * input.abs().max(1e-10).log10();
        let below_threshold = input_db < self.threshold;
        
        if below_threshold {
            if self.hold_timer <= 0.0 {
                self.envelope = self.release * self.envelope + (1.0 - self.release) * 0.0;
            } else {
                self.hold_timer -= 1.0 / self.sample_rate as f32;
            }
        } else {
            self.envelope = self.attack * self.envelope + (1.0 - self.attack) * 1.0;
            self.hold_timer = self.hold;
        }
        
        input * self.envelope
    }
}

/// Limiter
#[derive(Debug, Clone)]
pub struct Limiter {
    threshold: f32,
    release: f32,
    envelope: f32,
    sample_rate: u32,
}

impl Limiter {
    pub fn new(sample_rate: u32) -> Self {
        Self { threshold: -1.0, release: 0.05, envelope: 0.0, sample_rate }
    }

    pub fn set_params(&mut self, threshold_db: f32, release_ms: f32, sample_rate: u32) {
        self.threshold = 10.0_f32.powf(threshold_db / 20.0);
        self.release = (-1.0 / (release_ms / 1000.0 * sample_rate as f32)).exp();
        self.sample_rate = sample_rate;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let abs_input = input.abs();
        
        if abs_input > self.envelope {
            self.envelope = abs_input;
        } else {
            self.envelope = self.release * self.envelope + (1.0 - self.release) * abs_input;
        }
        
        if self.envelope > self.threshold {
            input * (self.threshold / self.envelope)
        } else {
            input
        }
    }
}

/// Parametric EQ
#[derive(Debug, Clone)]
pub struct EQ {
    bands: Vec<EQBand>,
    sample_rate: u32,
}

#[derive(Debug, Clone)]
struct EQBand {
    filter: Biquad,
    enabled: bool,
    freq: f32,
    gain: f32,
    q: f32,
    filter_type: FilterType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterType {
    #[default] Peak, LowShelf, HighShelf, LowPass, HighPass, Notch }

impl EQ {
    pub fn new(sample_rate: u32) -> Self {
        let mut eq = Self { bands: Vec::new(), sample_rate };
        for _ in 0..6 {
            eq.bands.push(EQBand { filter: Biquad::new(), enabled: false, freq: 1000.0, gain: 0.0, q: 1.0, filter_type: FilterType::Peak });
        }
        eq
    }

    pub fn set_band(&mut self, index: usize, freq: f32, gain: f32, q: f32, enabled: bool) {
        if index < self.bands.len() {
            let band = &mut self.bands[index];
            band.enabled = enabled;
            band.freq = freq.clamp(20.0, 20000.0);
            band.gain = gain.clamp(-24.0, 24.0);
            band.q = q.clamp(0.1, 10.0);
            
            if enabled {
                match band.filter_type {
                    FilterType::Peak => band.filter.set_peaking_eq(freq, gain, q, self.sample_rate),
                    FilterType::LowShelf => band.filter.set_low_shelf(freq, gain, 1.0, self.sample_rate),
                    FilterType::HighShelf => band.filter.set_high_shelf(freq, gain, 1.0, self.sample_rate),
                    FilterType::LowPass => band.filter.set_lowpass(freq, q, self.sample_rate),
                    FilterType::HighPass => band.filter.set_highpass(freq, q, self.sample_rate),
                    FilterType::Notch => band.filter.set_notch(freq, q, self.sample_rate),
                }
            }
        }
    }

    pub fn set_band_type(&mut self, index: usize, filter_type: FilterType) {
        if index < self.bands.len() {
            self.bands[index].filter_type = filter_type;
        }
    }

    pub fn process(&mut self, input: f32, channel: usize) -> f32 {
        let mut output = input;
        for band in &mut self.bands {
            if band.enabled {
                output = band.filter.process(output);
            }
        }
        output
    }
}

/// Audio analyzer
#[derive(Debug, Clone)]
pub struct Analyzer {
    rms: f32,
    peak: f32,
    peak_hold: f32,
    peak_hold_timer: f32,
    sample_rate: u32,
}

impl Analyzer {
    pub fn new(sample_rate: u32) -> Self {
        Self { rms: 0.0, peak: 0.0, peak_hold: 0.0, peak_hold_timer: 0.0, sample_rate }
    }

    pub fn process(&mut self, input: f32) -> (f32, f32, f32) {
        let abs = input.abs();
        
        // RMS
        self.rms = 0.999 * self.rms + 0.001 * (input * input);
        let rms_db = 10.0 * self.rms.max(1e-10).log10();
        
        // Peak
        self.peak = self.peak.max(abs);
        let peak_db = 20.0 * self.peak.max(1e-10).log10();
        
        // Peak hold
        if abs > self.peak_hold {
            self.peak_hold = abs;
            self.peak_hold_timer = 2.0; // 2 seconds
        } else {
            self.peak_hold_timer -= 1.0 / self.sample_rate as f32;
            if self.peak_hold_timer <= 0.0 {
                self.peak_hold *= 0.999;
            }
        }
        let peak_hold_db = 20.0 * self.peak_hold.max(1e-10).log10();
        
        (rms_db, peak_db, peak_hold_db)
    }
}