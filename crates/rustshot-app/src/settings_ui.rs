//! Preferences window.

use rustshot_core::config::Hotkeys;
use rustshot_core::filename::{self, PatternContext};
use rustshot_core::{Config, Destination, OutputFormat, TrayClick};

use crate::hotkeys;

pub enum SettingsAction {
    Save(Box<Config>),
    Cancel,
}

pub struct SettingsWindow {
    draft: Config,
    focus: bool,
}

impl SettingsWindow {
    pub fn new(config: &Config) -> Self {
        Self { draft: config.clone(), focus: true }
    }

    pub fn show(&mut self, ctx: &egui::Context, hotkey_errors: &[String]) -> Option<SettingsAction> {
        let id = egui::ViewportId::from_hash_of("rustshot-settings");
        let builder = egui::ViewportBuilder::default()
            .with_title("Rustshot Preferences")
            .with_inner_size([560.0, 720.0])
            .with_min_inner_size([460.0, 420.0]);
        let action = ctx.show_viewport_immediate(id, builder, |ui, _| {
            if ui.input(|i| i.viewport().close_requested()) {
                return Some(SettingsAction::Cancel);
            }
            let mut action = None;
            egui::Panel::bottom("settings_buttons").show(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let valid = self.hotkeys_valid();
                    if ui.add_enabled(valid, egui::Button::new("Save")).clicked() {
                        action = Some(SettingsAction::Save(Box::new(self.draft.clone())));
                    }
                    if ui.button("Cancel").clicked() {
                        action = Some(SettingsAction::Cancel);
                    }
                    if ui.button("Reset to defaults").clicked() {
                        let last = self.draft.last_region;
                        self.draft = Config { last_region: last, ..Config::default() };
                    }
                });
                ui.add_space(6.0);
            });
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.form(ui, hotkey_errors));
            });
            action
        });
        if self.focus {
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
            self.focus = false;
        }
        action
    }

    fn hotkeys_valid(&self) -> bool {
        let h = &self.draft.hotkeys;
        [&h.region, &h.window, &h.fullscreen, &h.last_region]
            .iter()
            .all(|k| hotkeys::validate(k).is_ok())
    }

    fn form(&mut self, ui: &mut egui::Ui, hotkey_errors: &[String]) {
        let d = &mut self.draft;

        ui.heading("Hotkeys");
        ui.label(
            egui::RichText::new("Format: modifiers + key, e.g. alt+shift+KeyR, control+PrintScreen, super+Digit1")
                .small()
                .weak(),
        );
        egui::Grid::new("hotkeys").num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
            let Hotkeys { region, window, fullscreen, last_region } = &mut d.hotkeys;
            for (label, value) in [
                ("Capture region", region),
                ("Capture window", window),
                ("Capture full screen", fullscreen),
                ("Capture last region", last_region),
            ] {
                ui.label(label);
                ui.add(egui::TextEdit::singleline(value).desired_width(200.0));
                match hotkeys::validate(value) {
                    Ok(()) => ui.label(""),
                    Err(e) => ui.colored_label(ui.visuals().error_fg_color, e),
                };
                ui.end_row();
            }
        });
        for e in hotkey_errors {
            ui.colored_label(ui.visuals().warn_fg_color, e);
        }

        ui.add_space(12.0);
        ui.heading("After capture");
        let picker = d.destinations.contains(&Destination::Picker);
        let mut ask = picker;
        ui.checkbox(&mut ask, Destination::Picker.label());
        if ask != picker {
            d.destinations = if ask { vec![Destination::Picker] } else { vec![Destination::Editor] };
        }
        ui.add_enabled_ui(!ask, |ui| {
            for dest in Destination::ALL.into_iter().filter(|d| *d != Destination::Picker) {
                let mut on = d.destinations.contains(&dest);
                if ui.checkbox(&mut on, dest.label()).changed() {
                    if on {
                        d.destinations.push(dest);
                    } else {
                        d.destinations.retain(|x| *x != dest);
                    }
                }
            }
        });

        ui.add_space(12.0);
        ui.heading("Capture");
        ui.checkbox(&mut d.show_magnifier, "Show magnifier while selecting");
        ui.checkbox(&mut d.capture_pointer, "Include the mouse pointer");
        ui.horizontal(|ui| {
            ui.label("Wait before capturing");
            ui.add(egui::DragValue::new(&mut d.capture_delay_secs).range(0..=30).suffix(" s"));
            if d.capture_delay_secs > 0 {
                ui.weak("(time to open a menu or hover something)");
            }
        });

        ui.add_space(12.0);
        ui.heading("Menu-bar icon and notifications");
        ui.horizontal(|ui| {
            ui.label("Clicking the icon");
            egui::ComboBox::from_id_salt("tray_click")
                .selected_text(d.tray_click.label())
                .show_ui(ui, |ui| {
                    for c in TrayClick::ALL {
                        ui.selectable_value(&mut d.tray_click, c, c.label());
                    }
                });
        });
        if d.tray_click != TrayClick::ShowMenu {
            ui.weak("Right-click the icon for the menu.");
        }
        ui.checkbox(&mut d.notifications, "Show a notification after saving or copying");

        ui.add_space(12.0);
        ui.heading("Output");
        egui::Grid::new("output").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Folder");
            ui.horizontal(|ui| {
                let mut text = d.output_dir.display().to_string();
                if ui.add(egui::TextEdit::singleline(&mut text).desired_width(260.0)).changed() {
                    d.output_dir = text.into();
                }
                if ui.button("Browse…").clicked()
                    && let Some(dir) = rfd::FileDialog::new().set_directory(&d.output_dir).pick_folder() {
                        d.output_dir = dir;
                    }
            });
            ui.end_row();

            ui.label("Filename pattern");
            ui.add(egui::TextEdit::singleline(&mut d.filename_pattern).desired_width(320.0));
            ui.end_row();

            ui.label("");
            let preview = filename::expand(
                &d.filename_pattern,
                &PatternContext { time: chrono::Local::now(), title: "Window title" },
            );
            ui.label(egui::RichText::new(format!("{preview}.{}", d.format.extension())).weak());
            ui.end_row();

            ui.label("Format");
            egui::ComboBox::from_id_salt("format")
                .selected_text(d.format.extension().to_uppercase())
                .show_ui(ui, |ui| {
                    for f in OutputFormat::ALL {
                        ui.selectable_value(&mut d.format, f, f.extension().to_uppercase());
                    }
                });
            ui.end_row();

            if d.format == OutputFormat::Jpg {
                ui.label("JPEG quality");
                ui.add(egui::Slider::new(&mut d.jpeg_quality, 1..=100));
                ui.end_row();
            }
        });
        ui.checkbox(&mut d.copy_path_after_save, "Copy file path to clipboard after saving directly");

        ui.add_space(8.0);
        ui.collapsing("Filename placeholders", |ui| {
            ui.label("${YYYY} ${MM} ${DD} ${hh} ${mm} ${ss} ${ms}");
            ui.label("${title}  window title (or \"Screenshot\")");
            ui.label("${user}  ${hostname}");
            ui.label("${capturetime:d\"yyyy-MM-dd HH_mm_ss\"}  Greenshot-style date format");
        });
    }
}
