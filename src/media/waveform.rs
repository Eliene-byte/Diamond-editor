//! Audio waveform generation and display

use crate::core::error::{MediaError, Result};
use crate::core::id::{ClipId, MediaId};
use crate::core::time::{Frame, FrameRate};
use ffmpeg_next as ffmpeg;
use std::path::Path;
use std::sync::Arc;

/// Waveform data for visualization
#[derive(Debug, Clone)]
pub struct Waveform {
    pub clip_id: ClipId,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration: Frame,
    pub peaks: Vec<WaveformPeak>,
    pub rms: Vec<f32>,
    pub resolution: usize, // Samples per peak
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveformPeak {
    pub min: f32,
    pub max: f32,
    pub rms: f32,
}

impl Waveform {
    pub fn new(clip_id: ClipId, sample_rate: u32, channels: u16, duration: Frame) -> Self {
        Self {
            clip_id,
            sample_rate,
            channels,
            duration,
            peaks: Vec::new(),
            rms: Vec::new(),
            resolution: 1,
        }
    }

    pub fn peak_at(&self, frame: Frame, fps: FrameRate) -> Option<&WaveformPeak> {
        let sample = frame_to_sample(frame, self.sample_rate, fps);
        let index = sample / self.resolution;
        self.peaks.get(index)
    }

    pub fn rms_at(&self, frame: Frame, fps: FrameRate) -> Option<f32> {
        let sample = frame_to_sample(frame, self.sample_rate, fps);
        let index = sample / self.resolution;
        self.rms.get(index).copied()
    }

    pub fn max_peak(&self) -> f32 {
        self.peaks
            .iter()
            .map(|p| p.max.abs().max(p.min.abs()))
            .fold(0.0, f32::max)
    }

    pub fn duration_seconds(&self) -> f32 {
        self.duration.to_seconds(FrameRate::Fps24) as f32
    }
}

/// Waveform generator
pub struct WaveformGenerator {
    resolution: usize,    // Samples per peak point
    target_points: usize, // Target number of points for display
}

impl Default for WaveformGenerator {
    fn default() -> Self {
        Self {
            resolution: 256, // 256 samples per peak
            target_points: 2048,
        }
    }
}

impl WaveformGenerator {
    pub fn new(resolution: usize, target_points: usize) -> Self {
        Self {
            resolution,
            target_points,
        }
    }

    pub fn generate(&self, path: &Path) -> Result<Waveform> {
        ffmpeg::init()?;

        let mut ictx =
            ffmpeg::format::input(&path).map_err(|e| MediaError::Decoder(e.to_string()))?;

        let audio_stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Audio)
            .ok_or(MediaError::NoAudioStream)?;

        let mut decoder = audio_stream
            .codec()
            .decoder()
            .audio()
            .map_err(|e| MediaError::Decoder(e.to_string()))?;

        let sample_rate = decoder.rate();
        let channels = decoder.channels() as u16;
        let channel_layout = decoder.channel_layout();
        let duration = audio_stream.duration().unwrap_or(0);

        // Setup resampler to mono f32
        let resampler = ffmpeg::software::resampling::Context::get(
            decoder.format(),
            channel_layout,
            sample_rate,
            ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed),
            ffmpeg::channel_layout::ChannelLayout::MONO,
            sample_rate,
        )
        .map_err(|e| MediaError::Decoder(e.to_string()))?;

        let mut waveform = Waveform::new(
            ClipId::nil(), // Will be set by caller
            sample_rate,
            channels,
            Frame(duration),
        );

        waveform.resolution = self.resolution;

        let total_samples = (duration as f64 * sample_rate as f64
            / (audio_stream.time_base().1 as f64 / audio_stream.time_base().0 as f64))
            as usize;

        let peaks_count = (total_samples / self.resolution)
            .max(1)
            .min(self.target_points);
        waveform.peaks = vec![
            WaveformPeak {
                min: 0.0,
                max: 0.0,
                rms: 0.0
            };
            peaks_count
        ];
        waveform.rms = vec![0.0; peaks_count];

        let mut frame = ffmpeg::frame::Audio::empty();
        let mut packet = ffmpeg::Packet::empty();
        let mut sample_buffer = Vec::new();
        let mut peak_idx = 0;
        let mut sample_idx = 0;

        while ictx.read_packet(&mut packet).is_ok() {
            if packet.stream() != audio_stream.index() {
                continue;
            }

            decoder.send_packet(&packet).ok();
            while decoder.receive_frame(&mut frame).is_ok() {
                let mut out_frame = ffmpeg::frame::Audio::empty();
                resampler.run(&frame, &mut out_frame).ok();

                let samples: &[f32] = unsafe {
                    std::slice::from_raw_parts(
                        out_frame.data(0).as_ptr() as *const f32,
                        out_frame.samples(),
                    )
                };

                sample_buffer.extend_from_slice(samples);

                // Process accumulated samples
                while sample_buffer.len() >= self.resolution && peak_idx < peaks_count {
                    let chunk = &sample_buffer[..self.resolution];
                    sample_buffer.drain(..self.resolution);

                    let mut min = f32::INFINITY;
                    let mut max = f32::NEG_INFINITY;
                    let mut sum_sq = 0.0;

                    for &s in chunk {
                        min = min.min(s);
                        max = max.max(s);
                        sum_sq += s * s;
                    }

                    waveform.peaks[peak_idx] = WaveformPeak {
                        min,
                        max,
                        rms: (sum_sq / chunk.len() as f32).sqrt(),
                    };
                    waveform.rms[peak_idx] = (sum_sq / chunk.len() as f32).sqrt();

                    peak_idx += 1;
                    sample_idx += self.resolution;
                }
            }
        }

        // Process remaining
        while !sample_buffer.is_empty() && peak_idx < peaks_count {
            let chunk = &sample_buffer[..self.resolution.min(sample_buffer.len())];
            sample_buffer.drain(..chunk.len());

            if chunk.is_empty() {
                break;
            }

            let mut min = f32::INFINITY;
            let mut max = f32::NEG_INFINITY;
            let mut sum_sq = 0.0;

            for &s in chunk {
                min = min.min(s);
                max = max.max(s);
                sum_sq += s * s;
            }

            waveform.peaks[peak_idx] = WaveformPeak {
                min,
                max,
                rms: (sum_sq / chunk.len() as f32).sqrt(),
            };
            waveform.rms[peak_idx] = (sum_sq / chunk.len() as f32).sqrt();
            peak_idx += 1;
        }

        // Flush
        decoder.send_packet(ffmpeg::Packet::empty()).ok();
        while decoder.receive_frame(&mut frame).is_ok() {
            let mut out_frame = ffmpeg::frame::Audio::empty();
            resampler.run(&frame, &mut out_frame).ok();

            let samples: &[f32] = unsafe {
                std::slice::from_raw_parts(
                    out_frame.data(0).as_ptr() as *const f32,
                    out_frame.samples(),
                )
            };

            sample_buffer.extend_from_slice(samples);

            while sample_buffer.len() >= self.resolution && peak_idx < peaks_count {
                let chunk = &sample_buffer[..self.resolution];
                sample_buffer.drain(..self.resolution);

                let mut min = f32::INFINITY;
                let mut max = f32::NEG_INFINITY;
                let mut sum_sq = 0.0;

                for &s in chunk {
                    min = min.min(s);
                    max = max.max(s);
                    sum_sq += s * s;
                }

                waveform.peaks[peak_idx] = WaveformPeak {
                    min,
                    max,
                    rms: (sum_sq / chunk.len() as f32).sqrt(),
                };
                waveform.rms[peak_idx] = (sum_sq / chunk.len() as f32).sqrt();
                peak_idx += 1;
            }
        }

        Ok(waveform)
    }

    pub fn generate_from_samples(
        &self,
        samples: &[f32],
        sample_rate: u32,
        clip_id: ClipId,
        fps: FrameRate,
    ) -> Waveform {
        let total_samples = samples.len();
        let duration_samples = total_samples;
        let duration_frames =
            Frame((duration_samples as f64 / sample_rate as f64 * fps.fps_f32() as f64) as i64);

        let mut waveform = Waveform::new(clip_id, sample_rate, 1, duration_frames);
        waveform.resolution = self.resolution;

        let peaks_count = (total_samples / self.resolution)
            .max(1)
            .min(self.target_points);
        waveform.peaks = vec![
            WaveformPeak {
                min: 0.0,
                max: 0.0,
                rms: 0.0
            };
            peaks_count
        ];
        waveform.rms = vec![0.0; peaks_count];

        let mut peak_idx = 0;

        for chunk in samples.chunks(self.resolution) {
            if peak_idx >= peaks_count {
                break;
            }
            if chunk.is_empty() {
                continue;
            }

            let mut min = f32::INFINITY;
            let mut max = f32::NEG_INFINITY;
            let mut sum_sq = 0.0;

            for &s in chunk {
                min = min.min(s);
                max = max.max(s);
                sum_sq += s * s;
            }

            waveform.peaks[peak_idx] = WaveformPeak {
                min,
                max,
                rms: (sum_sq / chunk.len() as f32).sqrt(),
            };
            waveform.rms[peak_idx] = (sum_sq / chunk.len() as f32).sqrt();
            peak_idx += 1;
        }

        waveform
    }
}

/// Multi-resolution waveform for zoom levels
#[derive(Debug, Clone)]
pub struct MultiResolutionWaveform {
    pub base: Waveform,
    pub levels: Vec<WaveformLevel>,
}

#[derive(Debug, Clone)]
pub struct WaveformLevel {
    pub zoom: f32, // 1.0 = base, 2.0 = 2x zoom
    pub peaks: Vec<WaveformPeak>,
    pub rms: Vec<f32>,
}

impl MultiResolutionWaveform {
    pub fn from_waveform(waveform: Waveform, max_levels: usize) -> Self {
        let mut levels = Vec::new();
        let mut current = waveform.peaks.clone();

        for level in 1..max_levels {
            let zoom = 2.0_f32.powi(level as i32);
            let mut downsampled = Vec::new();
            let mut rms_downsampled = Vec::new();

            for chunk in current.chunks(2) {
                if chunk.len() == 2 {
                    downsampled.push(WaveformPeak {
                        min: chunk[0].min.min(chunk[1].min),
                        max: chunk[0].max.max(chunk[1].max),
                        rms: ((chunk[0].rms * chunk[0].rms + chunk[1].rms * chunk[1].rms) / 2.0)
                            .sqrt(),
                    });
                    rms_downsampled.push(
                        ((chunk[0].rms * chunk[0].rms + chunk[1].rms * chunk[1].rms) / 2.0).sqrt(),
                    );
                } else if chunk.len() == 1 {
                    downsampled.push(chunk[0]);
                    rms_downsampled.push(chunk[0].rms);
                }
            }

            levels.push(WaveformLevel {
                zoom,
                peaks: downsampled.clone(),
                rms: rms_downsampled,
            });
            current = downsampled;
        }

        Self {
            base: waveform,
            levels,
        }
    }

    pub fn get_level(&self, zoom: f32) -> &WaveformLevel {
        let level = (zoom.log2().floor() as usize).min(self.levels.len() - 1);
        &self.levels[level]
    }
}

/// Waveform cache
pub struct WaveformCache {
    cache: std::collections::HashMap<ClipId, Arc<Waveform>>,
    multi_cache: std::collections::HashMap<ClipId, Arc<MultiResolutionWaveform>>,
    max_entries: usize,
}

impl WaveformCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: std::collections::HashMap::new(),
            multi_cache: std::collections::HashMap::new(),
            max_entries,
        }
    }

    pub fn get(&self, clip_id: ClipId) -> Option<Arc<Waveform>> {
        self.cache.get(&clip_id).cloned()
    }

    pub fn get_multi(&self, clip_id: ClipId) -> Option<Arc<MultiResolutionWaveform>> {
        self.multi_cache.get(&clip_id).cloned()
    }

    pub fn put(&mut self, clip_id: ClipId, waveform: Waveform) {
        if self.cache.len() >= self.max_entries {
            // Simple eviction - remove first
            if let Some(key) = self.cache.keys().next().cloned() {
                self.cache.remove(&key);
                self.multi_cache.remove(&key);
            }
        }

        let multi = MultiResolutionWaveform::from_waveform(waveform.clone(), 5);
        self.cache.insert(clip_id, Arc::new(waveform));
        self.multi_cache.insert(clip_id, Arc::new(multi));
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.multi_cache.clear();
    }
}

fn frame_to_sample(frame: Frame, sample_rate: u32, fps: FrameRate) -> usize {
    let r = fps.to_rational();
    ((frame.0 as f64 * sample_rate as f64 * r.den as f64 / r.num as f64) as usize).max(0)
}

fn sample_to_frame(sample: usize, sample_rate: u32, fps: FrameRate) -> Frame {
    let r = fps.to_rational();
    Frame((sample as f64 * r.num as f64 / (sample_rate as f64 * r.den as f64)) as i64)
}
