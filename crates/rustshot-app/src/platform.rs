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

/// Work around an AppKit crash on macOS.
///
/// AppKit's Touch Bar support watches every window, even on Macs without a
/// Touch Bar. When one of Rustshot's windows closes, it sometimes tries to
/// stop watching an observer it never added, which raises an exception
/// (`_NSTouchBarFinderObservation invalidate`) and kills the app. Rustshot
/// has no Touch Bar UI, so switch the feature off before AppKit starts.
pub fn disable_touch_bar() {
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::{NSString, NSUserDefaults};
        let defaults = NSUserDefaults::standardUserDefaults();
        defaults.setBool_forKey(false, &NSString::from_str("NSFunctionBarAPIEnabled"));
    }
}

/// Title of eframe's root window, which Rustshot never shows.
pub const BACKGROUND_TITLE: &str = "Rustshot Background";

/// Make the always-present root window invisible to the user: fully
/// transparent, and excluded from Mission Control, App Exposé and window
/// cycling. (It can't simply be hidden, because eframe stops running the UI
/// of a hidden root window.)
pub fn hide_background_window() {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSApplication, NSWindowCollectionBehavior};
        use objc2_foundation::MainThreadMarker;

        let Some(mtm) = MainThreadMarker::new() else { return };
        for window in NSApplication::sharedApplication(mtm).windows().iter() {
            if window.title().to_string() == BACKGROUND_TITLE {
                window.setAlphaValue(0.0);
                window.setIgnoresMouseEvents(true);
                window.setCollectionBehavior(
                    NSWindowCollectionBehavior::Transient
                        | NSWindowCollectionBehavior::IgnoresCycle
                        | NSWindowCollectionBehavior::CanJoinAllSpaces,
                );
            }
        }
    }
}

pub const OVERLAY_TITLE: &str = "Rustshot Overlay";

/// Window title for the overlay on monitor `i`, so each window can be found again.
pub fn overlay_title(i: usize) -> String {
    format!("{OVERLAY_TITLE} {i}")
}

/// Lift each capture overlay above the menu bar and Dock, place it exactly
/// over its monitor (macOS otherwise keeps windows clear of the menu bar and
/// notch), and make it appear on the current Space even over full-screen apps.
///
/// `monitors[i]` is the bounds of overlay `i` in global top-left coordinates.
pub fn raise_overlays(monitors: &[rustshot_core::Region]) {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSApplication, NSScreen, NSWindowCollectionBehavior};
        use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};

        let Some(mtm) = MainThreadMarker::new() else { return };
        // Cocoa puts the origin at the bottom-left of the primary screen.
        let Some(primary) = NSScreen::screens(mtm).firstObject() else { return };
        let primary_h = primary.frame().size.height;
        let app = NSApplication::sharedApplication(mtm);
        for window in app.windows().iter() {
            let title = window.title().to_string();
            let Some(i) = title.strip_prefix(OVERLAY_TITLE).and_then(|n| n.trim().parse::<usize>().ok()) else {
                continue;
            };
            let Some(m) = monitors.get(i) else { continue };
            // NSScreenSaverWindowLevel
            window.setLevel(1000);
            window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary
                    | NSWindowCollectionBehavior::Stationary,
            );
            let target = NSRect::new(NSPoint::new(m.x, primary_h - m.y - m.h), NSSize::new(m.w, m.h));
            let current = window.frame();
            if current.origin != target.origin || current.size != target.size {
                log::debug!("moving overlay {i} from {current:?} to {target:?}");
                window.setFrame_display(target, true);
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = monitors;
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
