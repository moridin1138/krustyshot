//! Application state and the main event loop glue.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use rustshot_core::{capture, output, permissions, Config, Destination, RgbaImage, TrayClick};
use rustshot_editor::{Editor, EditorRequest};
use tray_icon::menu::MenuEvent;
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};

use crate::fonts;
use rustshot_i18n::tr;
use crate::hotkeys::HotkeyRegistry;
use crate::overlay::{Outcome, Overlay};
use crate::settings_ui::SettingsWindow;
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
    pub fn label(self) -> String {
        match self {
            CaptureMode::Region => tr!("capture-region"),
            CaptureMode::Window => tr!("capture-window"),
            CaptureMode::Fullscreen => tr!("capture-fullscreen"),
            CaptureMode::LastRegion => tr!("capture-last-region"),
        }
    }
}

enum AppEvent {
    Menu(String),
    Hotkey(u32),
    /// Left click on the tray icon (only sent when it doesn't open the menu).
    TrayClick,
}

/// A short-lived message near the corner of the screen ("Saved", "Copied").
struct Toast {
    id: egui::ViewportId,
    message: String,
    /// The saved file; clicking the toast shows it in Finder / Explorer.
    path: Option<PathBuf>,
    until: Instant,
    shown: bool,
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
    shown: bool,
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
    background_hidden: bool,
    /// Debug builds only: next step of the RUSTSHOT_STRESS window test.
    stress: Option<(Instant, u64)>,
    toast: Option<Toast>,
    hotkeys_paused: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        // Cmd +/- are used for canvas zoom in the editor.
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        fonts::install(&ctx);
        platform::hide_background_window();

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
            TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                    let _ = tx.send(AppEvent::TrayClick);
                    ctx.request_repaint();
                }
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
        rustshot_i18n::set_language(config.language.as_deref());
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
            background_hidden: false,
            stress: (cfg!(debug_assertions) && std::env::var_os("RUSTSHOT_STRESS").is_some())
                .then(|| (Instant::now() + Duration::from_secs(2), 0)),
            toast: None,
            hotkeys_paused: false,
        };

        // Let macOS show its own permission prompt once (this also adds Rustshot
        // to the Screen Recording list). Rustshot's explanation only appears if
        // a capture is attempted without permission, so the two never compete.
        if !permissions::has_screen_capture_access() {
            log::info!("no Screen Recording permission yet; asking macOS");
            permissions::request_screen_capture_access();
        }
        if !app.hotkey_errors.is_empty() {
            let body = tr!("notice-hotkeys-body", errors = app.hotkey_errors.join("\n"));
            app.notice(&tr!("notice-hotkeys-title"), &body, false);
        }
        // `rustshot --settings` opens the Settings window straight away.
        if std::env::args().any(|a| a == "--settings") {
            app.settings = Some(SettingsWindow::new());
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
            shown: false,
        });
    }

    /// Debug builds with RUSTSHOT_STRESS=1: open and close editor and notice
    /// windows in a loop, to reproduce window-lifecycle bugs without
    /// touching the user's files or clipboard.
    fn stress_step(&mut self, ctx: &egui::Context) {
        let Some((at, n)) = self.stress else { return };
        let now = Instant::now();
        if now < at {
            ctx.request_repaint_after(at - now);
            return;
        }
        match n % 4 {
            0 => {
                self.open_editor(RgbaImage::from_pixel(400, 300, image_rgba(n)), &format!("Stress {n}"), Local::now());
                if let Some(ew) = self.editors.last_mut() {
                    ew.editor.debug_start_text("typing");
                }
            }
            1 => self.editors.clear(),
            2 => self.notice("Stress", &format!("Stress notice {n}"), false),
            _ => self.notices.clear(),
        }
        log::debug!("stress step {n}");
        self.stress = Some((now + Duration::from_millis(700), n + 1));
        ctx.request_repaint();
    }

    fn notify_saved(&mut self, path: PathBuf) {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        self.notify(&tr!("toast-saved", name = name), Some(path));
    }

    /// Show a brief toast, if notifications are on.
    fn notify(&mut self, message: &str, path: Option<PathBuf>) {
        if !self.config.notifications {
            return;
        }
        self.next_id += 1;
        self.toast = Some(Toast {
            id: egui::ViewportId::from_hash_of(("rustshot-toast", self.next_id)),
            message: message.into(),
            path,
            until: Instant::now() + Duration::from_millis(2800),
            shown: false,
        });
    }

    fn show_toast(&mut self, ctx: &egui::Context) {
        let Some(toast) = &mut self.toast else { return };
        let now = Instant::now();
        if now >= toast.until {
            self.toast = None;
            return;
        }
        ctx.request_repaint_after(toast.until - now);
        const SIZE: egui::Vec2 = egui::vec2(320.0, 64.0);
        let monitor = ctx.input(|i| i.viewport().monitor_size).unwrap_or(egui::vec2(1440.0, 900.0));
        // Top right on macOS (where notifications appear), bottom right elsewhere.
        let y = if cfg!(target_os = "macos") { 40.0 } else { monitor.y - SIZE.y - 60.0 };
        let builder = egui::ViewportBuilder::default()
            .with_title(platform::TOAST_TITLE)
            .with_position([monitor.x - SIZE.x - 16.0, y])
            .with_inner_size(SIZE)
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_active(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop);
        let clicked = ctx.show_viewport_immediate(toast.id, builder, |ui, _| {
            let frame = egui::Frame::new().fill(ui.visuals().window_fill).inner_margin(12.0);
            let resp = egui::CentralPanel::default()
                .frame(frame)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(egui_phosphor::regular::CHECK_CIRCLE).size(22.0).color(egui::Color32::from_rgb(40, 170, 90)));
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&toast.message).strong());
                            if toast.path.is_some() {
                                ui.weak(if cfg!(target_os = "macos") { tr!("toast-show-finder") } else { tr!("toast-show-file") });
                            }
                        });
                    });
                })
                .response;
            ui.interact(resp.rect, egui::Id::new("toast-click"), egui::Sense::click()).clicked()
        });
        if !toast.shown {
            platform::make_transient(platform::TOAST_TITLE);
            toast.shown = true;
        }
        if clicked {
            if let Some(path) = &toast.path {
                platform::reveal(path);
            }
            self.toast = None;
        }
    }

    /// Explain how to grant Screen Recording access (shown at most once at a time).
    fn permission_notice(&mut self) {
        if self.notices.iter().any(|n| n.permission) {
            return;
        }
        self.notice(&tr!("permission-title"), &tr!("permission-body"), true);
    }

    fn handle_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                AppEvent::Hotkey(id) => {
                    if let Some(mode) = self.hotkeys.mode_for(id) {
                        self.request_capture(mode, false, ctx);
                    }
                }
                AppEvent::Menu(id) => self.handle_menu(&id, ctx),
                AppEvent::TrayClick => {
                    let mode = match self.config.tray_click {
                        TrayClick::ShowMenu => None,
                        TrayClick::CaptureRegion => Some(CaptureMode::Region),
                        TrayClick::CaptureWindow => Some(CaptureMode::Window),
                        TrayClick::CaptureFullscreen => Some(CaptureMode::Fullscreen),
                        TrayClick::CaptureLastRegion => Some(CaptureMode::LastRegion),
                    };
                    if let Some(mode) = mode {
                        self.request_capture(mode, true, ctx);
                    }
                }
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
        match id {
            tray::CAPTURE_REGION => self.request_capture(CaptureMode::Region, true, ctx),
            tray::CAPTURE_WINDOW => self.request_capture(CaptureMode::Window, true, ctx),
            tray::CAPTURE_FULLSCREEN => self.request_capture(CaptureMode::Fullscreen, true, ctx),
            tray::CAPTURE_LAST_REGION => self.request_capture(CaptureMode::LastRegion, true, ctx),
            tray::OPEN_CLIPBOARD => match output::image_from_clipboard() {
                Some(img) => self.open_editor(img, &tr!("title-clipboard"), Local::now()),
                None => self.notice(&tr!("title-clipboard"), &tr!("notice-no-clipboard-image"), false),
            },
            tray::OPEN_FILE => {
                let file = rfd::FileDialog::new()
                    .add_filter(tr!("filter-images"), &["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"])
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
                        Err(e) => self.notice(&tr!("notice-open-image"), &format!("{e:#}"), false),
                    }
                }
            }
            tray::OPEN_FOLDER => {
                let _ = std::fs::create_dir_all(&self.config.output_dir);
                platform::open_folder(&self.config.output_dir);
            }
            tray::PREFERENCES => {
                if self.settings.is_none() {
                    self.settings = Some(SettingsWindow::new());
                }
                platform::activate_app();
            }
            tray::ABOUT => self.notice(&tr!("menu-about"), &tr!("about-body", version = env!("CARGO_PKG_VERSION")), false),
            tray::QUIT => self.quit = true,
            _ => {}
        }
        ctx.request_repaint();
    }

    fn snapshot_options(&self, windows: bool) -> capture::SnapshotOptions {
        capture::SnapshotOptions { windows, pointer: self.config.capture_pointer }
    }

    /// Start a capture now, or after the configured delay. Menu clicks wait
    /// at least long enough for the menu to disappear from the screen.
    fn request_capture(&mut self, mode: CaptureMode, from_menu: bool, ctx: &egui::Context) {
        let mut delay = Duration::from_secs(self.config.capture_delay_secs as u64);
        if from_menu {
            delay = delay.max(Duration::from_millis(250));
        }
        if delay.is_zero() {
            self.start_capture(mode, ctx);
        } else {
            log::info!("{} in {:.1}s", mode.label(), delay.as_secs_f32());
            self.pending_capture = Some((mode, Instant::now() + delay));
            ctx.request_repaint_after(delay);
        }
    }

    fn start_capture(&mut self, mode: CaptureMode, ctx: &egui::Context) {
        if self.overlay.is_some() {
            return;
        }
        // Without permission macOS returns only the wallpaper, which looks like
        // a broken screenshot. Explain instead of capturing.
        // Debug builds can set RUSTSHOT_IGNORE_PERMISSION=1 to exercise the
        // overlay under a debugger, which never has Screen Recording access.
        let ignore_permission = cfg!(debug_assertions) && std::env::var_os("RUSTSHOT_IGNORE_PERMISSION").is_some();
        if !ignore_permission && !permissions::has_screen_capture_access() {
            log::warn!("capture blocked: no Screen Recording permission");
            self.permission_notice();
            platform::activate_app();
            ctx.request_repaint();
            return;
        }
        let result = match mode {
            CaptureMode::Region | CaptureMode::Window => capture::take_snapshot(self.snapshot_options(true)).map(|snap| {
                self.overlay = Some(Overlay::new(snap, mode == CaptureMode::Window, self.config.show_magnifier));
            }),
            CaptureMode::Fullscreen => capture::take_snapshot(self.snapshot_options(false)).map(|snap| {
                if let Some(image) = snap.stitch_all() {
                    self.deliver(Capture { image, title: tr!("title-full-screen"), taken_at: snap.taken_at });
                }
            }),
            CaptureMode::LastRegion => match self.config.last_region {
                None => {
                    self.start_capture(CaptureMode::Region, ctx);
                    Ok(())
                }
                Some(region) => capture::take_snapshot(self.snapshot_options(false)).map(|snap| {
                    if let Some(image) = snap.crop(&region) {
                        self.deliver(Capture { image, title: String::new(), taken_at: snap.taken_at });
                    }
                }),
            },
        };
        if let Err(e) = result {
            log::error!("capture failed: {e:#}");
            self.notice(&tr!("notice-capture-failed"), &format!("{e:#}"), false);
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
            Destination::Clipboard => {
                output::copy_image_to_clipboard(&cap.image).map(|_| self.notify(&tr!("toast-copied"), None))
            }
            Destination::File => {
                let path = output::auto_path(&self.config, &cap.title, cap.taken_at);
                save_and_report(&self.config, &cap.image, &path).map(|p| self.notify_saved(p))
            }
            Destination::FileAs => match ask_save_path(&self.config, &cap.title, cap.taken_at) {
                Some(path) => output::save_to(&cap.image, &path, &self.config).map(|_| self.notify_saved(path)),
                None => Ok(()),
            },
        };
        if let Err(e) = result {
            log::error!("{}: {e:#}", dest.label());
            self.notice(&dest.label(), &format!("{e:#}"), false);
        }
    }

    fn open_editor(&mut self, image: RgbaImage, title: &str, taken_at: DateTime<Local>) {
        self.next_id += 1;
        let title = if title.trim().is_empty() { tr!("title-screenshot") } else { title.to_string() };
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
                ui.label(egui::RichText::new(tr!("picker-heading", width = w, height = h)).strong());
                ui.add_space(6.0);
                let full = |ui: &mut egui::Ui, text: &str| {
                    ui.add_sized([ui.available_width(), 28.0], egui::Button::new(text)).clicked()
                };
                if full(ui, &tr!("dest-editor")) {
                    choice = Some(Some(Destination::Editor));
                }
                if full(ui, &tr!("dest-clipboard")) {
                    choice = Some(Some(Destination::Clipboard));
                }
                if full(ui, &tr!("dest-file")) {
                    choice = Some(Some(Destination::File));
                }
                if full(ui, &tr!("dest-file-as")) {
                    choice = Some(Some(Destination::FileAs));
                }
                ui.add_space(4.0);
                if full(ui, &tr!("button-cancel")) {
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
        let mut toasts: Vec<(String, Option<PathBuf>)> = Vec::new();
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
                    EditorRequest::Save | EditorRequest::SaveAs => {
                        save_editor(config, ew, req == EditorRequest::SaveAs).map(|saved| {
                            if let Some(path) = saved {
                                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                toasts.push((tr!("toast-saved", name = name), Some(path)));
                            }
                        })
                    }
                    EditorRequest::Copy => output::copy_image_to_clipboard(&ew.editor.flatten()).map(|_| {
                        ew.editor.set_status(tr!("toast-copied"));
                        toasts.push((tr!("toast-copied"), None));
                    }),
                    EditorRequest::Close => {
                        close = true;
                        Ok(())
                    }
                };
                if let Err(e) = result {
                    ew.editor.set_status(tr!("status-error", error = format!("{e:#}")));
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
        for (message, path) in toasts {
            self.notify(&message, path);
        }
    }

    fn show_settings(&mut self, ctx: &egui::Context) {
        let Some(settings) = &mut self.settings else { return };
        let out = settings.show(ctx, &self.config, &self.hotkey_errors);
        if let Some(new) = out.config {
            let hotkeys_changed = new.hotkeys != self.config.hotkeys;
            let language_changed = new.language != self.config.language;
            if language_changed {
                rustshot_i18n::set_language(new.language.as_deref());
            }
            let tray_changed = hotkeys_changed || language_changed || new.tray_click != self.config.tray_click;
            self.config = new;
            if let Err(e) = self.config.save() {
                log::error!("saving settings: {e:#}");
            }
            if hotkeys_changed && !out.recording {
                self.hotkey_errors = self.hotkeys.register(&self.config.hotkeys);
            }
            if tray_changed {
                // Rebuild the tray so menu shortcuts and click behaviour match.
                self.tray = None;
                self.tray = Tray::new(&self.config).map_err(|e| log::error!("tray icon: {e}")).ok();
            }
        }
        // Pause global hotkeys while a new shortcut is being recorded, so
        // pressing the old one doesn't start a capture.
        if out.recording && !self.hotkeys_paused {
            self.hotkeys.suspend();
            self.hotkeys_paused = true;
        } else if !out.recording && self.hotkeys_paused {
            self.hotkey_errors = self.hotkeys.register(&self.config.hotkeys);
            self.hotkeys_paused = false;
        }
        if out.close {
            self.settings = None;
            if self.hotkeys_paused {
                self.hotkey_errors = self.hotkeys.register(&self.config.hotkeys);
                self.hotkeys_paused = false;
            }
        }
    }

    fn show_notices(&mut self, ctx: &egui::Context) {
        let mut done = Vec::new();
        let mut quit = false;
        for (i, n) in self.notices.iter_mut().enumerate() {
            let size = if n.permission { [500.0, 380.0] } else { [420.0, 220.0] };
            let builder = egui::ViewportBuilder::default()
                .with_title(&n.title)
                .with_inner_size(size)
                .with_window_level(egui::WindowLevel::AlwaysOnTop);
            let close = ctx.show_viewport_immediate(n.id, builder, |ui, _| {
                let mut close = ui.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape));
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.heading(&n.title);
                    ui.add_space(6.0);
                    ui.label(&n.body);
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        if n.permission {
                            if ui.button(tr!("permission-open-settings")).clicked() {
                                permissions::open_privacy_settings();
                            }
                            if ui
                                .button(tr!("permission-reset"))
                                .on_hover_text(tr!("permission-reset-tip"))
                                .clicked()
                            {
                                // macOS shows its own prompt next; get out of its way.
                                permissions::reset_screen_capture_access();
                                close = true;
                            }
                            if ui.button(tr!("menu-quit")).clicked() {
                                quit = true;
                            }
                        }
                        if ui.button(if n.permission { tr!("button-close") } else { tr!("button-ok") }).clicked() {
                            close = true;
                        }
                    });
                });
                close
            });
            if !n.shown {
                ctx.send_viewport_cmd_to(n.id, egui::ViewportCommand::Focus);
                platform::activate_app();
                n.shown = true;
            }
            if close {
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            self.notices.remove(i);
        }
        if quit {
            self.quit = true;
            ctx.request_repaint();
        }
    }
}

fn image_rgba(n: u64) -> rustshot_core::image::Rgba<u8> {
    rustshot_core::image::Rgba([(n * 37 % 255) as u8, 120, 200, 255])
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

/// Save an editor's image. Returns the path, or `None` if the user cancelled.
fn save_editor(config: &Config, ew: &mut EditorWindow, ask: bool) -> anyhow::Result<Option<PathBuf>> {
    let path = if ask {
        match ask_save_path(config, &ew.editor.title, ew.taken_at) {
            Some(p) => p,
            None => return Ok(None),
        }
    } else {
        match &ew.path {
            Some(p) => p.clone(),
            None => output::auto_path(config, &ew.editor.title, ew.taken_at),
        }
    };
    let path = save_and_report(config, &ew.editor.flatten(), &path)?;
    ew.editor.mark_saved();
    ew.editor.set_status(tr!("status-saved-to", path = path.display().to_string()));
    ew.path = Some(path.clone());
    Ok(Some(path))
}

/// Ask about unsaved changes. Returns true if the editor may close.
fn confirm_close(config: &Config, ew: &mut EditorWindow) -> bool {
    if !ew.editor.is_dirty() {
        return true;
    }
    let answer = rfd::MessageDialog::new()
        .set_title("Rustshot")
        .set_description(tr!("confirm-save-changes"))
        .set_buttons(rfd::MessageButtons::YesNoCancel)
        .show();
    match answer {
        rfd::MessageDialogResult::Yes => match save_editor(config, ew, false) {
            Ok(_) => true,
            Err(e) => {
                ew.editor.set_status(tr!("status-error", error = format!("{e:#}")));
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
        self.stress_step(ctx);
        if self.quit {
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.background_hidden {
            // Again here in case the window didn't exist yet in `App::new`.
            platform::hide_background_window();
            self.background_hidden = true;
        }
        self.show_overlay(&ctx);
        self.show_picker(&ctx);
        self.show_editors(&ctx);
        self.show_settings(&ctx);
        self.show_notices(&ctx);
        self.show_toast(&ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
}
