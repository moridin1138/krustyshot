//! Scrolling capture: scroll an area of the screen automatically and join
//! the screenshots into one tall image.
//!
//! A background thread repeats: put the pointer in the middle of the area
//! (so a nudged mouse can't send the scrolling elsewhere), scroll down,
//! wait for the page to settle, capture, and add the rows that came into
//! view ([`stitch`]). It stops at the bottom (nothing moves any more), when
//! a mouse button is pressed, when asked to, or at a size limit. The
//! pointer goes back where it was.

pub mod input;
pub mod stitch;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use image::RgbaImage;

use crate::pointer::Probe;
use crate::recording::CropRect;
use input::Scroller;
use stitch::{Step, Stitcher};

/// The image stops growing at this height (pixels), so endless pages end.
pub const MAX_HEIGHT: u32 = 25_000;
/// Aim to scroll this share of the area each step: enough overlap to match.
const STEP_SHARE: f64 = 0.6;
/// How long a page gets to finish scrolling (smooth scrolling animates).
const SETTLE_MAX: Duration = Duration::from_millis(1500);
const SETTLE_POLL: Duration = Duration::from_millis(60);

/// Why a scrolling capture ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    /// The page stopped moving.
    Bottom,
    /// A mouse button, the pointer moving away, or [`ScrollCapture::stop`].
    Stopped,
    /// [`MAX_HEIGHT`] reached.
    Limit,
    /// The frames stopped lining up (the page jumped or changed).
    LostTrack,
}

pub struct Outcome {
    pub image: RgbaImage,
    pub reason: EndReason,
}

/// A running scrolling capture.
pub struct ScrollCapture {
    stop: Arc<AtomicBool>,
    height: Arc<Mutex<u32>>,
    thread: Option<JoinHandle<Result<Outcome>>>,
}

impl ScrollCapture {
    /// Start capturing `crop` (pixels) of the display `monitor_id`, whose
    /// scale is `scale` pixels per point.
    pub fn start(monitor_id: u32, crop: CropRect, scale: f64) -> Result<Self> {
        if crop.width < 8 || crop.height < 40 {
            return Err(anyhow!("the area is too small to scroll"));
        }
        let stop = Arc::new(AtomicBool::new(false));
        let height = Arc::new(Mutex::new(crop.height));
        let thread = {
            let (stop, height) = (stop.clone(), height.clone());
            std::thread::Builder::new().name("rustshot-scroll".into()).spawn(move || run(monitor_id, crop, scale, &stop, &height))?
        };
        Ok(Self { stop, height, thread: Some(thread) })
    }

    /// Height of the image so far, in pixels.
    pub fn height(&self) -> u32 {
        *self.height.lock().unwrap()
    }

    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_none_or(|t| t.is_finished())
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }

    /// The result, once [`Self::is_finished`]. Blocks otherwise.
    pub fn finish(mut self) -> Result<Outcome> {
        self.stop();
        let thread = self.thread.take().context("already finished")?;
        thread.join().map_err(|_| anyhow!("the scrolling capture crashed"))?
    }
}

impl Drop for ScrollCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(monitor_id: u32, crop: CropRect, scale: f64, stop: &AtomicBool, height: &Mutex<u32>) -> Result<Outcome> {
    let monitor = xcap::Monitor::all()?
        .into_iter()
        .find(|m| m.id().ok() == Some(monitor_id))
        .context("that display is no longer connected")?;
    let mut scroller = Scroller::new()?;
    let mut probe = Probe::new();

    // The middle of the area, in pointer coordinates (points on macOS,
    // pixels elsewhere).
    let factor = if cfg!(target_os = "macos") { scale.max(1.0) } else { 1.0 };
    let (mx, my) = (monitor.x().unwrap_or(0) as f64, monitor.y().unwrap_or(0) as f64);
    let target = (mx + (crop.x as f64 + crop.width as f64 / 2.0) / factor, my + (crop.y as f64 + crop.height as f64 / 2.0) / factor);

    // Let go of the button that finished selecting the area first.
    let released = Instant::now();
    while probe.sample().is_some_and(|(_, down)| down) && released.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(20));
    }
    let original = probe.sample().map(|(p, _)| p);
    scroller.move_to(target.0, target.1);
    std::thread::sleep(Duration::from_millis(150));

    let grab = || -> Result<RgbaImage> {
        // Without the pointer, which sits in the middle of the area.
        let full = crate::capture::monitor_image(&monitor)?;
        let (w, h) = full.dimensions();
        if crop.x + crop.width > w || crop.y + crop.height > h {
            return Err(anyhow!("the display changed size"));
        }
        Ok(image::imageops::crop_imm(&full, crop.x, crop.y, crop.width, crop.height).to_image())
    };

    let first = grab()?;
    log::info!("scrolling capture: {}x{} px area, scale {scale}", first.width(), first.height());
    // The last frame added, to tell when the next scroll has begun.
    let mut last = first.clone();
    let mut stitcher = Stitcher::new(first);
    // Scroll amount per step: points on macOS; wheel notches elsewhere, with
    // the pixels per notch learnt as it goes.
    let want = (crop.height as f64 * STEP_SHARE).max(20.0);
    // Start with one notch to learn its size (it depends on system settings).
    let mut px_per_notch = want;
    let (mut unchanged, mut lost) = (0, 0);

    // Stop on the Stop button or any mouse button.
    let stopped = |probe: &mut Probe| -> Option<&'static str> {
        if stop.load(Ordering::Acquire) {
            return Some("Stop was clicked");
        }
        probe.sample().is_some_and(|(_, down)| down).then_some("a mouse button was pressed")
    };

    let reason = loop {
        if let Some(why) = stopped(&mut probe) {
            log::info!("scrolling capture: stopping, {why}");
            break EndReason::Stopped;
        }
        if stitcher.height() >= MAX_HEIGHT {
            break EndReason::Limit;
        }
        // After losing track, look again without scrolling further, which
        // would only widen the gap.
        let (amount, expected) = if lost > 0 {
            (0, 0)
        } else if Scroller::scrolls_by_points() {
            let points = (want / scale.max(1.0)).round().max(1.0) as u32;
            (points, (points as f64 * scale.max(1.0)) as u32)
        } else {
            let notches = (want / px_per_notch).round().max(1.0) as u32;
            (notches, (notches as f64 * px_per_notch) as u32)
        };
        if amount > 0 {
            scroller.move_to(target.0, target.1);
            scroller.scroll_down(amount);
        }

        // Wait for the screen to change (an app can take a moment to start
        // scrolling), then until two captures in a row agree (smooth
        // scrolling has finished). If it never changes, that's the bottom.
        let started = Instant::now();
        std::thread::sleep(SETTLE_POLL);
        let mut frame = grab()?;
        let mut changed = frame != last;
        let mut user_stopped = false;
        while started.elapsed() < SETTLE_MAX {
            std::thread::sleep(SETTLE_POLL);
            if let Some(why) = stopped(&mut probe) {
                log::info!("scrolling capture: stopping, {why}");
                user_stopped = true;
                break;
            }
            let next = grab()?;
            let settled = changed && next == frame;
            changed |= next != last;
            frame = next;
            if settled {
                break;
            }
        }
        if user_stopped {
            break EndReason::Stopped;
        }

        let step = stitcher.add(frame.clone(), expected);
        log::info!(
            "scrolling capture: scrolled {amount}{}, expected ~{expected} px, {step:?} after {:?}, height {} ({})",
            if Scroller::scrolls_by_points() { " pt" } else { " notches" },
            started.elapsed(),
            stitcher.height(),
            stitcher.detail
        );
        if step != Step::Lost {
            last = frame;
        }
        match step {
            Step::Moved(px) => {
                unchanged = 0;
                lost = 0;
                if !Scroller::scrolls_by_points() && amount > 0 {
                    px_per_notch = (px as f64 / amount as f64).max(4.0);
                }
            }
            Step::Unchanged => {
                // Each step already waits for the page to move, so once is
                // enough: this is the bottom.
                unchanged += 1;
                if unchanged >= 1 {
                    break EndReason::Bottom;
                }
            }
            Step::Lost => {
                lost += 1;
                if lost >= 3 {
                    break EndReason::LostTrack;
                }
            }
        }
        *height.lock().unwrap() = stitcher.height().min(MAX_HEIGHT);
    };

    if let Some((x, y)) = original {
        scroller.move_to(x, y);
    }
    let mut image = stitcher.finish();
    if image.height() > MAX_HEIGHT {
        image = image::imageops::crop_imm(&image, 0, 0, image.width(), MAX_HEIGHT).to_image();
    }
    log::info!("scrolling capture: {}x{} ({reason:?})", image.width(), image.height());
    Ok(Outcome { image, reason })
}
