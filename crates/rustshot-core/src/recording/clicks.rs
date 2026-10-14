//! "Show mouse clicks": a ring that grows and fades where you click, and a
//! soft dot while a button is held, drawn into recorded frames.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tiny_skia::{FillRule, Paint, PathBuilder, PixmapMut, Stroke, Transform};

use crate::pointer::Probe;

/// How long the expanding ring lasts.
const RING_TIME: Duration = Duration::from_millis(450);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Click {
    /// Position in the recorded area's pixels.
    pub x: f32,
    pub y: f32,
    /// Time since the recording started.
    pub pressed: Duration,
    pub released: Option<Duration>,
}

/// Polls the mouse 100 times a second on its own thread and keeps a short
/// list of recent clicks for the capture thread to draw.
pub struct ClickTracker {
    clicks: Arc<Mutex<Vec<Click>>>,
    /// Pixels per point on the recorded display, for sizing the effect.
    scale: f32,
}

/// Maps raw pointer coordinates to pixels in the recorded area.
#[derive(Clone, Copy)]
pub struct PointerMapping {
    /// Display origin in raw pointer coordinates.
    pub origin: (f64, f64),
    /// Raw pointer units → display pixels (the scale on macOS, where the
    /// pointer is in points; 1 where it is already in pixels).
    pub factor: f64,
    /// Recorded area's offset within the display, in pixels.
    pub crop: (u32, u32),
}

impl PointerMapping {
    fn to_area(self, (x, y): (f64, f64)) -> (f32, f32) {
        (
            ((x - self.origin.0) * self.factor - self.crop.0 as f64) as f32,
            ((y - self.origin.1) * self.factor - self.crop.1 as f64) as f32,
        )
    }
}

impl ClickTracker {
    pub fn start(mapping: PointerMapping, scale: f32, started: Instant, stop: Arc<AtomicBool>) -> Self {
        let clicks = Arc::new(Mutex::new(Vec::<Click>::new()));
        let shared = clicks.clone();
        let _ = std::thread::Builder::new().name("rustshot-clicks".into()).spawn(move || {
            let mut probe = Probe::new();
            let mut was_down = false;
            while !stop.load(Ordering::Acquire) {
                if let Some((pos, down)) = probe.sample() {
                    let now = started.elapsed();
                    let mut list = shared.lock().unwrap();
                    if down && !was_down {
                        let (x, y) = mapping.to_area(pos);
                        list.push(Click { x, y, pressed: now, released: None });
                    } else if !down && was_down && let Some(last) = list.last_mut() {
                        last.released = Some(now);
                    }
                    // Forget clicks that have finished animating.
                    list.retain(|c| c.released.is_none_or(|r| now.saturating_sub(r.max(c.pressed + RING_TIME)) < Duration::from_secs(1)));
                    was_down = down;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        Self { clicks, scale }
    }

    /// Draw the effects visible at `pts` onto an RGBA frame of the recorded area.
    pub fn draw(&self, rgba: &mut [u8], width: u32, height: u32, pts: Duration) {
        let clicks = self.clicks.lock().unwrap().clone();
        if clicks.is_empty() {
            return;
        }
        draw_clicks(rgba, width, height, pts, &clicks, self.scale);
    }
}

/// Pure drawing, separate for testing.
pub fn draw_clicks(rgba: &mut [u8], width: u32, height: u32, pts: Duration, clicks: &[Click], scale: f32) {
    let Some(mut pm) = PixmapMut::from_bytes(rgba, width, height) else { return };
    let (r0, r1) = (9.0 * scale, 30.0 * scale);
    for c in clicks {
        if pts < c.pressed {
            continue;
        }
        let age = pts - c.pressed;
        let held = c.released.is_none_or(|r| pts < r);
        let mut paint = Paint { anti_alias: true, ..Paint::default() };
        if held {
            // Frames are opaque, so straight and premultiplied colours match
            // except for alpha, which tiny-skia handles for us.
            paint.set_color_rgba8(255, 200, 0, 110);
            if let Some(path) = PathBuilder::from_circle(c.x, c.y, r0) {
                pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }
        let t = age.as_secs_f32() / RING_TIME.as_secs_f32();
        if t < 1.0 {
            let eased = 1.0 - (1.0 - t) * (1.0 - t);
            let radius = r0 + (r1 - r0) * eased;
            paint.set_color_rgba8(255, 200, 0, ((1.0 - t) * 230.0) as u8);
            let stroke = Stroke { width: 3.0 * scale, ..Stroke::default() };
            if let Some(path) = PathBuilder::from_circle(c.x, c.y, radius) {
                pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(w: u32, h: u32) -> Vec<u8> {
        (0..w * h).flat_map(|_| [100, 100, 100, 255]).collect()
    }

    #[test]
    fn ring_appears_then_fades_away() {
        let click = Click { x: 50.0, y: 50.0, pressed: Duration::from_millis(100), released: Some(Duration::from_millis(150)) };
        // Before the click: untouched.
        let mut before = grey(100, 100);
        draw_clicks(&mut before, 100, 100, Duration::from_millis(50), &[click], 1.0);
        assert!(before.chunks(4).all(|p| p == [100, 100, 100, 255]));
        // Mid-animation: some yellowish pixels.
        let mut mid = grey(100, 100);
        draw_clicks(&mut mid, 100, 100, Duration::from_millis(250), &[click], 1.0);
        assert!(mid.chunks(4).any(|p| p[0] > 150 && p[2] < 90));
        // Long after: untouched again.
        let mut after = grey(100, 100);
        draw_clicks(&mut after, 100, 100, Duration::from_millis(900), &[click], 1.0);
        assert!(after.chunks(4).all(|p| p == [100, 100, 100, 255]));
    }

    #[test]
    fn held_button_shows_a_dot() {
        let click = Click { x: 20.0, y: 20.0, pressed: Duration::ZERO, released: None };
        let mut frame = grey(40, 40);
        draw_clicks(&mut frame, 40, 40, Duration::from_secs(2), &[click], 1.0);
        let centre = &frame[(20 * 40 + 20) * 4..][..4];
        assert!(centre[0] > 130 && centre[2] < 100, "{centre:?}");
    }

    #[test]
    fn mapping_from_points_to_area_pixels() {
        let m = PointerMapping { origin: (-100.0, 0.0), factor: 2.0, crop: (10, 20) };
        assert_eq!(m.to_area((0.0, 50.0)), (190.0, 80.0));
    }
}
