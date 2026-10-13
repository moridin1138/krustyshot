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

/// Grab all monitors and, optionally, the window list.
pub fn take_snapshot(with_windows: bool) -> Result<Snapshot> {
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
        let image = m.capture_image().context("capturing monitor")?;
        shots.push(MonitorShot {
            id: m.id().unwrap_or_default(),
            name: m.friendly_name().or_else(|_| m.name()).unwrap_or_default(),
            is_primary: m.is_primary().unwrap_or(false),
            rect: r.logical,
            image,
        });
    }

    let windows = if with_windows { list_windows(&raw) } else { Vec::new() };
    Ok(Snapshot { monitors: shots, windows, taken_at })
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

    /// Crop a global region out of the frozen screen. Regions spanning
    /// several monitors are clipped to the monitor they overlap most.
    pub fn crop(&self, region: &Region) -> Option<RgbaImage> {
        let m = self.monitor_for(region)?;
        crop_monitor(m, region)
    }

    /// All monitors stitched into one image at the highest pixel density.
    pub fn stitch_all(&self) -> Option<RgbaImage> {
        stitch(&self.monitors)
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

fn stitch(monitors: &[MonitorShot]) -> Option<RgbaImage> {
    let first = monitors.first()?;
    if monitors.len() == 1 {
        return Some(first.image.clone());
    }
    let scale = monitors.iter().map(|m| m.scale()).fold(1.0, f64::max);
    let min_x = monitors.iter().map(|m| m.rect.x).fold(f64::MAX, f64::min);
    let min_y = monitors.iter().map(|m| m.rect.y).fold(f64::MAX, f64::min);
    let max_x = monitors.iter().map(|m| m.rect.right()).fold(f64::MIN, f64::max);
    let max_y = monitors.iter().map(|m| m.rect.bottom()).fold(f64::MIN, f64::max);
    let w = ((max_x - min_x) * scale).round() as u32;
    let h = ((max_y - min_y) * scale).round() as u32;
    let mut canvas = RgbaImage::new(w, h);
    for m in monitors {
        let tw = (m.rect.w * scale).round() as u32;
        let th = (m.rect.h * scale).round() as u32;
        let ox = ((m.rect.x - min_x) * scale).round() as i64;
        let oy = ((m.rect.y - min_y) * scale).round() as i64;
        if tw == m.image.width() && th == m.image.height() {
            imageops::overlay(&mut canvas, &m.image, ox, oy);
        } else {
            let resized = imageops::resize(&m.image, tw, th, imageops::FilterType::Triangle);
            imageops::overlay(&mut canvas, &resized, ox, oy);
        }
    }
    Some(canvas)
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

    #[test]
    fn stitch_mixed_dpi() {
        let img = stitch(&[shot(0.0, 100.0, 2), shot(100.0, 100.0, 1)]).unwrap();
        assert_eq!((img.width(), img.height()), (400, 200));
    }
}
