//! The Settings window: a sidebar of sections; every change applies at once.

use std::sync::{Arc, Mutex};

use egui::{Color32, RichText, Ui};
use egui_phosphor::regular as icon;
use rustshot_core::config::{Hotkeys, RecordingSettings};
use rustshot_core::recording::audio::{self, Microphone, MicrophoneAccess};
use rustshot_core::filename::{self, PatternContext};
use rustshot_core::update::{self, UpdateStatus};
use rustshot_core::{autostart, disk, recording, Config, Destination, OutputFormat, TrayClick, VideoFormat, VideoQuality};

use crate::{hotkeys, platform};
use rustshot_i18n::tr;

/// The project on GitHub.
const REPO_URL: &str = "https://github.com/moridin1138/rustshot";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    General,
    Capture,
    Recording,
    Shortcuts,
    Saving,
    About,
}

impl Section {
    const ALL: [Section; 6] =
        [Section::General, Section::Capture, Section::Recording, Section::Shortcuts, Section::Saving, Section::About];

    fn label(self) -> String {
        match self {
            Section::General => tr!("settings-general"),
            Section::Capture => tr!("settings-capture"),
            Section::Recording => tr!("settings-recording"),
            Section::Shortcuts => tr!("settings-shortcuts"),
            Section::Saving => tr!("settings-saving"),
            Section::About => tr!("settings-about"),
        }
    }

    /// File name for the design tour pictures (not translated).
    fn slug(self) -> &'static str {
        match self {
            Section::General => "general",
            Section::Capture => "capture",
            Section::Recording => "recording",
            Section::Shortcuts => "shortcuts",
            Section::Saving => "saving",
            Section::About => "about",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Section::General => icon::SLIDERS_HORIZONTAL,
            Section::Capture => icon::CROP,
            Section::Recording => icon::VIDEO_CAMERA,
            Section::Shortcuts => icon::KEYBOARD,
            Section::Saving => icon::FOLDER,
            Section::About => icon::INFO,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HotkeyField {
    Region,
    Window,
    Fullscreen,
    LastRegion,
    Record,
}

impl HotkeyField {
    const ALL: [HotkeyField; 5] =
        [HotkeyField::Region, HotkeyField::Window, HotkeyField::Fullscreen, HotkeyField::LastRegion, HotkeyField::Record];

    fn label(self) -> String {
        match self {
            HotkeyField::Region => tr!("capture-region"),
            HotkeyField::Window => tr!("capture-window"),
            HotkeyField::Fullscreen => tr!("capture-fullscreen"),
            HotkeyField::LastRegion => tr!("capture-last-region"),
            HotkeyField::Record => tr!("shortcut-record"),
        }
    }

    fn get(self, h: &mut Hotkeys) -> &mut String {
        match self {
            HotkeyField::Region => &mut h.region,
            HotkeyField::Window => &mut h.window,
            HotkeyField::Fullscreen => &mut h.fullscreen,
            HotkeyField::LastRegion => &mut h.last_region,
            HotkeyField::Record => &mut h.record,
        }
    }
}

#[derive(Clone, Debug, Default)]
enum UpdateCheck {
    #[default]
    Idle,
    Checking,
    Done(Result<UpdateStatus, String>),
}

/// What the app should do after a frame of the Settings window.
#[derive(Default)]
pub struct SettingsOutput {
    /// The settings changed: apply and save these.
    pub config: Option<Config>,
    pub close: bool,
    /// A shortcut is being recorded: global hotkeys should be paused.
    pub recording: bool,
}

pub struct SettingsWindow {
    section: Section,
    focus: bool,
    recording: Option<HotkeyField>,
    record_hint: Option<String>,
    autostart: bool,
    autostart_error: Option<String>,
    update: Arc<Mutex<UpdateCheck>>,
    /// Primary display size in pixels and its scale, for size estimates.
    display: Option<(u32, u32, f64)>,
    /// Connected microphones, listed when first needed.
    microphones: Option<Vec<Microphone>>,
    /// Debug builds: RUSTSHOT_SETTINGS_TOUR=<dir> saves a picture of each
    /// section (just this window), for reviewing the design.
    tour: Option<Tour>,
}

struct Tour {
    dir: std::path::PathBuf,
    index: usize,
    frames: u32,
}

const WINDOW_SIZE: [f32; 2] = [720.0, 540.0];
const ROW_HEIGHT: f32 = 30.0;

impl SettingsWindow {
    pub fn new() -> Self {
        Self {
            section: Section::General,
            focus: true,
            recording: None,
            record_hint: None,
            autostart: autostart::is_enabled(),
            autostart_error: None,
            update: Arc::default(),
            display: rustshot_core::capture::primary_display_pixels(),
            microphones: None,
            tour: cfg!(debug_assertions)
                .then(|| std::env::var_os("RUSTSHOT_SETTINGS_TOUR"))
                .flatten()
                .map(|dir| Tour { dir: dir.into(), index: 0, frames: 0 }),
        }
    }

    /// Step through the sections, photographing just this window for each
    /// (macOS `screencapture -l`, which ignores whatever covers the window).
    fn run_tour(&mut self, ui: &mut Ui, out: &mut SettingsOutput) {
        let Some(tour) = &mut self.tour else { return };
        if tour.index >= Section::ALL.len() {
            self.tour = None;
            out.close = true;
            return;
        }
        self.section = Section::ALL[tour.index];
        tour.frames += 1;
        if tour.frames == 8 {
            let path = tour.dir.join(format!("{}.png", self.section.slug()));
            match platform::window_number(&tr!("settings-title")) {
                Some(n) => {
                    let status = std::process::Command::new("screencapture")
                        .args(["-x", "-o", &format!("-l{n}")])
                        .arg(&path)
                        .status();
                    log::info!("settings tour: {} -> {status:?}", path.display());
                }
                None => log::warn!("settings tour: window not found"),
            }
            tour.index += 1;
            tour.frames = 0;
        }
        ui.ctx().request_repaint();
    }

    pub fn show(&mut self, ctx: &egui::Context, config: &Config, hotkey_errors: &[String]) -> SettingsOutput {
        let id = egui::ViewportId::from_hash_of("rustshot-settings");
        let builder = egui::ViewportBuilder::default()
            .with_title(tr!("settings-title"))
            .with_inner_size(WINDOW_SIZE)
            .with_min_inner_size([600.0, 420.0]);
        let mut out = SettingsOutput::default();
        // Work on a fresh copy each frame so values changed elsewhere (such as
        // the last captured region) are never overwritten with stale ones.
        let mut draft = config.clone();
        ctx.show_viewport_immediate(id, builder, |ui, _| {
            self.run_tour(ui, &mut out);
            if self.recording.is_some() {
                self.record_shortcut(ui, &mut draft);
            } else if ui.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape)) {
                out.close = true;
            }
            egui::Panel::left("settings_nav")
                .resizable(false)
                .exact_size(180.0)
                .frame(egui::Frame::new().fill(ui.visuals().faint_bg_color).inner_margin(10.0))
                .show(ui, |ui| self.sidebar(ui));
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(ui.visuals().panel_fill).inner_margin(egui::Margin::symmetric(24, 18)))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        ui.set_max_width(ui.available_width());
                        ui.heading(self.section.label());
                        ui.add_space(12.0);
                        match self.section {
                            Section::General => self.general(ui, &mut draft),
                            Section::Capture => capture(ui, &mut draft),
                            Section::Recording => recording(ui, &mut draft, self.display, &mut self.microphones),
                            Section::Shortcuts => self.shortcuts(ui, &mut draft, hotkey_errors),
                            Section::Saving => saving(ui, &mut draft),
                            Section::About => self.about(ui, &mut draft),
                        }
                    });
                });
        });
        if self.focus {
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
            self.focus = false;
        }
        if draft != *config {
            out.config = Some(draft);
        }
        out.recording = self.recording.is_some();
        out
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        for section in Section::ALL {
            let selected = self.section == section;
            let text = RichText::new(format!("{}   {}", section.icon(), section.label())).size(14.0);
            let button = egui::Button::selectable(selected, text).min_size(egui::vec2(ui.available_width(), 30.0));
            if ui.add(button).clicked() {
                self.section = section;
                self.recording = None;
            }
        }
    }

    fn general(&mut self, ui: &mut Ui, d: &mut Config) {
        group(ui, |ui| {
            row(ui, &tr!("settings-language"), None, |ui| {
                let system = rustshot_i18n::LANGUAGES.iter().find(|l| l.code == rustshot_i18n::system_language());
                let system_label = tr!("settings-language-system", language = system.map_or("English", |l| l.name));
                let selected = match &d.language {
                    Some(code) => rustshot_i18n::LANGUAGES.iter().find(|l| l.code == code).map_or(system_label.clone(), |l| l.name.to_string()),
                    None => system_label.clone(),
                };
                egui::ComboBox::from_id_salt("language").selected_text(selected).width(170.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut d.language, None, system_label);
                    for lang in rustshot_i18n::LANGUAGES {
                        ui.selectable_value(&mut d.language, Some(lang.code.to_string()), lang.name);
                    }
                });
            });
        });

        caption(ui, &tr!("settings-after-capture"));
        group(ui, |ui| {
            let mut ask = d.destinations.contains(&Destination::Picker);
            row(ui, &tr!("settings-ask"), Some(&tr!("settings-ask-sub")), |ui| {
                if toggle(ui, &mut ask).changed() {
                    d.destinations = if ask { vec![Destination::Picker] } else { vec![Destination::Editor] };
                }
            });
            divider(ui);
            ui.add_enabled_ui(!ask, |ui| {
                ui.add_space(6.0);
                ui.weak(if ask { tr!("settings-otherwise-always") } else { tr!("settings-always") });
                ui.horizontal_wrapped(|ui| {
                    let short = |dest: Destination| match dest {
                        Destination::Editor => tr!("settings-dest-editor"),
                        Destination::Clipboard => tr!("settings-dest-copy"),
                        Destination::File => tr!("settings-dest-folder"),
                        other => other.label(),
                    };
                    let choices = Destination::ALL.into_iter().filter(|d| *d != Destination::Picker && d.available());
                    for (dest, label) in choices.map(|d| (d, format!("{}  {}", crate::app::dest_icon(d), short(d)))) {
                        let on = d.destinations.contains(&dest);
                        if ui.add(egui::Button::selectable(on, label)).clicked() {
                            if on {
                                // Keep at least one destination.
                                if d.destinations.len() > 1 {
                                    d.destinations.retain(|x| *x != dest);
                                }
                            } else {
                                d.destinations.retain(|x| *x != Destination::Picker);
                                d.destinations.push(dest);
                            }
                        }
                    }
                });
                ui.add_space(6.0);
            });
            divider(ui);
            row(ui, &tr!("settings-open-with"), Some(&tr!("settings-open-with-sub")), |ui| {
                if d.external_app.is_some() && ui.button(tr!("button-default")).clicked() {
                    d.external_app = None;
                }
                if ui.button(tr!("button-change")).clicked() {
                    let mut dialog = rfd::FileDialog::new();
                    if cfg!(target_os = "macos") {
                        dialog = dialog.set_directory("/Applications").add_filter("App", &["app"]);
                    } else if cfg!(windows) {
                        dialog = dialog.add_filter("Program", &["exe"]);
                    }
                    if let Some(app) = dialog.pick_file() {
                        d.external_app = Some(app);
                    }
                }
                let name = match &d.external_app {
                    None => tr!("settings-open-with-default"),
                    Some(app) => app.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                };
                ui.label(RichText::new(name).weak());
            });
        });

        caption(ui, &if cfg!(target_os = "macos") { tr!("settings-menubar-icon") } else { tr!("settings-tray-icon") });
        group(ui, |ui| {
            row(ui, &tr!("settings-icon-click"), None, |ui| {
                egui::ComboBox::from_id_salt("tray_click")
                    .selected_text(d.tray_click.label())
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        for c in TrayClick::ALL {
                            ui.selectable_value(&mut d.tray_click, c, c.label());
                        }
                    });
            });
            if d.tray_click != TrayClick::ShowMenu {
                ui.label(RichText::new(tr!("settings-right-click-hint")).small().weak());
                ui.add_space(4.0);
            }
            divider(ui);
            row(ui, &tr!("settings-notifications"), Some(&tr!("settings-notifications-sub")), |ui| {
                toggle(ui, &mut d.notifications);
            });
            divider(ui);
            row(ui, &tr!("settings-open-at-login"), None, |ui| {
                let mut on = self.autostart;
                if toggle(ui, &mut on).changed() {
                    match autostart::set_enabled(on) {
                        Ok(()) => {
                            self.autostart = on;
                            self.autostart_error = None;
                        }
                        Err(e) => self.autostart_error = Some(format!("{e:#}")),
                    }
                }
            });
            if let Some(e) = &self.autostart_error {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            divider(ui);
            row(ui, &tr!("settings-check-updates"), Some(&tr!("settings-check-updates-sub")), |ui| {
                toggle(ui, &mut d.check_for_updates);
            });
        });
    }

    fn shortcuts(&mut self, ui: &mut Ui, d: &mut Config, errors: &[String]) {
        group(ui, |ui| {
            for (i, field) in HotkeyField::ALL.into_iter().enumerate() {
                if i > 0 {
                    divider(ui);
                }
                row(ui, &field.label(), None, |ui| {
                    let recording = self.recording == Some(field);
                    let text = if recording {
                        RichText::new(tr!("shortcut-press-keys")).color(ui.visuals().selection.stroke.color)
                    } else {
                        let value = field.get(&mut d.hotkeys).clone();
                        if value.is_empty() {
                            RichText::new(tr!("shortcut-none")).weak()
                        } else {
                            RichText::new(hotkeys::keycaps(&value).join("  ")).monospace()
                        }
                    };
                    let button = egui::Button::selectable(recording, text).min_size(egui::vec2(140.0, 24.0));
                    if ui.add(button).on_hover_text(tr!("shortcut-tip")).clicked() {
                        self.recording = if recording { None } else { Some(field) };
                        self.record_hint = None;
                    }
                });
            }
        });
        ui.add_space(4.0);
        let help = if self.recording.is_some() {
            tr!("shortcut-help-recording")
        } else {
            tr!("shortcut-help")
        };
        ui.weak(help);
        if let Some(hint) = &self.record_hint {
            ui.colored_label(ui.visuals().warn_fg_color, hint);
        }
        for e in errors {
            ui.colored_label(ui.visuals().error_fg_color, format!("{} {e}", icon::WARNING));
        }
        ui.add_space(12.0);
        if ui.button(tr!("shortcut-restore")).clicked() {
            d.hotkeys = Hotkeys::default();
            self.recording = None;
        }
    }

    /// While recording, the next key press (with modifiers) becomes the shortcut.
    fn record_shortcut(&mut self, ui: &mut Ui, d: &mut Config) {
        let Some(field) = self.recording else { return };
        let pressed = ui.input_mut(|i| {
            let found = i.events.iter().find_map(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            });
            // Keep the key away from the rest of the window.
            i.events.retain(|e| !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_)));
            found
        });
        let Some((key, mods)) = pressed else { return };
        match key {
            egui::Key::Escape if mods.is_none() => self.recording = None,
            egui::Key::Backspace | egui::Key::Delete if mods.is_none() => {
                field.get(&mut d.hotkeys).clear();
                self.recording = None;
            }
            _ => match hotkeys::from_key_press(key, mods) {
                Some(text) => {
                    *field.get(&mut d.hotkeys) = text;
                    self.recording = None;
                    self.record_hint = None;
                }
                None => {
                    self.record_hint = Some(if cfg!(target_os = "macos") {
                        tr!("shortcut-need-modifier-mac")
                    } else {
                        tr!("shortcut-need-modifier")
                    })
                }
            },
        }
    }

    fn about(&mut self, ui: &mut Ui, d: &mut Config) {
        group(ui, |ui| {
            row(ui, &format!("Rustshot {}", env!("CARGO_PKG_VERSION")), Some("PolyForm Shield License 1.0.0"), |ui| {
                let checking = matches!(*self.update.lock().unwrap(), UpdateCheck::Checking);
                if ui.add_enabled(!checking, egui::Button::new(tr!("update-check"))).clicked() {
                    self.start_update_check(ui.ctx().clone());
                }
            });
            let status = self.update.lock().unwrap().clone();
            match status {
                UpdateCheck::Idle => {}
                UpdateCheck::Checking => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.weak(tr!("update-checking"));
                    });
                }
                UpdateCheck::Done(Ok(UpdateStatus::UpToDate { .. })) => {
                    ui.label(format!("{} {}", icon::CHECK_CIRCLE, tr!("update-latest")));
                }
                UpdateCheck::Done(Ok(UpdateStatus::Available { version, url })) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{} {}", icon::ARROW_CIRCLE_UP, tr!("update-available", version = version.clone()))).strong());
                        if ui.link(tr!("update-download")).clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        }
                    });
                }
                UpdateCheck::Done(Err(e)) => {
                    ui.colored_label(ui.visuals().warn_fg_color, tr!("update-failed", error = e));
                }
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button(format!("{}  {}", icon::GITHUB_LOGO, tr!("about-view-github"))).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(REPO_URL));
                }
                if ui.button(format!("{}  {}", icon::NOTE, tr!("about-release-notes"))).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(update::RELEASES_URL));
                }
                if ui.button(format!("{}  {}", icon::BUG, tr!("about-report-problem"))).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(format!("{REPO_URL}/issues")));
                }
            });
            ui.add_space(4.0);
            divider(ui);
            row(ui, &tr!("about-settings-file"), None, |ui| {
                if let Some(path) = Config::path()
                    && ui.button(if cfg!(target_os = "macos") { tr!("button-show-in-finder") } else { tr!("button-show-file") }).clicked()
                {
                    if !path.exists() {
                        let _ = d.save();
                    }
                    platform::reveal(&path);
                }
            });
        });
        ui.add_space(12.0);
        if ui.button(tr!("about-reset-all")).on_hover_text(tr!("about-reset-all-tip")).clicked() {
            let last = d.last_region;
            *d = Config { last_region: last, ..Config::default() };
        }
        ui.add_space(16.0);
        ui.weak(tr!("about-credits"));
    }

    fn start_update_check(&self, ctx: egui::Context) {
        *self.update.lock().unwrap() = UpdateCheck::Checking;
        let state = Arc::clone(&self.update);
        std::thread::spawn(move || {
            let result = update::check(env!("CARGO_PKG_VERSION")).map_err(|e| format!("{e:#}"));
            *state.lock().unwrap() = UpdateCheck::Done(result);
            ctx.request_repaint();
        });
    }
}

fn capture(ui: &mut Ui, d: &mut Config) {
    group(ui, |ui| {
        row(ui, &tr!("settings-magnifier"), None, |ui| {
            toggle(ui, &mut d.show_magnifier);
        });
        divider(ui);
        row(ui, &tr!("settings-pointer"), None, |ui| {
            toggle(ui, &mut d.capture_pointer);
        });
        divider(ui);
        row(ui, &tr!("settings-delay"), Some(&tr!("settings-delay-sub")), |ui| {
            let secs = |n: u32| tr!("settings-seconds", n = n);
            let mut options = vec![(0u32, tr!("settings-off")), (3, secs(3)), (5, secs(5)), (10, secs(10))];
            if !options.iter().any(|(v, _)| *v == d.capture_delay_secs) {
                options.push((d.capture_delay_secs, secs(d.capture_delay_secs)));
            }
            segmented(ui, &mut d.capture_delay_secs, &options);
        });
    });
}

fn recording(ui: &mut Ui, d: &mut Config, display: Option<(u32, u32, f64)>, microphones: &mut Option<Vec<Microphone>>) {
    let r = &mut d.recording;
    let mp4_available = recording::supports(VideoFormat::Mp4);
    group(ui, |ui| {
        row(ui, &tr!("rec-format"), None, |ui| {
            let mut options = vec![(VideoFormat::Gif, "GIF".to_string())];
            if mp4_available {
                options.insert(0, (VideoFormat::Mp4, "MP4".to_string()));
            }
            segmented(ui, &mut r.format, &options);
        });
        let hint = match r.format {
            VideoFormat::Mp4 => tr!("rec-format-mp4-hint"),
            VideoFormat::Gif if mp4_available => tr!("rec-format-gif-hint"),
            VideoFormat::Gif => tr!("rec-format-gif-only-hint"),
        };
        ui.label(RichText::new(hint).small().weak());
        ui.add_space(4.0);
        divider(ui);
        if r.format == VideoFormat::Mp4 {
            row(ui, &tr!("rec-quality"), Some(&tr!("rec-quality-sub")), |ui| {
                let options: Vec<(VideoQuality, String)> = VideoQuality::ALL
                    .iter()
                    .map(|q| {
                        (*q, match q {
                            VideoQuality::Low => tr!("rec-quality-low"),
                            VideoQuality::Medium => tr!("rec-quality-medium"),
                            VideoQuality::High => tr!("rec-quality-high"),
                        })
                    })
                    .collect();
                segmented(ui, &mut r.quality, &options);
            });
            divider(ui);
        }
        row(ui, &tr!("rec-full-resolution"), Some(&tr!("rec-full-resolution-sub")), |ui| {
            toggle(ui, &mut r.full_resolution);
        });
        divider(ui);
        row(ui, &tr!("rec-fps"), None, |ui| {
            let choices: &[u32] = if r.format == VideoFormat::Gif { &[10, 15] } else { &[15, 30, 60] };
            let mut options: Vec<(u32, String)> = choices.iter().map(|f| (*f, tr!("rec-fps-value", fps = *f))).collect();
            if !choices.contains(&r.effective_fps()) {
                options.push((r.fps, tr!("rec-fps-value", fps = r.effective_fps())));
            }
            segmented(ui, &mut r.fps, &options);
        });
        divider(ui);
        row(ui, &tr!("rec-show-clicks"), Some(&tr!("rec-show-clicks-sub")), |ui| {
            toggle(ui, &mut r.show_clicks);
        });
        if r.format == VideoFormat::Mp4 && audio::supported() {
            divider(ui);
            sound(ui, r, microphones);
        }
        if let Some((w, h, scale)) = display {
            divider(ui);
            let per_minute = disk::format_bytes(r.bytes_per_minute(w, h, scale));
            let (ow, oh) = r.output_size(w, h, scale);
            ui.add_space(4.0);
            ui.label(format!("{}  {}", icon::HARD_DRIVES, tr!("rec-estimate", size = per_minute)));
            ui.label(RichText::new(tr!("rec-estimate-sub", width = ow, height = oh, fps = r.effective_fps())).small().weak());
            ui.add_space(4.0);
        }
    });
    ui.weak(tr!("rec-shortcut-hint"));
}

/// Microphone and system sound (MP4 only).
fn sound(ui: &mut Ui, r: &mut RecordingSettings, microphones: &mut Option<Vec<Microphone>>) {
    row(ui, &tr!("rec-microphone"), Some(&tr!("rec-microphone-sub")), |ui| {
        // Ask now, while the prompt can be answered, never mid-recording.
        if toggle(ui, &mut r.microphone).changed() && r.microphone {
            audio::request_microphone_access();
        }
    });
    if r.microphone && audio::microphone_access() == MicrophoneAccess::Denied {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{}  {}", icon::WARNING, tr!("rec-microphone-denied"))).small());
            if ui.small_button(tr!("button-open-settings")).clicked() {
                rustshot_core::permissions::open_microphone_settings();
            }
        });
    }
    if r.microphone {
        let mics = microphones.get_or_insert_with(audio::microphones);
        row(ui, &tr!("rec-microphone-device"), None, |ui| {
            let default = tr!("rec-microphone-default");
            let selected = match &r.microphone_device {
                None => default.clone(),
                Some(id) => mics
                    .iter()
                    .find(|m| &m.id == id)
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| tr!("rec-microphone-missing")),
            };
            egui::ComboBox::from_id_salt("microphone").selected_text(selected).width(220.0).show_ui(ui, |ui| {
                ui.selectable_value(&mut r.microphone_device, None, default);
                for m in mics.iter() {
                    ui.selectable_value(&mut r.microphone_device, Some(m.id.clone()), &m.name);
                }
            });
        });
    }
    divider(ui);
    let system_ok = audio::system_audio_supported();
    let sub = if system_ok { tr!("rec-system-audio-sub") } else { tr!("rec-system-audio-unavailable") };
    row(ui, &tr!("rec-system-audio"), Some(&sub), |ui| {
        ui.add_enabled_ui(system_ok, |ui| {
            let mut on = r.system_audio && system_ok;
            if toggle(ui, &mut on).changed() {
                r.system_audio = on;
            }
        });
    });
}

fn saving(ui: &mut Ui, d: &mut Config) {
    group(ui, |ui| {
        row(ui, &tr!("settings-folder"), None, |ui| {
            if ui.button(tr!("button-show")).clicked() {
                let _ = std::fs::create_dir_all(&d.output_dir);
                platform::open_folder(&d.output_dir);
            }
            if ui.button(tr!("button-change")).clicked()
                && let Some(dir) = rfd::FileDialog::new().set_directory(&d.output_dir).pick_folder()
            {
                d.output_dir = dir;
            }
            ui.add(egui::Label::new(RichText::new(short_path(&d.output_dir)).weak()).truncate());
        });
        if let Some(space) = disk::space_for(&d.output_dir) {
            let text = tr!(
                "settings-free-space-value",
                free = disk::format_bytes(space.available),
                total = disk::format_bytes(space.total),
                percent = format!("{:.0}", space.fraction_free() * 100.0)
            );
            row(ui, &tr!("settings-free-space"), None, |ui| {
                if space.is_low() {
                    ui.colored_label(ui.visuals().warn_fg_color, format!("{} {text}", icon::WARNING));
                } else {
                    ui.weak(text);
                }
            });
            if space.is_low() {
                ui.label(RichText::new(tr!("settings-free-space-low")).small().color(ui.visuals().warn_fg_color));
            }
        }
        divider(ui);
        ui.add_space(6.0);
        ui.label(tr!("settings-file-name"));
        ui.add(egui::TextEdit::singleline(&mut d.filename_pattern).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
        ui.horizontal_wrapped(|ui| {
            for (label, token) in [
                (tr!("token-date"), "${YYYY}-${MM}-${DD}"),
                (tr!("token-time"), "${hh}.${mm}.${ss}"),
                (tr!("token-title"), "${title}"),
                (tr!("token-user"), "${user}"),
            ] {
                if ui.small_button(format!("+ {label}")).clicked() {
                    if !d.filename_pattern.is_empty() && !d.filename_pattern.ends_with(' ') {
                        d.filename_pattern.push(' ');
                    }
                    d.filename_pattern.push_str(token);
                }
            }
            if ui.small_button(tr!("button-default")).on_hover_text(filename::DEFAULT_PATTERN).clicked() {
                d.filename_pattern = filename::DEFAULT_PATTERN.into();
            }
        });
        let example = filename::expand(&d.filename_pattern, &PatternContext { time: chrono::Local::now(), title: "Safari" });
        ui.weak(tr!("settings-example", name = format!("{example}.{}", d.format.extension())));
        ui.add_space(6.0);
        divider(ui);
        row(ui, &tr!("settings-format"), None, |ui| {
            let options: Vec<(OutputFormat, String)> =
                OutputFormat::ALL.iter().map(|f| (*f, f.extension().to_uppercase())).collect();
            segmented(ui, &mut d.format, &options);
        });
        if d.format == OutputFormat::Jpg {
            row(ui, &tr!("settings-jpeg-quality"), None, |ui| {
                ui.add(egui::Slider::new(&mut d.jpeg_quality, 1..=100));
            });
        }
        divider(ui);
        row(ui, &tr!("settings-copy-path"), None, |ui| {
            toggle(ui, &mut d.copy_path_after_save);
        });
    });
}

/// `~/…` instead of the full home path.
fn short_path(p: &std::path::Path) -> String {
    let home = directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf());
    match home.and_then(|h| p.strip_prefix(h).ok().map(|r| r.to_path_buf())) {
        Some(rest) => format!("~/{}", rest.display()),
        None => p.display().to_string(),
    }
}

// ---- small widgets ----

fn caption(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).small().weak());
    ui.add_space(2.0);
}

/// A rounded box grouping related rows.
fn group(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::group(ui.style())
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    ui.add_space(14.0);
}

fn divider(ui: &mut Ui) {
    ui.add(egui::Separator::default().spacing(4.0));
}

/// Label (and optional explanation) on the left, control on the right.
fn row(ui: &mut Ui, label: &str, sub: Option<&str>, control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_height(ROW_HEIGHT);
        ui.vertical(|ui| {
            ui.add_space(if sub.is_some() { 2.0 } else { 6.0 });
            ui.label(label);
            if let Some(sub) = sub {
                ui.label(RichText::new(sub).small().weak());
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control);
    });
}

/// An on/off switch.
fn toggle(ui: &mut Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(36.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let off_bg = ui.visuals().widgets.inactive.bg_fill;
        let on_bg = ui.visuals().selection.bg_fill;
        let bg = lerp_color(off_bg, on_bg, t);
        let radius = rect.height() / 2.0;
        ui.painter().rect(rect, radius, bg, visuals.bg_stroke, egui::StrokeKind::Inside);
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter().circle_filled(egui::pos2(x, rect.center().y), radius - 3.0, Color32::WHITE);
    }
    response
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// A row of joined buttons, one selected.
fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, String)]) {
    ui.spacing_mut().item_spacing.x = 2.0;
    // Right-to-left layout: add in reverse so they read left to right.
    for (v, label) in options.iter().rev() {
        if ui.add(egui::Button::selectable(*value == *v, label.as_str()).min_size(egui::vec2(44.0, 22.0))).clicked() {
            *value = *v;
        }
    }
}
