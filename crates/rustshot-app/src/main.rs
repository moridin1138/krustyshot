// No console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! Rustshot: a cross-platform screenshot tool modelled on Greenshot.

mod app;
mod fonts;
mod hotkeys;
mod overlay;
mod platform;
mod recording_ui;
mod scroll_ui;
mod send;
mod settings_ui;
mod touchbar_guard;
mod tray;

/// Log lines go to the terminal and to the log file.
struct Tee(Option<std::fs::File>);

impl std::io::Write for Tee {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = std::io::stderr().write_all(buf);
        if let Some(f) = &mut self.0 {
            let _ = f.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(f) = &mut self.0 {
            f.flush()?;
        }
        std::io::stderr().flush()
    }
}

/// Start logging. The previous run's log is kept as `rustshot.old.log`.
fn init_logging() {
    let file = rustshot_core::config::log_path().and_then(|path| {
        std::fs::create_dir_all(path.parent()?).ok()?;
        let _ = std::fs::rename(&path, path.with_file_name("rustshot.old.log"));
        std::fs::File::create(&path).ok()
    });
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(Box::new(Tee(file))))
        .init();
}

fn main() -> eframe::Result {
    init_logging();
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
