//! Audio output device facade.
//!
//! Wraps `cpal` so the rest of the audio engine never names the backend
//! directly. With the `audio-cpal` feature off, the types still exist and the
//! process compiles, but starting a stream is a no-op — enough for headless
//! rendering, tests, and CI.

use std::sync::{Arc, Mutex};

/// Output stream parameters, in a backend-independent form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: u32,
}

impl StreamConfig {
    pub fn new(sample_rate: u32, channels: u16, buffer_size: u32) -> Self {
        Self {
            sample_rate,
            channels,
            buffer_size,
        }
    }

    /// Bytes per callback for interleaved `f32` samples.
    pub fn bytes_per_callback(&self) -> usize {
        self.buffer_size as usize * self.channels as usize * std::mem::size_of::<f32>()
    }
}

/// A running output stream.
///
/// With `audio-cpal` on, this holds the live `cpal::Stream` (which must be kept
/// alive for playback to continue). With it off, the handle is inert.
pub struct Stream {
    #[cfg(feature = "audio-cpal")]
    inner: cpal::Stream,
    config: StreamConfig,
    running: Arc<Mutex<bool>>,
}

impl Stream {
    /// The configuration this stream was created with.
    pub fn config(&self) -> &StreamConfig {
        &self.config
    }

    /// True while the stream is meant to be producing audio.
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Begin playback. A no-op without a real backend.
    pub fn play(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(feature = "audio-cpal")]
        {
            use cpal::traits::StreamTrait;
            self.inner
                .play()
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
        }
        *self.running.lock().unwrap_or_else(|e| e.into_inner()) = true;
        Ok(())
    }

    /// Pause playback. A no-op without a real backend.
    pub fn pause(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(feature = "audio-cpal")]
        {
            use cpal::traits::StreamTrait;
            self.inner
                .pause()
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
        }
        *self.running.lock().unwrap_or_else(|e| e.into_inner()) = false;
        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        *self.running.lock().unwrap_or_else(|e| e.into_inner()) = false;
    }
}

/// Open the default output device and start an f32 stream that pulls from
/// `source` on every callback.
///
/// `source` is invoked with the number of `f32` samples to produce and must fill
/// it. With `audio-cpal` off this succeeds but produces no sound.
pub fn open_output_stream<F>(
    config: StreamConfig,
    source: F,
) -> Result<Stream, Box<dyn std::error::Error>>
where
    F: Fn(&mut [f32]) + Send + Sync + 'static,
{
    let running = Arc::new(Mutex::new(false));

    #[cfg(feature = "audio-cpal")]
    {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no audio output device available")?;
        let device_config = device
            .default_output_config()
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

        // Only f32 output is supported; convert the device format if needed.
        if device_config.sample_format() != cpal::SampleFormat::F32 {
            return Err("audio device does not support f32 samples".into());
        }

        let source = Arc::new(source);
        let stream = device
            .build_output_stream(
                &device_config.config(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Zero first so a short fill cannot leave stale audio.
                    data.iter_mut().for_each(|s| *s = 0.0);
                    source(data);
                },
                |err| eprintln!("audio stream error: {err}"),
                None,
            )
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

        stream
            .play()
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
        *running.lock().unwrap_or_else(|e| e.into_inner()) = true;

        return Ok(Stream {
            inner: stream,
            config,
            running,
        });
    }

    #[cfg(not(feature = "audio-cpal"))]
    {
        let _ = source;
        // No backend, so nothing is audible; report "not running" rather than
        // pretending otherwise.
        *running.lock().unwrap_or_else(|e| e.into_inner()) = false;
        Ok(Stream { config, running })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_size_accounts_for_channels() {
        let c = StreamConfig::new(48_000, 2, 512);
        assert_eq!(c.bytes_per_callback(), 512 * 2 * 4);
    }

    #[test]
    fn headless_stream_can_be_created_and_toggled() {
        let stream = open_output_stream(StreamConfig::new(48_000, 2, 256), |_| {}).unwrap();
        assert_eq!(stream.config().sample_rate, 48_000);

        // Without `audio-cpal` there is no sound card, so the stream starts
        // idle; with a backend `open_output_stream` plays immediately.
        stream.pause().unwrap();
        assert!(!stream.is_running());
        stream.play().unwrap();
        assert!(stream.is_running());
    }
}
