//! While a scrolling capture runs: the dimmed frame around the area and a
//! small bar with the height so far and Stop.

use std::time::Duration;

use chrono::{DateTime, Local};
use egui::RichText;
use egui_phosphor::regular as icon;
use rustshot_core::scroll::ScrollCapture;
use rustshot_core::Region;
use rustshot_i18n::tr;

use crate::platform;
use crate::recording_ui::{place_bar, show_area_frame};

const BAR_SIZE: egui::Vec2 = egui::vec2(340.0, 44.0);

pub struct ActiveScroll {
    pub capture: ScrollCapture,
    region: Region,
    monitor: Option<Region>,
    pub title: String,
    pub taken_at: DateTime<Local>,
    frame_shown: bool,
    bar_shown: bool,
}

impl ActiveScroll {
    pub fn new(capture: ScrollCapture, region: Region, monitor: Option<Region>, title: String, taken_at: DateTime<Local>) -> Self {
        Self { capture, region, monitor, title, taken_at, frame_shown: false, bar_shown: false }
    }

    /// Show the frame and bar. Clicking Stop is itself a mouse press, which
    /// the capture notices; the button also stops it directly.
    pub fn show(&mut self, ctx: &egui::Context) {
        if let Some(m) = self.monitor {
            show_area_frame(ctx, "rustshot-scroll-frame", self.region, m, &mut self.frame_shown);
        }
        let id = egui::ViewportId::from_hash_of("rustshot-scroll-bar");
        let title = tr!("scroll-bar-title");
        let builder = egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_position(place_bar(self.region, self.monitor, BAR_SIZE))
            .with_inner_size(BAR_SIZE)
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_active(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop);
        let height = self.capture.height();
        let stop = ctx.show_viewport_immediate(id, builder, |ui, _| {
            let mut stop = false;
            let frame = egui::Frame::new().fill(ui.visuals().window_fill).inner_margin(egui::Margin::symmetric(12, 8)).corner_radius(10.0);
            egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spinner();
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tr!("scroll-progress", pixels = height)).strong());
                        ui.label(RichText::new(tr!("scroll-hint")).small().weak());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(format!("{} {}", icon::STOP, tr!("scroll-stop"))).clicked() {
                            stop = true;
                        }
                    });
                });
            });
            stop
        });
        if stop {
            self.capture.stop();
        }
        if !self.bar_shown {
            platform::exclude_from_capture(&title);
            platform::make_transient(&title);
            self.bar_shown = true;
        }
        ctx.request_repaint_after(Duration::from_millis(150));
    }
}
