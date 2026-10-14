//! Screen-recording permission checks (macOS only; other platforms always allow).

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

/// True if the app may capture other windows' contents.
pub fn has_screen_capture_access() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        CGPreflightScreenCaptureAccess()
    }
    #[cfg(not(target_os = "macos"))]
    true
}

/// Ask the OS for screen-recording access. On macOS this shows the system
/// prompt once; afterwards the user must enable it in System Settings.
pub fn request_screen_capture_access() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        CGRequestScreenCaptureAccess()
    }
    #[cfg(not(target_os = "macos"))]
    true
}

/// Bundle identifier of the macOS app (see `crates/rustshot-app/bundle/Info.plist`).
pub const MACOS_BUNDLE_ID: &str = "org.rustshot.Rustshot";

/// Forget any Screen Recording decision for Rustshot and ask again.
///
/// macOS ties the permission to the app's code signature. Builds that are not
/// signed with a stable certificate get a new signature every release, so an
/// old "on" entry in System Settings can silently stop applying. Resetting
/// removes it so the user can grant access to the current build.
pub fn reset_screen_capture_access() {
    #[cfg(target_os = "macos")]
    {
        match std::process::Command::new("tccutil").args(["reset", "ScreenCapture", MACOS_BUNDLE_ID]).status() {
            Ok(s) if s.success() => log::info!("reset Screen Recording permission for {MACOS_BUNDLE_ID}"),
            Ok(s) => log::warn!("tccutil exited with {s}"),
            Err(e) => log::warn!("could not run tccutil: {e}"),
        }
        request_screen_capture_access();
    }
}

/// Open System Settings at the microphone permissions (macOS).
pub fn open_microphone_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
            .spawn();
    }
}

/// Open the relevant System Settings pane, where supported.
pub fn open_privacy_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn();
    }
}
