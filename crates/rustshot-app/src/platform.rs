//! Small platform-specific tweaks.

/// Run as a menu-bar accessory on macOS (no Dock icon, no app menu).
pub fn configure_event_loop<T>(builder: &mut winit::event_loop::EventLoopBuilder<T>) {
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        builder.with_activation_policy(ActivationPolicy::Accessory);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = builder;
}

pub const OVERLAY_TITLE: &str = "Rustshot Overlay";

/// Lift the capture overlay above the menu bar and Dock, and make it appear
/// on the current Space even when another app is full screen.
pub fn raise_overlays() {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSApplication, NSWindowCollectionBehavior};
        use objc2_foundation::MainThreadMarker;

        let Some(mtm) = MainThreadMarker::new() else { return };
        let app = NSApplication::sharedApplication(mtm);
        for window in app.windows().iter() {
            if window.title().to_string() == OVERLAY_TITLE {
                // NSScreenSaverWindowLevel
                window.setLevel(1000);
                window.setCollectionBehavior(
                    NSWindowCollectionBehavior::CanJoinAllSpaces
                        | NSWindowCollectionBehavior::FullScreenAuxiliary
                        | NSWindowCollectionBehavior::Stationary,
                );
            }
        }
    }
}

/// Bring the app to the foreground so its windows get keyboard focus.
pub fn activate_app() {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSApplication;
        use objc2_foundation::MainThreadMarker;
        if let Some(mtm) = MainThreadMarker::new() {
            #[allow(deprecated)]
            NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
        }
    }
}

/// Open a folder in Finder / Explorer / the default file manager.
pub fn open_folder(path: &std::path::Path) {
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(cmd).arg(path).spawn() {
        log::warn!("could not open {}: {e}", path.display());
    }
}
