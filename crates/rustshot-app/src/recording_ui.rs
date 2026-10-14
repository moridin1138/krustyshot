//! The floating recording bar: timer, size so far, Stop and Discard.

use std::time::Duration;

use egui::{Color32, RichText};
use egui_phosphor::regular as icon;
use rustshot_core::disk;
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
}

pub enum BarAction {
    Stop,
    Discard,
}

const BAR_SIZE: egui::Vec2 = egui::vec2(300.0, 44.0);

impl ActiveRecording {
    pub fn new(recording: Recording, region: Region, monitor: Option<Region>) -> Self {
        Self { recording, region, monitor, shown: false }
    }

    /// Where the bar goes: just above the recorded area if there's room on
    /// its display, else just below, else inside the top edge.
    fn bar_position(&self) -> egui::Pos2 {
        let r = self.region;
        let m = self.monitor.unwrap_or(r);
        let x = (r.x + r.w / 2.0 - BAR_SIZE.x as f64 / 2.0).clamp(m.x + 8.0, (m.right() - BAR_SIZE.x as f64 - 8.0).max(m.x));
        let gap = 10.0;
        let above = r.y - BAR_SIZE.y as f64 - gap;
        let below = r.bottom() + gap;
        let y = if above >= m.y + 30.0 {
            above
        } else if below + BAR_SIZE.y as f64 <= m.bottom() - 8.0 {
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
            .with_inner_size(BAR_SIZE)
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_active(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop);
        let elapsed = self.recording.elapsed();
        let size = disk::format_bytes(self.recording.bytes_so_far());
        let action = ctx.show_viewport_immediate(id, builder, |ui, _| {
            let mut action = None;
            let frame = egui::Frame::new().fill(ui.visuals().window_fill).inner_margin(egui::Margin::symmetric(12, 8)).corner_radius(10.0);
            egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    // Blinking red dot.
                    let on = (elapsed.as_millis() / 600).is_multiple_of(2);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 6.0, if on { Color32::from_rgb(230, 40, 40) } else { Color32::from_rgb(150, 40, 40) });
                    ui.label(RichText::new(format_elapsed(elapsed)).monospace().strong());
                    ui.weak(&size);
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
    fn elapsed_format() {
        assert_eq!(format_elapsed(Duration::from_secs(7)), "0:07");
        assert_eq!(format_elapsed(Duration::from_secs(125)), "2:05");
        assert_eq!(format_elapsed(Duration::from_secs(3725)), "1:02:05");
    }
}
