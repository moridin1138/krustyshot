// No console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! Rustshot: a cross-platform screenshot tool modelled on Greenshot.

mod app;
mod fonts;
mod hotkeys;
mod overlay;
mod platform;
mod recording_ui;
mod send;
mod settings_ui;
mod touchbar_guard;
mod tray;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    touchbar_guard::install();

    // The root window is never meant to be seen: the app lives in the tray
    // and opens overlay/editor/settings windows as child viewports.
    let viewport = egui::ViewportBuilder::default()
        .with_title(platform::BACKGROUND_TITLE)
        .with_inner_size([1.0, 1.0])
        // Keep it on-screen: an off-screen window makes macOS Mission Control
        // zoom all the way out to fit it. It is transparent, click-through,
        // and hidden from Mission Control by `platform::hide_background_window`.
        .with_position([0.0, 0.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_taskbar(false)
        .with_active(false)
        .with_mouse_passthrough(true);

    let options = eframe::NativeOptions {
        viewport,
        event_loop_builder: Some(Box::new(platform::configure_event_loop)),
        ..Default::default()
    };

    eframe::run_native("Rustshot", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
