//! Global hotkey registration.

use std::str::FromStr;

use global_hotkey::{hotkey::HotKey, GlobalHotKeyManager};
use rustshot_core::config::Hotkeys;

use crate::app::CaptureMode;

pub struct HotkeyRegistry {
    manager: Option<GlobalHotKeyManager>,
    registered: Vec<(HotKey, CaptureMode)>,
}

impl HotkeyRegistry {
    pub fn new() -> Self {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| log::error!("global hotkeys unavailable: {e}"))
            .ok();
        Self { manager, registered: Vec::new() }
    }

    /// Replace all registrations. Returns human-readable errors, if any.
    pub fn register(&mut self, keys: &Hotkeys) -> Vec<String> {
        let Some(manager) = &self.manager else {
            return vec!["Global hotkeys are not supported on this system.".into()];
        };
        for (hk, _) in self.registered.drain(..) {
            let _ = manager.unregister(hk);
        }
        let mut errors = Vec::new();
        for (text, mode) in [
            (&keys.region, CaptureMode::Region),
            (&keys.window, CaptureMode::Window),
            (&keys.fullscreen, CaptureMode::Fullscreen),
            (&keys.last_region, CaptureMode::LastRegion),
        ] {
            if text.trim().is_empty() {
                continue;
            }
            match HotKey::from_str(text) {
                Ok(hk) => match manager.register(hk) {
                    Ok(()) => self.registered.push((hk, mode)),
                    Err(e) => errors.push(format!("{} ({text}): {e}", mode.label())),
                },
                Err(e) => errors.push(format!("{} ({text}): {e}", mode.label())),
            }
        }
        errors
    }

    pub fn mode_for(&self, id: u32) -> Option<CaptureMode> {
        self.registered.iter().find(|(hk, _)| hk.id() == id).map(|(_, m)| *m)
    }
}

/// Validate a hotkey string without registering it.
pub fn validate(text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    HotKey::from_str(text).map(|_| ()).map_err(|e| e.to_string())
}
