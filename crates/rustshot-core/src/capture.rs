//! Screen and window capture built on `xcap`.
//!
//! Every capture starts with a [`Snapshot`]: all monitors are grabbed at once
//! (plus the list of visible windows) so the overlay can show a frozen screen,
//! exactly like Greenshot does.

use anyhow::{Context, Result};
use image::{imageops, RgbaImage};

use crate::geometry::Region;

/// One monitor's frozen image and where it sits on the desktop.
pub struct MonitorShot {
    pub id: u32,
    pub name: String,
    pub is_primary: bool,
    /// Bounds in global logical coordinates.
    pub rect: Region,
    /// Captured pixels (physical resolution, e.g. 2x on Retina).
    pub image: RgbaImage,
}

impl MonitorShot {
    /// Image pixels per logical unit (2.0 on a Retina display).
    pub fn scale(&self) -> f64 {
        if self.rect.w <= 0.0 {
            1.0
        } else {
            self.image.width() as f64 / self.rect.w
        }
    }
}

/// A visible top-level window that can be picked in window-capture mode.
#[derive(Clone, Debug)]
pub struct WindowInfo {
    pub title: String,
    pub app_name: String,
    /// Bounds in global logical coordinates.
    pub rect: Region,
}

/// Everything captured at the moment a capture was triggered.
pub struct Snapshot {
    pub monitors: Vec<MonitorShot>,
    /// Windows ordered front to back.
    pub windows: Vec<WindowInfo>,
    /// Pointer position (global logical coordinates) when the snapshot was taken.
    pub pointer: Option<(f64, f64)>,
    pub taken_at: chrono::DateTime<chrono::Local>,
}

/// System UI that should never be offered as a capturable window.
const IGNORED_APPS: &[&str] = &[
    "Window Server",
    "Dock",
    "Control Center",
    "SystemUIServer",
    "Notification Center",
    "Spotlight",
    "TextInputMenuAgent",
];

struct RawMonitor {
    phys: Region,
    scale: f64,
    logical: Region,
}

/// xcap reports logical points on macOS but physical pixels elsewhere; map
/// everything to logical points so it lines up with egui window positions.
fn to_logical(monitors: &[RawMonitor], x: f64, y: f64) -> (f64, f64) {
    let m = monitors
        .iter()
        .find(|m| m.phys.contains(x, y))
        .or_else(|| monitors.first());
    match m {
        Some(m) => (
            m.logical.x + (x - m.phys.x) / m.scale,
            m.logical.y + (y - m.phys.y) / m.scale,
        ),
        None => (x, y),
    }
}

fn platform_scale(monitor: &xcap::Monitor) -> f64 {
    if cfg!(target_os = "macos") {
        1.0
    } else {
        monitor.scale_factor().unwrap_or(1.0).max(0.1) as f64
    }
}

/// What to include in a snapshot besides the monitor images.
#[derive(Clone, Copy, Debug, Default)]
pub struct SnapshotOptions {
    /// List visible windows (needed for window capture and highlighting).
    pub windows: bool,
    /// Draw the mouse pointer into the captured images.
    pub pointer: bool,
}

/// Size of the primary display in pixels, and its pixels per point.
pub fn primary_display_pixels() -> Option<(u32, u32, f64)> {
    let monitors = xcap::Monitor::all().ok()?;
    let m = monitors.iter().find(|m| m.is_primary().unwrap_or(false)).or(monitors.first())?;
    let scale = m.scale_factor().unwrap_or(1.0).max(1.0) as f64;
    let (w, h) = (m.width().ok()?, m.height().ok()?);
    if cfg!(target_os = "macos") {
        // macOS reports points; convert to pixels.
        Some(((w as f64 * scale) as u32, (h as f64 * scale) as u32, scale))
    } else {
        Some((w, h, scale))
    }
}

/// One display's picture, without the mouse pointer. (Rustshot draws the
/// pointer itself when that's asked for.) On recent macOS the classic
/// capture call always includes the pointer, so macOS 14 and later use
/// ScreenCaptureKit's screenshots instead, falling back if that fails.
pub fn monitor_image(m: &xcap::Monitor) -> Result<RgbaImage> {
    #[cfg(target_os = "macos")]
    if crate::sck::screenshots_supported() {
        let attempt = (|| -> Result<RgbaImage> {
            let scale = m.scale_factor()? as f64;
            let (w, h) = ((m.width()? as f64 * scale).round() as u32, (m.height()? as f64 * scale).round() as u32);
            crate::sck::screenshot(m.id()?, w, h)
        })();
        match attempt {
            Ok(image) => return Ok(image),
            Err(e) => {
                static WARNED: std::sync::Once = std::sync::Once::new();
                WARNED.call_once(|| log::warn!("ScreenCaptureKit screenshot failed, using the classic capture: {e:#}"));
            }
        }
    }
    Ok(m.capture_image()?)
}

/// Grab all monitors and, optionally, the window list and mouse pointer.
pub fn take_snapshot(opts: SnapshotOptions) -> Result<Snapshot> {
    let taken_at = chrono::Local::now();
    let monitors = xcap::Monitor::all().context("listing monitors")?;

    let mut raw = Vec::new();
    for m in &monitors {
        let phys = Region::new(
            m.x()? as f64,
            m.y()? as f64,
            m.width()? as f64,
            m.height()? as f64,
        );
        let scale = platform_scale(m);
        raw.push(RawMonitor { phys, scale, logical: phys });
    }
    // Logical origin of each monitor: keep the primary at its physical origin
    // and scale the others relative to their own origin.
    for r in &mut raw {
        r.logical = Region::new(r.phys.x / r.scale, r.phys.y / r.scale, r.phys.w / r.scale, r.phys.h / r.scale);
    }

    let mut shots = Vec::new();
    for (m, r) in monitors.iter().zip(&raw) {
        let image = monitor_image(m).context("capturing monitor")?;
        shots.push(MonitorShot {
            id: m.id().unwrap_or_default(),
            name: m.friendly_name().or_else(|_| m.name()).unwrap_or_default(),
            is_primary: m.is_primary().unwrap_or(false),
            rect: r.logical,
            image,
        });
    }

    let pointer = crate::pointer::raw_position().map(|(x, y)| to_logical(&raw, x, y));
    if opts.pointer
        && let Some((px, py)) = pointer
        && let Some(m) = shots.iter_mut().find(|m| m.rect.contains(px, py))
    {
        let scale = m.scale();
        let x = ((px - m.rect.x) * scale) as f32;
        let y = ((py - m.rect.y) * scale) as f32;
        crate::pointer::draw(&mut m.image, x, y, scale as f32);
    }

    let windows = if opts.windows { list_windows(&raw) } else { Vec::new() };
    Ok(Snapshot { monitors: shots, windows, pointer, taken_at })
}

fn list_windows(raw: &[RawMonitor]) -> Vec<WindowInfo> {
    let own_pid = std::process::id();
    let Ok(windows) = xcap::Window::all() else {
        return Vec::new();
    };
    windows
        .into_iter()
        .filter_map(|w| {
            if w.pid().ok()? == own_pid || w.is_minimized().unwrap_or(false) {
                return None;
            }
            let app_name = w.app_name().unwrap_or_default();
            if IGNORED_APPS.contains(&app_name.as_str()) {
                return None;
            }
            let (x, y) = (w.x().ok()? as f64, w.y().ok()? as f64);
            let (x2, y2) = (x + w.width().ok()? as f64, y + w.height().ok()? as f64);
            let a = to_logical(raw, x, y);
            let b = to_logical(raw, x2, y2);
            let rect = Region::from_points(a, b);
            if rect.w < 24.0 || rect.h < 24.0 {
                return None;
            }
            Some(WindowInfo { title: w.title().unwrap_or_default(), app_name, rect })
        })
        .collect()
}

impl Snapshot {
    /// The monitor that overlaps `region` the most.
    pub fn monitor_for(&self, region: &Region) -> Option<&MonitorShot> {
        self.monitors
            .iter()
            .filter_map(|m| m.rect.intersect(region).map(|i| (m, i.area())))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(m, _)| m)
    }

    /// The monitor containing a global point.
    pub fn monitor_at(&self, x: f64, y: f64) -> Option<&MonitorShot> {
        self.monitors.iter().find(|m| m.rect.contains(x, y))
    }

    /// The front-most window under a global point.
    pub fn window_at(&self, x: f64, y: f64) -> Option<&WindowInfo> {
        self.windows.iter().find(|w| w.rect.contains(x, y))
    }

    /// Crop a global region out of the frozen screen. A region spanning
    /// several monitors is assembled from all of them (see [`compose`]).
    pub fn crop(&self, region: &Region) -> Option<RgbaImage> {
        compose(&self.monitors, region)
    }

    /// What to record for a selected region: the display it overlaps most,
    /// and the region in that display's pixels. (Recordings use one display.)
    pub fn recording_target(&self, region: &Region) -> Option<(u32, crate::recording::CropRect, f64)> {
        let m = self.monitor_for(region)?;
        let r = m.rect.intersect(region)?;
        let s = m.scale();
        let (iw, ih) = m.image.dimensions();
        let x = (((r.x - m.rect.x) * s).round().max(0.0) as u32).min(iw.saturating_sub(2));
        let y = (((r.y - m.rect.y) * s).round().max(0.0) as u32).min(ih.saturating_sub(2));
        let width = ((r.w * s).round() as u32).min(iw - x);
        let height = ((r.h * s).round() as u32).min(ih - y);
        Some((m.id, crate::recording::CropRect { x, y, width, height }, s))
    }

    /// Bounds of the whole desktop.
    pub fn desktop(&self) -> Option<Region> {
        desktop_bounds(&self.monitors)
    }

    /// All monitors stitched into one image at the highest pixel density.
    pub fn stitch_all(&self) -> Option<RgbaImage> {
        compose(&self.monitors, &self.desktop()?)
    }
}

/// Crop a global region from a single monitor image, clipping to its bounds.
pub fn crop_monitor(m: &MonitorShot, region: &Region) -> Option<RgbaImage> {
    let r = m.rect.intersect(region)?;
    let s = m.scale();
    let x = ((r.x - m.rect.x) * s).round().max(0.0) as u32;
    let y = ((r.y - m.rect.y) * s).round().max(0.0) as u32;
    let w = ((r.w * s).round() as u32).min(m.image.width().saturating_sub(x));
    let h = ((r.h * s).round() as u32).min(m.image.height().saturating_sub(y));
    (w > 0 && h > 0).then(|| imageops::crop_imm(&m.image, x, y, w, h).to_image())
}

fn desktop_bounds(monitors: &[MonitorShot]) -> Option<Region> {
    monitors.first()?;
    let min_x = monitors.iter().map(|m| m.rect.x).fold(f64::MAX, f64::min);
    let min_y = monitors.iter().map(|m| m.rect.y).fold(f64::MAX, f64::min);
    let max_x = monitors.iter().map(|m| m.rect.right()).fold(f64::MIN, f64::max);
    let max_y = monitors.iter().map(|m| m.rect.bottom()).fold(f64::MIN, f64::max);
    Some(Region::new(min_x, min_y, max_x - min_x, max_y - min_y))
}

/// Assemble the pixels of a global region from every monitor it touches.
///
/// The result uses the highest pixel density among those monitors, so a
/// region spanning a Retina and a standard display keeps Retina sharpness
/// (the standard display's part is scaled up). Parts of the region not on
/// any monitor stay transparent.
pub fn compose(monitors: &[MonitorShot], region: &Region) -> Option<RgbaImage> {
    let parts: Vec<(&MonitorShot, Region)> =
        monitors.iter().filter_map(|m| m.rect.intersect(region).map(|r| (m, r))).collect();
    match parts.as_slice() {
        [] => None,
        [(m, _)] => crop_monitor(m, region),
        _ => {
            let region = parts.iter().skip(1).fold(parts[0].1, |acc, (_, r)| union(&acc, r));
            let scale = parts.iter().map(|(m, _)| m.scale()).fold(1.0, f64::max);
            let w = (region.w * scale).round() as u32;
            let h = (region.h * scale).round() as u32;
            let mut canvas = RgbaImage::new(w, h);
            for (m, part) in &parts {
                let Some(piece) = crop_monitor(m, part) else { continue };
                let tw = (part.w * scale).round() as u32;
                let th = (part.h * scale).round() as u32;
                let piece = if (tw, th) == piece.dimensions() {
                    piece
                } else {
                    imageops::resize(&piece, tw.max(1), th.max(1), imageops::FilterType::Triangle)
                };
                let ox = ((part.x - region.x) * scale).round() as i64;
                let oy = ((part.y - region.y) * scale).round() as i64;
                imageops::replace(&mut canvas, &piece, ox, oy);
            }
            Some(canvas)
        }
    }
}

fn union(a: &Region, b: &Region) -> Region {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Region::new(x, y, a.right().max(b.right()) - x, a.bottom().max(b.bottom()) - y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(x: f64, w: f64, scale: u32) -> MonitorShot {
        MonitorShot {
            id: 0,
            name: String::new(),
            is_primary: x == 0.0,
            rect: Region::new(x, 0.0, w, 100.0),
            image: RgbaImage::new(w as u32 * scale, 100 * scale),
        }
    }

    #[test]
    fn crop_respects_retina_scale() {
        let m = shot(0.0, 200.0, 2);
        let img = crop_monitor(&m, &Region::new(10.0, 10.0, 50.0, 20.0)).unwrap();
        assert_eq!((img.width(), img.height()), (100, 40));
    }

    #[test]
    fn crop_clips_to_monitor() {
        let m = shot(100.0, 100.0, 1);
        let img = crop_monitor(&m, &Region::new(150.0, 50.0, 500.0, 500.0)).unwrap();
        assert_eq!((img.width(), img.height()), (50, 50));
        assert!(crop_monitor(&m, &Region::new(0.0, 0.0, 50.0, 50.0)).is_none());
    }

    fn filled(x: f64, w: f64, scale: u32, value: u8) -> MonitorShot {
        let mut m = shot(x, w, scale);
        m.image = RgbaImage::from_pixel(w as u32 * scale, 100 * scale, image::Rgba([value, value, value, 255]));
        m
    }

    #[test]
    fn stitch_mixed_dpi() {
        let mons = [shot(0.0, 100.0, 2), shot(100.0, 100.0, 1)];
        let img = compose(&mons, &desktop_bounds(&mons).unwrap()).unwrap();
        assert_eq!((img.width(), img.height()), (400, 200));
    }

    #[test]
    fn recording_target_uses_display_pixels() {
        let snap = Snapshot {
            monitors: vec![shot(0.0, 200.0, 2), shot(200.0, 100.0, 1)],
            windows: Vec::new(),
            pointer: None,
            taken_at: chrono::Local::now(),
        };
        let (id, crop, scale) = snap.recording_target(&Region::new(10.0, 20.0, 50.0, 30.0)).unwrap();
        assert_eq!((id, scale), (0, 2.0));
        assert_eq!((crop.x, crop.y, crop.width, crop.height), (20, 40, 100, 60));
        // Mostly on the second display: clipped to it.
        let (_, crop, scale) = snap.recording_target(&Region::new(190.0, 0.0, 100.0, 50.0)).unwrap();
        assert_eq!(scale, 1.0);
        assert_eq!((crop.x, crop.width), (0, 90));
    }

    #[test]
    fn region_spanning_two_monitors() {
        // Retina (scale 2) on the left, standard on the right.
        let mons = [filled(0.0, 100.0, 2, 10), filled(100.0, 100.0, 1, 200)];
        let img = compose(&mons, &Region::new(80.0, 10.0, 40.0, 20.0)).unwrap();
        // 40x20 points at the highest density (2x).
        assert_eq!(img.dimensions(), (80, 40));
        // Left half from the Retina monitor, right half from the other.
        assert_eq!(img.get_pixel(10, 10).0[0], 10);
        assert_eq!(img.get_pixel(70, 10).0[0], 200);
    }

    #[test]
    fn region_with_gap_outside_monitors() {
        // Monitors of different heights: the area below the short one is transparent.
        let mut tall = filled(100.0, 100.0, 1, 50);
        tall.rect.h = 200.0;
        tall.image = RgbaImage::from_pixel(100, 200, image::Rgba([50, 50, 50, 255]));
        let mons = [filled(0.0, 100.0, 1, 10), tall];
        let img = compose(&mons, &Region::new(50.0, 50.0, 100.0, 100.0)).unwrap();
        assert_eq!(img.dimensions(), (100, 100));
        assert_eq!(img.get_pixel(10, 90).0[3], 0, "below the short monitor stays transparent");
        assert_eq!(img.get_pixel(90, 90).0[0], 50);
    }
}
