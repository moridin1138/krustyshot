//! The frozen-screen capture overlay: one borderless window per monitor
//! showing the screenshot taken when the capture started. The user drags a
//! region or clicks a window, with a magnifier and size readout, like
//! Greenshot.

use egui::{Align2, Color32, FontId, Key, Pos2, Rect, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2};
use rustshot_core::{RgbaImage, Region, Snapshot};

use crate::platform;

pub enum Outcome {
    Cancelled,
    Captured { image: RgbaImage, title: String, region: Region },
}

pub struct Overlay {
    snapshot: Snapshot,
    textures: Vec<Option<TextureHandle>>,
    window_mode: bool,
    show_magnifier: bool,
    drag_start: Option<(f64, f64)>,
    /// The mouse button went down while the overlay was open. A release
    /// without this (e.g. from a click that started before the overlay
    /// appeared) must not trigger a capture.
    pressed_here: bool,
    cursor: Option<(f64, f64)>,
    frames: u32,
    outcome: Option<Outcome>,
}

const ACCENT: Color32 = Color32::from_rgb(0, 170, 255);

impl Overlay {
    pub fn new(snapshot: Snapshot, window_mode: bool, show_magnifier: bool) -> Self {
        let textures = snapshot.monitors.iter().map(|_| None).collect();
        Self {
            snapshot,
            textures,
            window_mode,
            show_magnifier,
            drag_start: None,
            pressed_here: false,
            cursor: None,
            frames: 0,
            outcome: None,
        }
    }

    pub fn taken_at(&self) -> chrono::DateTime<chrono::Local> {
        self.snapshot.taken_at
    }

    /// Show the overlay windows. Returns the outcome once the user is done.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Outcome> {
        for i in 0..self.snapshot.monitors.len() {
            let r = self.snapshot.monitors[i].rect;
            let id = egui::ViewportId::from_hash_of(("rustshot-overlay", i));
            let builder = egui::ViewportBuilder::default()
                .with_title(platform::overlay_title(i))
                .with_position([r.x as f32, r.y as f32])
                .with_inner_size([r.w as f32, r.h as f32])
                .with_decorations(false)
                .with_resizable(false)
                .with_has_shadow(false)
                .with_taskbar(false)
                .with_window_level(egui::WindowLevel::AlwaysOnTop)
                .with_active(true);
            ctx.show_viewport_immediate(id, builder, |ui, _| self.monitor_ui(ui, i));
            if self.frames < 3 {
                ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
            }
        }
        // Cheap, and catches the OS moving or resizing the windows after creation.
        let rects: Vec<Region> = self.snapshot.monitors.iter().map(|m| m.rect).collect();
        platform::raise_overlays(&rects);
        if self.frames == 0 {
            platform::activate_app();
        }
        self.frames += 1;
        self.outcome.take()
    }

    /// The highlighted area: the drag selection, or the window under the cursor.
    fn highlight(&self) -> Option<(Region, String)> {
        let cursor = self.cursor?;
        if let Some(start) = self.drag_start.filter(|_| !self.window_mode) {
            return Some((Region::from_points(start, cursor), String::new()));
        }
        self.snapshot
            .window_at(cursor.0, cursor.1)
            .map(|w| (w.rect, w.title.clone()))
    }

    fn finish(&mut self, region: Region, title: String) {
        self.outcome = Some(match self.snapshot.crop(&region) {
            Some(image) => Outcome::Captured { image, title, region },
            None => Outcome::Cancelled,
        });
    }

    fn monitor_ui(&mut self, ui: &mut egui::Ui, i: usize) {
        let screen = ui.max_rect();
        let m = &self.snapshot.monitors[i];
        let mon = m.rect;
        let scale = m.scale();
        let is_primary = m.is_primary;
        let tex = self.textures[i]
            .get_or_insert_with(|| {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [m.image.width() as usize, m.image.height() as usize],
                    m.image.as_raw(),
                );
                ui.ctx().load_texture(format!("rustshot-overlay-{i}"), img, TextureOptions::NEAREST)
            })
            .clone();

        // Where this window really is on the desktop. The OS may not have
        // given us exactly the monitor's bounds (macOS keeps windows below the
        // menu bar until `raise_overlays` resizes them), so draw the frozen
        // screenshot at the monitor's true position rather than stretching it
        // to fit the window.
        let window = ui.input(|inp| inp.viewport().inner_rect);
        let origin = match window {
            Some(w) => (w.min.x as f64, w.min.y as f64),
            None => (mon.x, mon.y),
        };
        if self.frames == 5 {
            log::debug!("overlay {i}: monitor {mon:?}, window {window:?}, ui {screen:?}");
        }
        let to_global = |p: Pos2| (origin.0 + (p.x - screen.min.x) as f64, origin.1 + (p.y - screen.min.y) as f64);
        let to_local = |x: f64, y: f64| {
            Pos2::new(screen.min.x + (x - origin.0) as f32, screen.min.y + (y - origin.1) as f32)
        };
        let to_local_rect = |r: Region| Rect::from_min_max(to_local(r.x, r.y), to_local(r.right(), r.bottom()));
        let monitor_rect = to_local_rect(mon);

        // ---- input ----
        let (pos, pressed, released, esc, space) = ui.input(|inp| {
            (
                inp.pointer.latest_pos(),
                inp.pointer.primary_pressed(),
                inp.pointer.primary_released(),
                inp.key_pressed(Key::Escape),
                inp.key_pressed(Key::Space),
            )
        });
        if let Some(p) = pos {
            self.cursor = Some(to_global(p));
        }
        if esc || ui.input(|inp| inp.pointer.secondary_clicked()) {
            self.outcome = Some(Outcome::Cancelled);
        }
        if space {
            self.window_mode = !self.window_mode;
            self.drag_start = None;
        }
        if pressed && pos.is_some() {
            self.pressed_here = true;
            if !self.window_mode {
                self.drag_start = self.cursor;
            }
        }
        if released && pos.is_some() && std::mem::take(&mut self.pressed_here) {
            let cursor = self.cursor.unwrap_or_default();
            let dragged = self
                .drag_start
                .map(|s| Region::from_points(s, cursor))
                .filter(|r| r.w >= 3.0 || r.h >= 3.0);
            self.drag_start = None;
            match (dragged, self.snapshot.window_at(cursor.0, cursor.1)) {
                (Some(r), _) => self.finish(r, String::new()),
                (None, Some(w)) => {
                    let (r, t) = (w.rect, w.title.clone());
                    self.finish(r, t);
                }
                // Clicked on the bare desktop: take the whole monitor.
                (None, None) => {
                    if let Some(m) = self.snapshot.monitor_at(cursor.0, cursor.1) {
                        let r = m.rect;
                        self.finish(r, String::new());
                    }
                }
            }
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);

        // ---- painting ----
        let painter = ui.painter();
        let full_uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter.rect_filled(screen, 0.0, Color32::BLACK);
        painter.image(tex.id(), monitor_rect, full_uv, Color32::WHITE);
        let shade = Color32::from_black_alpha(110);

        let highlight = self.highlight();
        let hl_rect = highlight.as_ref().map(|(r, _)| to_local_rect(*r).intersect(screen));
        match hl_rect.filter(|r| r.is_positive()) {
            Some(h) => {
                for r in [
                    Rect::from_min_max(screen.min, Pos2::new(screen.max.x, h.min.y)),
                    Rect::from_min_max(Pos2::new(screen.min.x, h.max.y), screen.max),
                    Rect::from_min_max(Pos2::new(screen.min.x, h.min.y), Pos2::new(h.min.x, h.max.y)),
                    Rect::from_min_max(Pos2::new(h.max.x, h.min.y), Pos2::new(screen.max.x, h.max.y)),
                ] {
                    if r.is_positive() {
                        painter.rect_filled(r, 0.0, shade);
                    }
                }
                painter.rect_stroke(h, 0.0, Stroke::new(2.0, ACCENT), StrokeKind::Outside);
            }
            None => {
                painter.rect_filled(screen, 0.0, shade);
            }
        }

        // Size readout in real pixels next to the selection.
        if let Some((r, title)) = &highlight
            && let Some(h) = hl_rect.filter(|r| r.is_positive()) {
                let mut label = format!("{} × {}", (r.w * scale).round(), (r.h * scale).round());
                if !title.is_empty() && self.drag_start.is_none() {
                    label = format!("{title}  ·  {label}");
                }
                let anchor = Pos2::new(h.min.x, (h.max.y + 6.0).min(screen.max.y - 24.0));
                pill(painter, anchor, Align2::LEFT_TOP, &label);
            }

        let cursor_here = self.cursor.filter(|c| mon.contains(c.0, c.1));
        if let Some(c) = cursor_here {
            let p = to_local(c.0, c.1);
            if self.drag_start.is_none() && !self.window_mode {
                let line = Stroke::new(1.0, ACCENT.gamma_multiply(0.7));
                painter.hline(screen.x_range(), p.y, line);
                painter.vline(p.x, screen.y_range(), line);
            }
            if self.show_magnifier {
                magnifier(painter, &tex, screen, p, (c.0 - mon.x, c.1 - mon.y), mon, scale);
            }
        }

        // Instructions at the top of the monitor with the cursor (or primary).
        let show_help = cursor_here.is_some() || (self.cursor.is_none() && is_primary);
        if show_help {
            let text = if self.window_mode {
                "Click a window to capture it  ·  Space: region mode  ·  Esc: cancel"
            } else {
                "Drag to select a region  ·  Click a window to capture it  ·  Space: window mode  ·  Esc: cancel"
            };
            pill(painter, Pos2::new(screen.center().x, screen.min.y + 40.0), Align2::CENTER_TOP, text);
        }
    }
}

fn pill(painter: &egui::Painter, pos: Pos2, align: Align2, text: &str) {
    let galley = painter.layout_no_wrap(text.to_string(), FontId::proportional(13.0), Color32::WHITE);
    let rect = align.anchor_size(pos, galley.size() + Vec2::new(16.0, 8.0));
    painter.rect_filled(rect, 6.0, Color32::from_black_alpha(200));
    painter.galley(rect.min + Vec2::new(8.0, 4.0), galley, Color32::WHITE);
}

/// Zoomed view of the pixels around the cursor, with coordinates.
fn magnifier(
    painter: &egui::Painter,
    tex: &TextureHandle,
    screen: Rect,
    p: Pos2,
    local: (f64, f64),
    mon: Region,
    scale: f64,
) {
    const SIZE: f32 = 140.0;
    const ZOOM: f32 = 8.0;
    let mut pos = p + Vec2::new(24.0, 24.0);
    if pos.x + SIZE > screen.max.x {
        pos.x = p.x - 24.0 - SIZE;
    }
    if pos.y + SIZE + 24.0 > screen.max.y {
        pos.y = p.y - 24.0 - SIZE - 24.0;
    }
    let rect = Rect::from_min_size(pos, Vec2::splat(SIZE));
    // Source area in points, centred on the cursor, as UV coordinates.
    let half = SIZE / ZOOM / 2.0;
    let u = (local.0 / mon.w) as f32;
    let v = (local.1 / mon.h) as f32;
    let du = half / mon.w as f32;
    let dv = half / mon.h as f32;
    let uv = Rect::from_min_max(Pos2::new(u - du, v - dv), Pos2::new(u + du, v + dv));
    painter.rect_filled(rect, 0.0, Color32::BLACK);
    painter.image(tex.id(), rect, uv, Color32::WHITE);
    let c = rect.center();
    let s = Stroke::new(1.0, ACCENT);
    painter.hline(rect.x_range(), c.y, s);
    painter.vline(c.x, rect.y_range(), s);
    painter.rect_stroke(rect, 0.0, Stroke::new(2.0, Color32::WHITE), StrokeKind::Outside);
    let label = format!("{}, {}", (local.0 * scale).round(), (local.1 * scale).round());
    pill(painter, Pos2::new(rect.center().x, rect.max.y + 4.0), Align2::CENTER_TOP, &label);
}
