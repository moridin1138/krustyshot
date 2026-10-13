//! Application state and the main event loop glue.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use rustshot_core::{capture, output, permissions, Config, Destination, RgbaImage};
use rustshot_editor::{Editor, EditorRequest};
use tray_icon::menu::MenuEvent;

use crate::hotkeys::HotkeyRegistry;
use crate::overlay::{Outcome, Overlay};
use crate::settings_ui::{SettingsAction, SettingsWindow};
use crate::tray::{self, Tray};
use crate::platform;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    Region,
    Window,
    Fullscreen,
    LastRegion,
}

impl CaptureMode {
    pub fn label(self) -> &'static str {
        match self {
            CaptureMode::Region => "Capture region",
            CaptureMode::Window => "Capture window",
            CaptureMode::Fullscreen => "Capture full screen",
            CaptureMode::LastRegion => "Capture last region",
        }
    }
}

enum AppEvent {
    Menu(String),
    Hotkey(u32),
}

/// A finished capture waiting to be sent somewhere.
#[derive(Clone)]
struct Capture {
    image: RgbaImage,
    title: String,
    taken_at: DateTime<Local>,
}

struct EditorWindow {
    id: egui::ViewportId,
    editor: Editor,
    taken_at: DateTime<Local>,
    path: Option<PathBuf>,
    focus: bool,
}

struct Notice {
    id: egui::ViewportId,
    title: String,
    body: String,
    permission: bool,
}

pub struct App {
    config: Config,
    events: Receiver<AppEvent>,
    tray: Option<Tray>,
    hotkeys: HotkeyRegistry,
    hotkey_errors: Vec<String>,
    overlay: Option<Overlay>,
    pending_capture: Option<(CaptureMode, Instant)>,
    picker: Option<(Capture, bool)>,
    editors: Vec<EditorWindow>,
    next_id: u64,
    settings: Option<SettingsWindow>,
    notices: Vec<Notice>,
    quit: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        // Cmd +/- are used for canvas zoom in the editor.
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        rustshot_editor::install_fonts(&ctx);

        let (tx, rx) = channel();
        {
            let (tx, ctx) = (tx.clone(), ctx.clone());
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                let _ = tx.send(AppEvent::Menu(e.id.0));
                ctx.request_repaint();
            }));
        }
        {
            let (tx, ctx) = (tx.clone(), ctx.clone());
            GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
                if e.state == HotKeyState::Pressed {
                    let _ = tx.send(AppEvent::Hotkey(e.id));
                    ctx.request_repaint();
                }
            }));
        }

        let config = Config::load();
        let tray = Tray::new(&config).map_err(|e| log::error!("tray icon: {e}")).ok();
        let mut hotkeys = HotkeyRegistry::new();
        let hotkey_errors = hotkeys.register(&config.hotkeys);
        for e in &hotkey_errors {
            log::warn!("hotkey: {e}");
        }

        let mut app = Self {
            config,
            events: rx,
            tray,
            hotkeys,
            hotkey_errors,
            overlay: None,
            pending_capture: None,
            picker: None,
            editors: Vec::new(),
            next_id: 0,
            settings: None,
            notices: Vec::new(),
            quit: false,
        };

        if !permissions::has_screen_capture_access() {
            permissions::request_screen_capture_access();
            app.notice(
                "Screen recording permission needed",
                "Rustshot needs Screen Recording access to capture other apps.\n\n\
                 Open System Settings → Privacy & Security → Screen Recording, enable Rustshot \
                 (or your terminal if you run it with cargo), then quit and restart Rustshot.",
                true,
            );
        }
        if !app.hotkey_errors.is_empty() {
            let body = format!(
                "Some hotkeys could not be registered (another app may be using them):\n\n{}\n\nChange them in Preferences.",
                app.hotkey_errors.join("\n")
            );
            app.notice("Hotkeys", &body, false);
        }
        log::info!("Rustshot running; config at {:?}", Config::path());
        app
    }

    fn notice(&mut self, title: &str, body: &str, permission: bool) {
        self.next_id += 1;
        self.notices.push(Notice {
            id: egui::ViewportId::from_hash_of(("rustshot-notice", self.next_id)),
            title: title.into(),
            body: body.into(),
            permission,
        });
    }

    fn handle_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                AppEvent::Hotkey(id) => {
                    if let Some(mode) = self.hotkeys.mode_for(id) {
                        self.start_capture(mode, ctx);
                    }
                }
                AppEvent::Menu(id) => self.handle_menu(&id, ctx),
            }
        }
        if let Some((mode, at)) = self.pending_capture {
            let now = Instant::now();
            if now >= at {
                self.pending_capture = None;
                self.start_capture(mode, ctx);
            } else {
                ctx.request_repaint_after(at - now);
            }
        }
    }

    fn handle_menu(&mut self, id: &str, ctx: &egui::Context) {
        // Give the tray menu time to disappear before grabbing the screen.
        let delayed = |mode| Some((mode, Instant::now() + Duration::from_millis(250)));
        match id {
            tray::CAPTURE_REGION => self.pending_capture = delayed(CaptureMode::Region),
            tray::CAPTURE_WINDOW => self.pending_capture = delayed(CaptureMode::Window),
            tray::CAPTURE_FULLSCREEN => self.pending_capture = delayed(CaptureMode::Fullscreen),
            tray::CAPTURE_LAST_REGION => self.pending_capture = delayed(CaptureMode::LastRegion),
            tray::OPEN_CLIPBOARD => match output::image_from_clipboard() {
                Some(img) => self.open_editor(img, "Clipboard", Local::now()),
                None => self.notice("Clipboard", "There is no image on the clipboard.", false),
            },
            tray::OPEN_FILE => {
                let file = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"])
                    .pick_file();
                if let Some(path) = file {
                    match output::load_image(&path) {
                        Ok(img) => {
                            let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                            self.open_editor(img, &title, Local::now());
                            if let Some(ew) = self.editors.last_mut() {
                                ew.path = Some(path);
                            }
                        }
                        Err(e) => self.notice("Open image", &format!("{e:#}"), false),
                    }
                }
            }
            tray::OPEN_FOLDER => {
                let _ = std::fs::create_dir_all(&self.config.output_dir);
                platform::open_folder(&self.config.output_dir);
            }
            tray::PREFERENCES => {
                if self.settings.is_none() {
                    self.settings = Some(SettingsWindow::new(&self.config));
                }
                platform::activate_app();
            }
            tray::ABOUT => self.notice(
                "About Rustshot",
                concat!(
                    "Rustshot ",
                    env!("CARGO_PKG_VERSION"),
                    "\n\nA cross-platform screenshot tool inspired by Greenshot, written in Rust with egui.\n\n",
                    "Licensed under the GPL v3 or later."
                ),
                false,
            ),
            tray::QUIT => self.quit = true,
            _ => {}
        }
        ctx.request_repaint();
    }

    fn start_capture(&mut self, mode: CaptureMode, ctx: &egui::Context) {
        if self.overlay.is_some() {
            return;
        }
        let result = match mode {
            CaptureMode::Region | CaptureMode::Window => capture::take_snapshot(true).map(|snap| {
                self.overlay = Some(Overlay::new(snap, mode == CaptureMode::Window, self.config.show_magnifier));
            }),
            CaptureMode::Fullscreen => capture::take_snapshot(false).map(|snap| {
                if let Some(image) = snap.stitch_all() {
                    self.deliver(Capture { image, title: "Full screen".into(), taken_at: snap.taken_at });
                }
            }),
            CaptureMode::LastRegion => match self.config.last_region {
                None => {
                    self.start_capture(CaptureMode::Region, ctx);
                    Ok(())
                }
                Some(region) => capture::take_snapshot(false).map(|snap| {
                    if let Some(image) = snap.crop(&region) {
                        self.deliver(Capture { image, title: String::new(), taken_at: snap.taken_at });
                    }
                }),
            },
        };
        if let Err(e) = result {
            log::error!("capture failed: {e:#}");
            self.notice("Capture failed", &format!("{e:#}"), false);
        }
        ctx.request_repaint();
    }

    /// Send a capture to the configured destinations.
    fn deliver(&mut self, cap: Capture) {
        let dests = self.config.destinations.clone();
        if dests.is_empty() || dests.contains(&Destination::Picker) {
            self.picker = Some((cap, true));
            return;
        }
        for d in dests {
            self.send_to(d, &cap);
        }
    }

    fn send_to(&mut self, dest: Destination, cap: &Capture) {
        let result = match dest {
            Destination::Picker => {
                self.picker = Some((cap.clone(), true));
                Ok(())
            }
            Destination::Editor => {
                self.open_editor(cap.image.clone(), &cap.title, cap.taken_at);
                Ok(())
            }
            Destination::Clipboard => output::copy_image_to_clipboard(&cap.image),
            Destination::File => {
                let path = output::auto_path(&self.config, &cap.title, cap.taken_at);
                save_and_report(&self.config, &cap.image, &path).map(|_| ())
            }
            Destination::FileAs => match ask_save_path(&self.config, &cap.title, cap.taken_at) {
                Some(path) => output::save_to(&cap.image, &path, &self.config),
                None => Ok(()),
            },
        };
        if let Err(e) = result {
            log::error!("{}: {e:#}", dest.label());
            self.notice(dest.label(), &format!("{e:#}"), false);
        }
    }

    fn open_editor(&mut self, image: RgbaImage, title: &str, taken_at: DateTime<Local>) {
        self.next_id += 1;
        let title = if title.trim().is_empty() { "Screenshot".to_string() } else { title.to_string() };
        self.editors.push(EditorWindow {
            id: egui::ViewportId::from_hash_of(("rustshot-editor", self.next_id)),
            editor: Editor::new(image, title),
            taken_at,
            path: None,
            focus: true,
        });
    }

    fn show_overlay(&mut self, ctx: &egui::Context) {
        let Some(overlay) = &mut self.overlay else { return };
        let Some(outcome) = overlay.show(ctx) else { return };
        let taken_at = overlay.taken_at();
        self.overlay = None;
        if let Outcome::Captured { image, title, region } = outcome {
            self.config.last_region = Some(region);
            if let Err(e) = self.config.save() {
                log::warn!("saving last region: {e:#}");
            }
            self.deliver(Capture { image, title, taken_at });
        }
    }

    fn show_picker(&mut self, ctx: &egui::Context) {
        let Some((cap, first_frame)) = &mut self.picker else { return };
        let id = egui::ViewportId::from_hash_of("rustshot-picker");
        let builder = egui::ViewportBuilder::default()
            .with_title("Rustshot")
            .with_inner_size([280.0, 250.0])
            .with_resizable(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop);
        let (w, h) = cap.image.dimensions();
        let choice = ctx.show_viewport_immediate(id, builder, |ui, _| {
            let mut choice = None;
            if ui.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape)) {
                return Some(None);
            }
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label(egui::RichText::new(format!("Screenshot {w} × {h}")).strong());
                ui.add_space(6.0);
                let full = |ui: &mut egui::Ui, text: &str| {
                    ui.add_sized([ui.available_width(), 28.0], egui::Button::new(text)).clicked()
                };
                if full(ui, "Open in image editor") {
                    choice = Some(Some(Destination::Editor));
                }
                if full(ui, "Copy to clipboard") {
                    choice = Some(Some(Destination::Clipboard));
                }
                if full(ui, "Save directly") {
                    choice = Some(Some(Destination::File));
                }
                if full(ui, "Save as…") {
                    choice = Some(Some(Destination::FileAs));
                }
                ui.add_space(4.0);
                if full(ui, "Cancel") {
                    choice = Some(None);
                }
            });
            choice
        });
        if *first_frame {
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
            platform::activate_app();
            *first_frame = false;
        }
        if let Some(choice) = choice {
            let (cap, _) = self.picker.take().unwrap();
            if let Some(dest) = choice {
                self.send_to(dest, &cap);
            }
        }
    }

    fn show_editors(&mut self, ctx: &egui::Context) {
        let config = &self.config;
        let mut closed = Vec::new();
        let mut errors = Vec::new();
        for (idx, ew) in self.editors.iter_mut().enumerate() {
            let (w, h) = ew.editor.image_size();
            let ppp = ctx.pixels_per_point().max(1.0);
            let size = [
                (w as f32 / ppp + 40.0).clamp(820.0, 1400.0),
                (h as f32 / ppp + 140.0).clamp(560.0, 950.0),
            ];
            let dirty = if ew.editor.is_dirty() { " •" } else { "" };
            let builder = egui::ViewportBuilder::default()
                .with_title(format!("Rustshot: {}{dirty}", ew.editor.title))
                .with_inner_size(size)
                .with_min_inner_size([620.0, 420.0]);
            let (requests, close_requested) = ctx.show_viewport_immediate(ew.id, builder, |ui, _| {
                let close = ui.input(|i| i.viewport().close_requested());
                (ew.editor.ui(ui), close)
            });
            if ew.focus {
                ctx.send_viewport_cmd_to(ew.id, egui::ViewportCommand::Focus);
                platform::activate_app();
                ew.focus = false;
            }

            let mut close = close_requested;
            for req in requests {
                let result = match req {
                    EditorRequest::Save => save_editor(config, ew, false),
                    EditorRequest::SaveAs => save_editor(config, ew, true),
                    EditorRequest::Copy => output::copy_image_to_clipboard(&ew.editor.flatten())
                        .map(|_| ew.editor.set_status("Copied to clipboard")),
                    EditorRequest::Close => {
                        close = true;
                        Ok(())
                    }
                };
                if let Err(e) = result {
                    ew.editor.set_status(format!("Error: {e:#}"));
                    errors.push(format!("{e:#}"));
                }
            }
            if close {
                if confirm_close(config, ew) {
                    closed.push(idx);
                } else if close_requested {
                    ctx.send_viewport_cmd_to(ew.id, egui::ViewportCommand::CancelClose);
                }
            }
        }
        for idx in closed.into_iter().rev() {
            self.editors.remove(idx);
        }
        for e in errors {
            log::error!("editor: {e}");
        }
    }

    fn show_settings(&mut self, ctx: &egui::Context) {
        let Some(settings) = &mut self.settings else { return };
        match settings.show(ctx, &self.hotkey_errors) {
            Some(SettingsAction::Save(cfg)) => {
                self.config = *cfg;
                if let Err(e) = self.config.save() {
                    self.notice("Preferences", &format!("Could not save preferences: {e:#}"), false);
                }
                self.hotkey_errors = self.hotkeys.register(&self.config.hotkeys);
                // Rebuild the tray so menu shortcuts match the new hotkeys.
                self.tray = None;
                self.tray = Tray::new(&self.config).map_err(|e| log::error!("tray icon: {e}")).ok();
                if self.hotkey_errors.is_empty() {
                    self.settings = None;
                }
            }
            Some(SettingsAction::Cancel) => self.settings = None,
            None => {}
        }
    }

    fn show_notices(&mut self, ctx: &egui::Context) {
        let mut done = Vec::new();
        for (i, n) in self.notices.iter().enumerate() {
            let builder = egui::ViewportBuilder::default()
                .with_title(&n.title)
                .with_inner_size([420.0, 220.0])
                .with_window_level(egui::WindowLevel::AlwaysOnTop);
            let close = ctx.show_viewport_immediate(n.id, builder, |ui, _| {
                let mut close = ui.input(|i| i.viewport().close_requested());
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.heading(&n.title);
                    ui.add_space(6.0);
                    ui.label(&n.body);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if n.permission && ui.button("Open System Settings").clicked() {
                            permissions::open_privacy_settings();
                        }
                        if ui.button("OK").clicked() {
                            close = true;
                        }
                    });
                });
                close
            });
            if close {
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            self.notices.remove(i);
        }
    }
}

fn ask_save_path(config: &Config, title: &str, time: DateTime<Local>) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_directory(&config.output_dir)
        .set_file_name(output::suggested_name(config, title, time))
        .add_filter("PNG", &["png"])
        .add_filter("JPEG", &["jpg", "jpeg"])
        .add_filter("BMP", &["bmp"])
        .add_filter("GIF", &["gif"])
        .save_file()
}

fn save_and_report(config: &Config, image: &RgbaImage, path: &std::path::Path) -> anyhow::Result<PathBuf> {
    output::save_to(image, path, config)?;
    log::info!("saved {}", path.display());
    if config.copy_path_after_save {
        output::copy_text_to_clipboard(&path.display().to_string())?;
    }
    Ok(path.to_path_buf())
}

fn save_editor(config: &Config, ew: &mut EditorWindow, ask: bool) -> anyhow::Result<()> {
    let path = if ask {
        match ask_save_path(config, &ew.editor.title, ew.taken_at) {
            Some(p) => p,
            None => return Ok(()),
        }
    } else {
        match &ew.path {
            Some(p) => p.clone(),
            None => output::auto_path(config, &ew.editor.title, ew.taken_at),
        }
    };
    let path = save_and_report(config, &ew.editor.flatten(), &path)?;
    ew.editor.mark_saved();
    ew.editor.set_status(format!("Saved to {}", path.display()));
    ew.path = Some(path);
    Ok(())
}

/// Ask about unsaved changes. Returns true if the editor may close.
fn confirm_close(config: &Config, ew: &mut EditorWindow) -> bool {
    if !ew.editor.is_dirty() {
        return true;
    }
    let answer = rfd::MessageDialog::new()
        .set_title("Rustshot")
        .set_description("Save changes to this screenshot before closing?")
        .set_buttons(rfd::MessageButtons::YesNoCancel)
        .show();
    match answer {
        rfd::MessageDialogResult::Yes => match save_editor(config, ew, false) {
            Ok(()) => true,
            Err(e) => {
                ew.editor.set_status(format!("Error: {e:#}"));
                false
            }
        },
        rfd::MessageDialogResult::No => true,
        _ => false,
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events(ctx);
        if self.quit {
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.show_overlay(&ctx);
        self.show_picker(&ctx);
        self.show_editors(&ctx);
        self.show_settings(&ctx);
        self.show_notices(&ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
}
