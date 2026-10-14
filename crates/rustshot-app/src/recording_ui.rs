//! While recording: the floating bar (timer, size so far, sound sources that
//! mute on a click, Stop and Discard) and, for a region or window, a dimmed
//! frame around the recorded area that keeps attention on it.

use std::time::Duration;

use egui::{Color32, RichText};
use egui_phosphor::regular as icon;
use rustshot_core::disk;
use rustshot_core::recording::audio::SourceControl;
use rustshot_core::recording::Recording;
use rustshot_core::Region;
use rustshot_i18n::tr;

use crate::platform;

pub struct ActiveRecording {
    pub recording: Recording,
    /// The recorded area and its display (global logical coordinates), for
    /// placing the bar.
    pub region: Region,
    pub monitor: Option<Region>,
    shown: bool,
    frame_shown: bool,
}

pub enum BarAction {
    Stop,
    Discard,
}

const BAR_SIZE: egui::Vec2 = egui::vec2(320.0, 44.0);
/// Extra width when the recording has sound, for the source icons.
const SOUND_WIDTH: f32 = 60.0;
/// Fixed slots, so nothing on the bar moves as the numbers change.
const TIME_WIDTH: f32 = 62.0;
const SIZE_WIDTH: f32 = 70.0;
const ICON_WIDTH: f32 = 26.0;

const RECORDING_RED: Color32 = Color32::from_rgb(230, 40, 40);
/// How dark the area outside the recording is.
const DIM: Color32 = Color32::from_black_alpha(110);

impl ActiveRecording {
    pub fn new(recording: Recording, region: Region, monitor: Option<Region>) -> Self {
        Self { recording, region, monitor, shown: false, frame_shown: false }
    }

    /// A region or window recording (not a whole display) gets the frame.
    fn wants_frame(&self) -> bool {
        self.monitor.is_some_and(|m| {
            let r = self.region;
            (r.x - m.x).abs() > 1.0 || (r.y - m.y).abs() > 1.0 || (r.w - m.w).abs() > 1.0 || (r.h - m.h).abs() > 1.0
        })
    }

    /// Dim the recorded display outside the recorded area, with a thin red
    /// line around it. The window ignores the mouse and is kept out of
    /// recordings and Mission Control.
    pub fn show_frame(&mut self, ctx: &egui::Context) {
        let (Some(m), true) = (self.monitor, self.wants_frame()) else { return };
        let id = egui::ViewportId::from_hash_of("rustshot-recording-frame");
        let title = tr!("recording-frame-title");
        let builder = egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_position([m.x as f32, m.y as f32])
            .with_inner_size([m.w as f32, m.h as f32])
            .with_decorations(false)
            .with_resizable(false)
            .with_has_shadow(false)
            .with_transparent(true)
            .with_mouse_passthrough(true)
            .with_taskbar(false)
            .with_active(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop);
        let region = self.region;
        ctx.show_viewport_immediate(id, builder, |ui, _| {
            // Work from where the window really is: the OS may keep it clear
            // of the menu bar, so it can sit lower than asked.
            let origin = ui.ctx().input(|i| i.viewport().inner_rect).map(|r| r.min).unwrap_or(egui::pos2(m.x as f32, m.y as f32));
            let screen = ui.ctx().content_rect();
            let area = egui::Rect::from_min_size(
                egui::pos2(region.x as f32 - origin.x, region.y as f32 - origin.y),
                egui::vec2(region.w as f32, region.h as f32),
            );
            let painter = ui.painter();
            for r in outside(screen, area) {
                painter.rect_filled(r, 0.0, DIM);
            }
            painter.rect_stroke(area, 0.0, egui::Stroke::new(2.0, RECORDING_RED), egui::StrokeKind::Outside);
        });
        if !self.frame_shown {
            platform::exclude_from_capture(&title);
            platform::make_transient(&title);
            self.frame_shown = true;
        }
    }

    /// Where the bar goes: just above the recorded area if there's room on
    /// its display, else just below, else inside the top edge.
    fn bar_size(&self) -> egui::Vec2 {
        if self.recording.sound().is_some() { BAR_SIZE + egui::vec2(SOUND_WIDTH, 0.0) } else { BAR_SIZE }
    }

    fn bar_position(&self) -> egui::Pos2 {
        let size = self.bar_size();
        let r = self.region;
        let m = self.monitor.unwrap_or(r);
        let x = (r.x + r.w / 2.0 - size.x as f64 / 2.0).clamp(m.x + 8.0, (m.right() - size.x as f64 - 8.0).max(m.x));
        let gap = 10.0;
        let above = r.y - size.y as f64 - gap;
        let below = r.bottom() + gap;
        let y = if above >= m.y + 30.0 {
            above
        } else if below + size.y as f64 <= m.bottom() - 8.0 {
            below
        } else {
            r.y + gap
        };
        egui::pos2(x as f32, y as f32)
    }

    pub fn show_bar(&mut self, ctx: &egui::Context) -> Option<BarAction> {
        let id = egui::ViewportId::from_hash_of("rustshot-recording-bar");
        let title = tr!("recording-bar-title");
        let builder = egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_position(self.bar_position())
            .with_inner_size(self.bar_size())
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_active(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop);
        let elapsed = self.recording.elapsed();
        let size = disk::format_bytes(self.recording.bytes_so_far());
        let sound = self.recording.sound();
        let action = ctx.show_viewport_immediate(id, builder, |ui, _| {
            let mut action = None;
            let frame = egui::Frame::new().fill(ui.visuals().window_fill).inner_margin(egui::Margin::symmetric(12, 8)).corner_radius(10.0);
            egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    // Blinking red dot.
                    let on = (elapsed.as_millis() / 600).is_multiple_of(2);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 6.0, if on { RECORDING_RED } else { Color32::from_rgb(150, 40, 40) });
                    slot(ui, TIME_WIDTH, egui::Align::Min, |ui| {
                        ui.label(RichText::new(format_elapsed(elapsed)).monospace().strong());
                    });
                    slot(ui, SIZE_WIDTH, egui::Align::Max, |ui| {
                        ui.label(RichText::new(&size).monospace().weak());
                    });
                    if let Some(sound) = sound {
                        ui.add_space(4.0);
                        let mic = (icon::MICROPHONE, icon::MICROPHONE_SLASH, tr!("recording-mute-microphone"), tr!("recording-unmute-microphone"));
                        source_button(ui, &sound.microphone, on, mic);
                        let system = (icon::SPEAKER_HIGH, icon::SPEAKER_SLASH, tr!("recording-mute-system"), tr!("recording-unmute-system"));
                        source_button(ui, &sound.system, on, system);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(format!("{} {}", icon::STOP, tr!("recording-stop"))).clicked() {
                            action = Some(BarAction::Stop);
                        }
                        if ui.button(icon::TRASH).on_hover_text(tr!("recording-discard-tip")).clicked() {
                            action = Some(BarAction::Discard);
                        }
                    });
                });
            });
            action
        });
        if !self.shown {
            // Keep the bar out of the recording itself and out of Mission Control.
            platform::exclude_from_capture(&title);
            platform::make_transient(&title);
            self.shown = true;
        }
        // Keep the timer ticking.
        ctx.request_repaint_after(Duration::from_millis(250));
        action
    }
}

/// A sound source's icon, shown once it is recording: it blinks with the
/// red dot while live, and a click mutes it (crossed out and steady) or
/// unmutes it.
fn source_button(ui: &mut egui::Ui, control: &SourceControl, blink_on: bool, (live, muted, mute_tip, unmute_tip): (&str, &str, String, String)) {
    // The slot is kept even before the source starts, so nothing shifts.
    slot(ui, ICON_WIDTH, egui::Align::Center, |ui| source_icon(ui, control, blink_on, (live, muted, mute_tip, unmute_tip)));
}

fn source_icon(ui: &mut egui::Ui, control: &SourceControl, blink_on: bool, (live, muted, mute_tip, unmute_tip): (&str, &str, String, String)) {
    if !control.is_live() {
        return;
    }
    let is_muted = control.is_muted();
    let (glyph, color, tip) = if is_muted {
        (muted, ui.visuals().weak_text_color(), unmute_tip)
    } else {
        let color = if blink_on { ui.visuals().strong_text_color() } else { ui.visuals().weak_text_color() };
        (live, color, mute_tip)
    };
    let button = egui::Button::new(RichText::new(glyph).size(16.0).color(color)).frame(false);
    if ui.add(button).on_hover_text(tip).clicked() {
        control.set_muted(!is_muted);
    }
}

/// A fixed-width space on the bar, its content aligned within it.
fn slot(ui: &mut egui::Ui, width: f32, align: egui::Align, content: impl FnOnce(&mut egui::Ui)) {
    let layout = match align {
        egui::Align::Min => egui::Layout::left_to_right(egui::Align::Center),
        egui::Align::Max => egui::Layout::right_to_left(egui::Align::Center),
        egui::Align::Center => egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
    };
    ui.allocate_ui_with_layout(egui::vec2(width, ui.available_height()), layout, |ui| {
        ui.set_min_width(width);
        ui.set_max_width(width);
        content(ui)
    });
}

/// The parts of `screen` outside `area`, as up to four rectangles.
fn outside(screen: egui::Rect, area: egui::Rect) -> Vec<egui::Rect> {
    let a = area.intersect(screen);
    if !a.is_positive() {
        return vec![screen];
    }
    [
        egui::Rect::from_min_max(screen.min, egui::pos2(screen.max.x, a.min.y)),
        egui::Rect::from_min_max(egui::pos2(screen.min.x, a.max.y), screen.max),
        egui::Rect::from_min_max(egui::pos2(screen.min.x, a.min.y), egui::pos2(a.min.x, a.max.y)),
        egui::Rect::from_min_max(egui::pos2(a.max.x, a.min.y), egui::pos2(screen.max.x, a.max.y)),
    ]
    .into_iter()
    .filter(|r| r.is_positive())
    .collect()
}

/// `m:ss`, or `h:mm:ss` past an hour.
pub fn format_elapsed(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_covers_everything_but_the_area() {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 80.0));
        let area = egui::Rect::from_min_max(egui::pos2(20.0, 10.0), egui::pos2(60.0, 50.0));
        let parts = outside(screen, area);
        let total: f32 = parts.iter().map(|r| r.area()).sum();
        assert_eq!(total, screen.area() - area.area());
        assert!(parts.iter().all(|r| !r.intersects(area.shrink(0.5))));
        // An area touching the edge leaves no empty strips.
        assert_eq!(outside(screen, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(50.0, 80.0))).len(), 1);
    }

    #[test]
    fn elapsed_format() {
        assert_eq!(format_elapsed(Duration::from_secs(7)), "0:07");
        assert_eq!(format_elapsed(Duration::from_secs(125)), "2:05");
        assert_eq!(format_elapsed(Duration::from_secs(3725)), "1:02:05");
    }
}
