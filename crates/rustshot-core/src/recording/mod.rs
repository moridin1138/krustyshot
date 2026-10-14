//! Screen recording: capture frames from one display, crop them to a region,
//! and encode them to MP4 (the OS's own encoder) or GIF (everywhere).
//!
//! Two threads: one takes frames from the display at the chosen frame rate
//! and crops them; the other encodes. If the encoder falls behind, frames are
//! dropped (and counted) rather than piling up in memory.

mod gif;
#[cfg(target_os = "macos")]
mod mp4_macos;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};

use crate::config::{RecordingSettings, VideoFormat};
use crate::disk;

/// One cropped frame, tightly packed RGBA.
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Time since the recording started.
    pub pts: Duration,
}

/// Something that turns frames into a file.
pub trait VideoEncoder: Send {
    fn push(&mut self, frame: &VideoFrame) -> Result<()>;
    fn finish(self: Box<Self>) -> Result<()>;
}

/// Whether this platform can record `format`.
pub fn supports(format: VideoFormat) -> bool {
    match format {
        VideoFormat::Gif => true,
        VideoFormat::Mp4 => cfg!(target_os = "macos"),
    }
}

/// Pixel area of a display to record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StopReason {
    /// The user stopped it.
    User,
    /// Free space ran critically low; what was recorded is kept.
    DiskFull,
    /// Cancelled: the file is deleted.
    Cancelled,
    Error(String),
}

/// Live numbers for the recording bar.
#[derive(Default)]
pub struct Stats {
    pub frames: AtomicU64,
    pub dropped: AtomicU64,
}

pub struct RecordingResult {
    pub path: PathBuf,
    pub duration: Duration,
    pub frames: u64,
    pub dropped: u64,
    pub reason: StopReason,
}

/// A recording in progress.
pub struct Recording {
    stop: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    pub stats: Arc<Stats>,
    pub started: Instant,
    pub path: PathBuf,
    reason: Arc<Mutex<Option<StopReason>>>,
    capture: Option<JoinHandle<Result<()>>>,
    encode: Option<JoinHandle<Result<()>>>,
}

impl Recording {
    /// Start recording `crop` of the display `monitor_id` into `path`.
    pub fn start(monitor_id: u32, crop: CropRect, scale: f64, settings: &RecordingSettings, path: &Path) -> Result<Self> {
        if !supports(settings.format) {
            bail!("{} recording isn't available on this platform yet", settings.format.extension().to_uppercase());
        }
        if crop.width < 2 || crop.height < 2 {
            bail!("the recording area is too small");
        }
        let (out_w, out_h) = settings.output_size(crop.width, crop.height, scale);
        let fps = settings.effective_fps();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut encoder: Box<dyn VideoEncoder> = match settings.format {
            VideoFormat::Gif => Box::new(gif::GifEncoder::create(path, out_w, out_h)?),
            #[cfg(target_os = "macos")]
            VideoFormat::Mp4 => Box::new(mp4_macos::Mp4Encoder::create(
                path,
                (crop.width, crop.height),
                (out_w, out_h),
                fps,
                settings.bit_rate(out_w, out_h),
            )?),
            #[cfg(not(target_os = "macos"))]
            VideoFormat::Mp4 => unreachable!(),
        };
        log::info!(
            "recording {}x{} crop of display {monitor_id} to {}x{} {} at {fps} fps -> {}",
            crop.width,
            crop.height,
            out_w,
            out_h,
            settings.format.extension(),
            path.display()
        );

        let stop = Arc::new(AtomicBool::new(false));
        let cancel = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(Stats::default());
        let reason = Arc::new(Mutex::new(None));
        let started = Instant::now();
        let (tx, rx) = sync_channel::<VideoFrame>(4);

        // Capture thread. The display is looked up inside it, because display
        // handles can't move between threads on every platform; it reports
        // back once recording has really started (or why it couldn't).
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        let capture = {
            let (stop, stats, reason) = (stop.clone(), stats.clone(), reason.clone());
            let out_dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
            std::thread::Builder::new().name("rustshot-capture".into()).spawn(move || -> Result<()> {
                let started_recorder = (|| -> Result<_> {
                    let monitor = xcap::Monitor::all()?
                        .into_iter()
                        .find(|m| m.id().ok() == Some(monitor_id))
                        .context("that display is no longer connected")?;
                    let (recorder, frames) = monitor.video_recorder()?;
                    recorder.start()?;
                    Ok((recorder, frames))
                })();
                let (recorder, frames) = match started_recorder {
                    Ok(r) => {
                        let _ = ready_tx.send(Ok(()));
                        r
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(format!("{e:#}")));
                        return Err(e);
                    }
                };
                let interval = Duration::from_secs_f64(1.0 / fps as f64);
                let mut next_due = Duration::ZERO;
                let mut last_disk_check = Instant::now();
                while !stop.load(Ordering::Acquire) {
                    if last_disk_check.elapsed() > Duration::from_secs(2) {
                        last_disk_check = Instant::now();
                        if disk::space_for(&out_dir).is_some_and(|s| s.is_critical()) {
                            log::warn!("recording stopped: disk almost full");
                            *reason.lock().unwrap() = Some(StopReason::DiskFull);
                            stop.store(true, Ordering::Release);
                            break;
                        }
                    }
                    let Ok(frame) = frames.recv_timeout(Duration::from_millis(250)) else { continue };
                    let pts = started.elapsed();
                    // Keep a steady grid at the target rate. Frames arrive at
                    // the display's rate with some jitter, so accept one that
                    // is up to a quarter interval early; if far behind (a
                    // stall), restart the grid rather than bursting.
                    if pts + interval / 4 < next_due {
                        continue;
                    }
                    next_due += interval;
                    if next_due + interval < pts {
                        next_due = pts + interval;
                    }
                    match crop_frame(&frame.raw, frame.width, frame.height, crop) {
                        Some(rgba) => {
                            let vf = VideoFrame { width: crop.width, height: crop.height, rgba, pts };
                            match tx.try_send(vf) {
                                Ok(()) => {}
                                Err(TrySendError::Full(_)) => {
                                    stats.dropped.fetch_add(1, Ordering::Relaxed);
                                }
                                Err(TrySendError::Disconnected(_)) => break,
                            }
                        }
                        None => log::warn!("display size changed while recording; frame skipped"),
                    }
                }
                let _ = recorder.stop();
                Ok(())
            })?
        };

        // Encoder thread.
        let encode = {
            let stats = stats.clone();
            std::thread::Builder::new().name("rustshot-encode".into()).spawn(move || -> Result<()> {
                for frame in rx {
                    encoder.push(&frame)?;
                    stats.frames.fetch_add(1, Ordering::Relaxed);
                }
                encoder.finish()
            })?
        };

        let failure = match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(format!("couldn't start recording: {e}")),
            Err(_) => Some("the display didn't start sending frames".to_string()),
        };
        if let Some(message) = failure {
            // Let both threads end before removing the file, so the encoder
            // can't recreate it afterwards.
            stop.store(true, Ordering::Release);
            let _ = capture.join();
            let _ = encode.join();
            let _ = std::fs::remove_file(path);
            bail!(message);
        }

        Ok(Self { stop, cancel, stats, started, path: path.to_path_buf(), reason, capture: Some(capture), encode: Some(encode) })
    }

    /// True once the recording has stopped by itself (e.g. disk full).
    pub fn stopped_by_itself(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Size of the file so far, where the encoder writes as it goes.
    pub fn bytes_so_far(&self) -> u64 {
        std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }

    /// Stop, finish the file and report. With `discard`, the file is deleted.
    pub fn finish(mut self, discard: bool) -> RecordingResult {
        if discard {
            self.cancel.store(true, Ordering::Release);
        }
        self.stop.store(true, Ordering::Release);
        let duration = self.started.elapsed();
        let join = |h: Option<JoinHandle<Result<()>>>| -> Result<()> {
            h.map(|h| h.join().map_err(|_| anyhow!("recording thread panicked"))?).unwrap_or(Ok(()))
        };
        let capture = join(self.capture.take());
        let encode = join(self.encode.take());
        let mut reason = self.reason.lock().unwrap().take().unwrap_or(StopReason::User);
        if let Err(e) = capture.and(encode) {
            log::error!("recording failed: {e:#}");
            reason = StopReason::Error(format!("{e:#}"));
        }
        if self.cancel.load(Ordering::Acquire) {
            let _ = std::fs::remove_file(&self.path);
            reason = StopReason::Cancelled;
        }
        RecordingResult {
            path: self.path.clone(),
            duration,
            frames: self.stats.frames.load(Ordering::Relaxed),
            dropped: self.stats.dropped.load(Ordering::Relaxed),
            reason,
        }
    }
}

/// Copy the `crop` area out of a full-display RGBA frame.
fn crop_frame(raw: &[u8], width: u32, height: u32, crop: CropRect) -> Option<Vec<u8>> {
    if crop.x + crop.width > width || crop.y + crop.height > height || raw.len() < (width * height * 4) as usize {
        return None;
    }
    let row = crop.width as usize * 4;
    let mut out = Vec::with_capacity(row * crop.height as usize);
    for y in crop.y..crop.y + crop.height {
        let start = (y as usize * width as usize + crop.x as usize) * 4;
        out.extend_from_slice(&raw[start..start + row]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_copies_the_right_pixels() {
        // 4x3 frame where each pixel's red channel is its index.
        let raw: Vec<u8> = (0..12u8).flat_map(|i| [i, 0, 0, 255]).collect();
        let out = crop_frame(&raw, 4, 3, CropRect { x: 1, y: 1, width: 2, height: 2 }).unwrap();
        let reds: Vec<u8> = out.chunks(4).map(|p| p[0]).collect();
        assert_eq!(reds, vec![5, 6, 9, 10]);
        assert!(crop_frame(&raw, 4, 3, CropRect { x: 3, y: 0, width: 2, height: 1 }).is_none());
    }
}
