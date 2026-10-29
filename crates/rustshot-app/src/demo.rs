//! Screenshots for the README (built only with `--features demo`).
//!
//! Run the signed bundle with `KRUSTYSHOT_DEMO=<folder>` (and
//! `RUSTSHOT_SETTINGS_TOUR=<folder>` for the Settings pages); see
//! `scripts/readme-screenshots.sh`. It opens each window in turn with sample
//! content and photographs only that window (`screencapture -l`), so nothing
//! else on the screen ends up in the pictures. The scrolling and recording
//! shots work on the demo's own page window, which stays on top.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::Local;
use egui::{Color32, Pos2, TextureHandle, TextureOptions};
use rustshot_core::{capture, Destination, Region, RgbaImage};
use rustshot_editor::document::{Element, HighlightMode, Kind, Style};
use rustshot_i18n::tr;

use super::{App, Capture};
use crate::settings_ui::SettingsWindow;

const PAGE_TITLE: &str = "Krustyshot demo page";
const SAMPLE_TITLE: &str = "Screenshot - Wikipedia";
/// Where the page window sits, in points; also the recorded and scrolled area.
const PAGE: Region = Region { x: 140.0, y: 140.0, w: 900.0, h: 640.0 };
/// How much of the tall page to show (pixels), so the scrolling result
/// stays a readable length.
const PAGE_HEIGHT: u32 = 5600;

pub struct Demo {
    dir: PathBuf,
    sample: RgbaImage,
    page: RgbaImage,
    texture: Option<TextureHandle>,
    show_page: bool,
    /// How far the page is scrolled, in points.
    offset: f32,
    scrolled: bool,
    /// Which parts to photograph (`KRUSTYSHOT_DEMO_PARTS`, comma-separated:
    /// editor, menu, settings, recording, scrolling); all by default.
    parts: Vec<String>,
    step: u32,
    next: Instant,
}

impl Demo {
    /// From `KRUSTYSHOT_DEMO`; the sample images sit in that folder.
    pub fn from_env() -> Option<Self> {
        let dir = PathBuf::from(std::env::var_os("KRUSTYSHOT_DEMO")?);
        let load = |name: &str| rustshot_core::image::open(dir.join(name)).map(|i| i.to_rgba8());
        let (sample, page) = match (load("sample.png"), load("page-tall.png")) {
            (Ok(s), Ok(p)) => (s, p),
            (s, p) => {
                log::error!("demo: sample images missing: {:?} {:?}", s.err(), p.err());
                return None;
            }
        };
        let height = page.height().min(PAGE_HEIGHT);
        let page = rustshot_core::image::imageops::crop_imm(&page, 0, 0, page.width(), height).to_image();
        Some(Self { dir, sample, page, texture: None, show_page: false,
            offset: 0.0,
            scrolled: false,
            parts: std::env::var("KRUSTYSHOT_DEMO_PARTS")
                .unwrap_or_else(|_| "editor,menu,settings,recording,scrolling".into())
                .split(',')
                .map(|p| p.trim().to_string())
                .collect(),
            step: 0, next: Instant::now() + Duration::from_secs(2) })
    }

    fn wait(&mut self, ms: u64) {
        self.next = Instant::now() + Duration::from_millis(ms);
        self.step += 1;
    }

    fn wants(&self, part: &str) -> bool {
        self.parts.iter().any(|p| p == part)
    }

    /// Go on to the next part that was asked for (parts start at multiples
    /// of 10: editor 10, menu 20, settings 30, recording 40, scrolling 50).
    fn next_part(&mut self, ms: u64) {
        let order = ["editor", "menu", "settings", "recording", "scrolling"];
        let from = (self.step / 10) as usize;
        self.step = order.iter().enumerate().skip(from).find(|(_, p)| self.wants(p)).map_or(99, |(i, _)| (i as u32 + 1) * 10);
        self.next = Instant::now() + Duration::from_millis(ms);
    }

    /// The scrolling capture's result arrives through the usual delivery;
    /// keep it here instead.
    pub fn take_delivery(&mut self, cap: &Capture) -> bool {
        let path = self.dir.join("scrolling.png");
        match cap.image.save(&path) {
            Ok(()) => log::info!("demo: saved {}", path.display()),
            Err(e) => log::error!("demo: saving {}: {e}", path.display()),
        }
        self.scrolled = true;
        true
    }
    fn photo(&self, title: &str, name: &str) {
        let Some(n) = crate::platform::window_number(title) else {
            log::warn!("demo: no window called {title:?}");
            return;
        };
        let path = self.dir.join(name);
        let status = std::process::Command::new("screencapture").args(["-x", "-o", &format!("-l{n}")]).arg(&path).status();
        log::info!("demo: {} -> {status:?}", path.display());
    }
}

/// The recording and scrolling bars hide from captures; let this one in.
fn set_capturable(title: &str) {
    use objc2_app_kit::{NSApplication, NSWindowSharingType};
    use objc2_foundation::MainThreadMarker;
    let Some(mtm) = MainThreadMarker::new() else { return };
    for window in NSApplication::sharedApplication(mtm).windows().iter() {
        if window.title().to_string() == title {
            window.setSharingType(NSWindowSharingType::ReadOnly);
        }
    }
}

/// Sample settings: MP4 with sound (for the bar's icons) and an uploader
/// (so the quick menu shows every destination).
pub fn prepare(app: &mut App) {
    let c = &mut app.config;
    c.language = Some("en".into());
    c.destinations = vec![Destination::Picker];
    c.recording.format = rustshot_core::VideoFormat::Mp4;
    // Sound only for the recording bar's icons.
    let sound = app.demo.as_ref().is_some_and(|d| d.wants("recording"));
    c.recording.microphone = sound;
    c.recording.system_audio = sound;
    c.uploader.name = "img.example.com".into();
    c.uploader.url = "https://img.example.com/upload".into();
    rustshot_i18n::set_language(Some("en"));
}

/// Annotations for the editor picture, in sample-image pixels (2200 x 1400).
fn annotations() -> Vec<Element> {
    let red = Style { width: 6.0, font_size: 40.0, ..Style::default() };
    let shape = |kind, a: (f32, f32), b: (f32, f32), style: Style| {
        let mut e = Element::new(kind, Pos2::new(a.0, a.1), style);
        e.b = Pos2::new(b.0, b.1);
        e
    };
    let mut speech = shape(Kind::Speech, (760.0, 800.0), (760.0, 800.0), Style { fill: Color32::WHITE, width: 4.0, font_size: 34.0, ..red });
    speech.text = "Annotate, then copy, save, print or upload".into();
    speech.fit_content();
    // The measurement runs a little short for this font size.
    speech.b.x += 70.0;
    speech.tail = Pos2::new(1610.0, 720.0);
    vec![
        shape(Kind::Highlight, (478.0, 500.0), (935.0, 548.0), Style { highlight_mode: HighlightMode::Text, ..red }),
        shape(Kind::Rect, (1625.0, 505.0), (2158.0, 852.0), red),
        shape(Kind::Ellipse, (1860.0, 185.0), (2170.0, 265.0), red),
        shape(Kind::Arrow, (780.0, 232.0), (440.0, 228.0), red),
        shape(Kind::Obfuscate, (1740.0, 38.0), (2165.0, 106.0), Style { strength: 14.0, ..red }),
        Element::counter(Pos2::new(830.0, 232.0), red),
        Element::counter(Pos2::new(1625.0, 505.0), red),
        Element::counter(Pos2::new(1700.0, 72.0), red),
        speech,
    ]
}

/// One step of the tour, when its time comes.
pub fn step(app: &mut App, ctx: &egui::Context) {
    let Some(demo) = &mut app.demo else { return };
    ctx.request_repaint_after(Duration::from_millis(100));
    if Instant::now() < demo.next {
        return;
    }
    let now = Local::now();
    match demo.step {
        0 => {
            demo.show_page = true;
            demo.next_part(2000);
        }
        10 => {
            let sample = demo.sample.clone();
            demo.wait(2500);
            app.open_editor(sample, SAMPLE_TITLE, now);
            if let Some(ew) = app.editors.last_mut() {
                ew.editor.debug_add(annotations());
                // No "unsaved" dot in the title.
                ew.editor.mark_saved();
            }
        }
        11 => {
            demo.photo(&format!("Krustyshot: {SAMPLE_TITLE}"), "editor.png");
            demo.next_part(1000);
            app.editors.clear();
        }
        20 => {
            let image = demo.sample.clone();
            demo.wait(1500);
            app.picker = Some((Capture { image, title: SAMPLE_TITLE.into(), taken_at: now }, true));
        }
        21 => {
            demo.photo("Krustyshot", "quick-menu.png");
            demo.next_part(1000);
            app.picker = None;
        }
        30 => {
            // RUSTSHOT_SETTINGS_TOUR photographs each section.
            demo.wait(9000);
            app.settings = Some(SettingsWindow::new());
        }
        31 => {
            demo.next_part(1500);
            app.settings = None;
        }
        40 => {
            demo.wait(3000);
            match capture::take_snapshot(capture::SnapshotOptions { windows: false, pointer: false }) {
                Ok(snap) => {
                    let target = snap.recording_target(&PAGE);
                    let monitor = snap.monitor_for(&PAGE).map(|m| m.rect);
                    app.start_recording(PAGE, target, monitor, SAMPLE_TITLE);
                }
                Err(e) => log::error!("demo: {e:#}"),
            }
        }
        41 => {
            // The bar hides from captures; it takes a moment to show again.
            set_capturable(&tr!("recording-bar-title"));
            demo.wait(1000);
        }
        42 => {
            demo.photo(&tr!("recording-bar-title"), "recording-bar.png");
            demo.next_part(2000);
            app.finish_recording(true);
        }
        50 => {
            demo.wait(1200);
            match capture::take_snapshot(capture::SnapshotOptions { windows: false, pointer: false }) {
                Ok(snap) => {
                    let target = snap.recording_target(&PAGE);
                    let monitor = snap.monitor_for(&PAGE).map(|m| m.rect);
                    app.start_scroll(PAGE, target, monitor, SAMPLE_TITLE.into(), now);
                }
                Err(e) => log::error!("demo: {e:#}"),
            }
        }
        51 => {
            set_capturable(&tr!("scroll-bar-title"));
            demo.wait(500);
        }
        52 => {
            demo.photo(&tr!("scroll-bar-title"), "scrolling-bar.png");
            // Then wait for the result (take_delivery), for up to two minutes.
            demo.step = 99;
            demo.next = Instant::now() + Duration::from_secs(120);
        }
        _ => {}
    }
    if let Some(d) = &app.demo
        && d.step == 99
        && (d.scrolled || !d.wants("scrolling") || Instant::now() >= d.next)
    {
        log::info!("demo: done{}", if d.scrolled { "" } else { ", but no scrolling result" });
        app.quit = true;
    }
}

/// The demo's long page, in its own window on top of everything.
pub fn show_page(app: &mut App, ctx: &egui::Context) {
    let Some(demo) = &mut app.demo else { return };
    if !demo.show_page {
        return;
    }
    let texture = demo.texture.get_or_insert_with(|| {
        let (w, h) = demo.page.dimensions();
        let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], demo.page.as_raw());
        ctx.load_texture("demo-page", image, TextureOptions::LINEAR)
    });
    let (id, size) = (texture.id(), texture.size_vec2());
    let builder = egui::ViewportBuilder::default()
        .with_title(PAGE_TITLE)
        .with_position([PAGE.x as f32, PAGE.y as f32])
        .with_inner_size([PAGE.w as f32, PAGE.h as f32])
        .with_decorations(false)
        .with_resizable(false)
        .with_taskbar(false)
        .with_active(false)
        .with_window_level(egui::WindowLevel::AlwaysOnTop);
    // Not a ScrollArea: it animates and adds its own speed. This page moves
    // by exactly the distance in each wheel event, like a browser.
    let width = PAGE.w as f32;
    let max = (size.y * width / size.x - PAGE.h as f32).max(0.0);
    let offset = &mut demo.offset;
    ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("krustyshot-demo-page"), builder, |ui, _| {
        let delta = ui.input(|i| {
            i.events.iter().filter_map(|e| if let egui::Event::MouseWheel { delta, unit, .. } = e { Some((delta.y, *unit)) } else { None }).collect::<Vec<_>>()
        });
        for (dy, unit) in delta {
            log::info!("demo page: wheel {dy} {unit:?}");
            *offset = (*offset - dy).clamp(0.0, max);
        }
        let rect = ui.max_rect();
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, *offset / (max + PAGE.h as f32)), egui::pos2(1.0, (*offset + PAGE.h as f32) / (max + PAGE.h as f32)));
        ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
        ui.painter().image(id, rect, uv, Color32::WHITE);
    });
}
