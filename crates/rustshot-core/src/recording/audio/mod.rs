//! Recording audio: the microphone and/or system sound, mixed into one
//! 48 kHz stereo track for the MP4.
//!
//! Each source delivers audio on its own thread; it is converted to 48 kHz
//! stereo, placed on the recording's timeline and sent to the encoder
//! thread, which mixes the sources and hands the result to the encoder.

mod mix;
#[cfg(target_os = "macos")]
mod system_macos;

use std::sync::mpsc::Sender;
use std::time::Instant;

use anyhow::Result;

pub use mix::{AudioChunk, Mixer, SourceChunk, SourceClock, RATE};

/// A microphone the user can pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Microphone {
    /// Stable system ID, stored in the settings.
    pub id: String,
    pub name: String,
}

/// Microphones connected now. Empty where audio isn't supported.
pub fn microphones() -> Vec<Microphone> {
    #[cfg(any(target_os = "macos", windows))]
    {
        use cpal::traits::{DeviceTrait, HostTrait};
        let Ok(devices) = cpal::default_host().input_devices() else { return Vec::new() };
        devices
            .filter_map(|d| {
                let id = d.id().ok()?.to_string();
                let name = d.description().ok()?.name().to_string();
                Some(Microphone { id, name })
            })
            .collect()
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    Vec::new()
}

/// Whether recordings can include sound at all (MP4 on macOS and Windows).
pub fn supported() -> bool {
    cfg!(any(target_os = "macos", windows))
}

/// Whether system sound can be recorded: on Windows always, on macOS from
/// version 13 (ScreenCaptureKit audio).
pub fn system_audio_supported() -> bool {
    #[cfg(target_os = "macos")]
    return system_macos::supported();
    #[cfg(windows)]
    return true;
    #[cfg(not(any(target_os = "macos", windows)))]
    false
}

/// What to record.
#[derive(Clone, Debug, Default)]
pub struct AudioSources {
    /// `Some(None)` for the default microphone, `Some(Some(id))` for a chosen one.
    pub microphone: Option<Option<String>>,
    pub system: bool,
}

impl AudioSources {
    pub fn any(&self) -> bool {
        self.microphone.is_some() || self.system
    }
}

/// Running audio sources. Recording stops when this is dropped. Not `Send`
/// on every platform, so it stays on the thread that started it.
pub struct AudioCapture {
    #[cfg(any(target_os = "macos", windows))]
    _streams: Vec<cpal::Stream>,
    #[cfg(target_os = "macos")]
    _system: Option<system_macos::SystemAudio>,
}

impl AudioCapture {
    /// Start the chosen sources, sending audio placed relative to `started`.
    /// A source that can't start is logged and skipped, so a missing
    /// microphone doesn't stop the screen recording; an error is returned
    /// only if every chosen source failed.
    pub fn start(sources: &AudioSources, started: Instant, out: Sender<SourceChunk>) -> Result<Self> {
        #[cfg(any(target_os = "macos", windows))]
        {
            let mut streams = Vec::new();
            let mut errors = Vec::new();
            if let Some(device) = &sources.microphone {
                match cpal_source::microphone(device.as_deref(), started, out.clone()) {
                    Ok(s) => streams.push(s),
                    Err(e) => errors.push(format!("microphone: {e:#}")),
                }
            }
            #[cfg(windows)]
            if sources.system {
                match cpal_source::loopback(started, out.clone()) {
                    Ok(s) => streams.push(s),
                    Err(e) => errors.push(format!("system sound: {e:#}")),
                }
            }
            #[cfg(target_os = "macos")]
            let system = if sources.system {
                match system_macos::SystemAudio::start(started, out.clone()) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        errors.push(format!("system sound: {e:#}"));
                        None
                    }
                }
            } else {
                None
            };
            for e in &errors {
                log::warn!("audio source unavailable: {e}");
            }
            #[cfg(target_os = "macos")]
            let none_started = streams.is_empty() && system.is_none();
            #[cfg(windows)]
            let none_started = streams.is_empty();
            if none_started && !errors.is_empty() {
                anyhow::bail!("no sound could be recorded ({})", errors.join("; "));
            }
            Ok(Self {
                _streams: streams,
                #[cfg(target_os = "macos")]
                _system: system,
            })
        }
        #[cfg(not(any(target_os = "macos", windows)))]
        {
            let _ = (sources, started, out);
            anyhow::bail!("recording sound isn't available on this platform")
        }
    }
}

/// Sends one source's audio, converted and placed on the timeline.
pub(crate) struct SourceSender {
    started: Instant,
    clock: SourceClock,
    out: Sender<SourceChunk>,
}

impl SourceSender {
    pub(crate) fn new(started: Instant, out: Sender<SourceChunk>) -> Self {
        Self { started, clock: SourceClock::default(), out }
    }

    /// `samples` interleaved, `channels` per frame, at `rate` Hz.
    pub(crate) fn send(&mut self, samples: &[f32], channels: usize, rate: u32) {
        let frames = mix::to_stereo_48k(samples, channels, rate);
        if frames.is_empty() {
            return;
        }
        let start = self.clock.place(self.started.elapsed(), frames.len());
        let _ = self.out.send(SourceChunk { start, frames });
    }
}

#[cfg(any(target_os = "macos", windows))]
mod cpal_source {
    use std::sync::mpsc::Sender;
    use std::time::Instant;

    use anyhow::{Context, Result};
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{Device, FromSample, SampleFormat, SizedSample, Stream, SupportedStreamConfig};

    use super::{SourceChunk, SourceSender};

    pub fn microphone(id: Option<&str>, started: Instant, out: Sender<SourceChunk>) -> Result<Stream> {
        let host = cpal::default_host();
        let chosen = id.and_then(|id| id.parse().ok()).and_then(|id| host.device_by_id(&id));
        if id.is_some() && chosen.is_none() {
            log::warn!("the chosen microphone isn't connected; using the default one");
        }
        let device = chosen.or_else(|| host.default_input_device()).context("no microphone found")?;
        let config = device.default_input_config().context("reading the microphone's format")?;
        stream(&device, config, started, out)
    }

    /// Windows: what the speakers play, by recording from the output device.
    #[cfg(windows)]
    pub fn loopback(started: Instant, out: Sender<SourceChunk>) -> Result<Stream> {
        let device = cpal::default_host().default_output_device().context("no speakers or headphones found")?;
        let config = device.default_output_config().context("reading the output format")?;
        stream(&device, config, started, out)
    }

    fn stream(device: &Device, config: SupportedStreamConfig, started: Instant, out: Sender<SourceChunk>) -> Result<Stream> {
        let s = match config.sample_format() {
            SampleFormat::F32 => build::<f32>(device, &config, started, out),
            SampleFormat::I16 => build::<i16>(device, &config, started, out),
            SampleFormat::I32 => build::<i32>(device, &config, started, out),
            SampleFormat::U16 => build::<u16>(device, &config, started, out),
            SampleFormat::U8 => build::<u8>(device, &config, started, out),
            other => anyhow::bail!("unsupported sample format {other}"),
        }?;
        s.play().context("starting the audio stream")?;
        Ok(s)
    }

    fn build<T>(device: &Device, config: &SupportedStreamConfig, started: Instant, out: Sender<SourceChunk>) -> Result<Stream>
    where
        T: SizedSample,
        f32: FromSample<T>,
    {
        let channels = config.channels() as usize;
        let rate = config.sample_rate();
        let mut sender = SourceSender::new(started, out);
        let mut buf: Vec<f32> = Vec::new();
        let stream = device.build_input_stream::<T, _, _>(
            config.config(),
            move |data: &[T], _| {
                buf.clear();
                buf.extend(data.iter().map(|s| s.to_sample::<f32>()));
                sender.send(&buf, channels, rate);
            },
            |e| log::warn!("audio stream error: {e}"),
            None,
        )?;
        Ok(stream)
    }
}
