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
            (&keys.record, CaptureMode::RecordRegion),
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

impl HotkeyRegistry {
    /// Release every hotkey (while the user is recording a new one, so the
    /// old combination doesn't fire a capture). `register` restores them.
    pub fn suspend(&mut self) {
        if let Some(manager) = &self.manager {
            for (hk, _) in self.registered.drain(..) {
                let _ = manager.unregister(hk);
            }
        }
    }
}

/// Turn a key press in the shortcut recorder into `global-hotkey` syntax.
/// Returns `None` for keys that can't be global shortcuts on their own.
pub fn from_key_press(key: egui::Key, mods: egui::Modifiers) -> Option<String> {
    use egui::Key;
    let name = key.name();
    let code = match key {
        k if (Key::A as u32..=Key::Z as u32).contains(&(k as u32)) => format!("Key{name}"),
        k if (Key::Num0 as u32..=Key::Num9 as u32).contains(&(k as u32)) => format!("Digit{}", &name[name.len() - 1..]),
        k if (Key::F1 as u32..=Key::F35 as u32).contains(&(k as u32)) => name.to_string(),
        Key::Space => "Space".into(),
        Key::Enter => "Enter".into(),
        Key::Tab => "Tab".into(),
        Key::ArrowUp => "ArrowUp".into(),
        Key::ArrowDown => "ArrowDown".into(),
        Key::ArrowLeft => "ArrowLeft".into(),
        Key::ArrowRight => "ArrowRight".into(),
        Key::Home => "Home".into(),
        Key::End => "End".into(),
        Key::PageUp => "PageUp".into(),
        Key::PageDown => "PageDown".into(),
        Key::Insert => "Insert".into(),
        Key::Minus => "Minus".into(),
        Key::Equals => "Equal".into(),
        Key::Comma => "Comma".into(),
        Key::Period => "Period".into(),
        Key::Slash => "Slash".into(),
        Key::Backslash => "Backslash".into(),
        Key::Semicolon => "Semicolon".into(),
        Key::Quote => "Quote".into(),
        Key::OpenBracket => "BracketLeft".into(),
        Key::CloseBracket => "BracketRight".into(),
        Key::Backtick => "Backquote".into(),
        _ => return None,
    };
    let is_function_key = code.starts_with('F') && code.len() <= 3;
    let mut parts = Vec::new();
    if mods.ctrl {
        parts.push("control");
    }
    if mods.alt {
        parts.push("alt");
    }
    if mods.shift {
        parts.push("shift");
    }
    if mods.mac_cmd || (!cfg!(target_os = "macos") && mods.command && !mods.ctrl) {
        parts.push("super");
    }
    // A plain letter would fire every time you type it.
    if parts.is_empty() && !is_function_key {
        return None;
    }
    parts.push(&code);
    let text = parts.join("+");
    validate(&text).ok().map(|_| text)
}

/// Split a shortcut into keycap labels. Modifiers use Phosphor icons on
/// macOS (egui's fonts lack ⌘⌥⇧ glyphs) and words elsewhere.
pub fn keycaps(text: &str) -> Vec<String> {
    use egui_phosphor::regular as icon;
    let mac = cfg!(target_os = "macos");
    text.split('+')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| match t.to_ascii_lowercase().as_str() {
            "alt" | "option" => if mac { icon::OPTION.to_string() } else { "Alt".into() },
            "shift" => if mac { icon::ARROW_FAT_UP.to_string() } else { "Shift".into() },
            "control" | "ctrl" => if mac { icon::CONTROL.to_string() } else { "Ctrl".into() },
            "super" | "cmd" | "command" => if mac { icon::COMMAND.to_string() } else { "Win".into() },
            _ => {
                let key = t.strip_prefix("Key").or_else(|| t.strip_prefix("Digit")).unwrap_or(t);
                match key {
                    "PrintScreen" => "Print Screen".into(),
                    "BracketLeft" => "[".into(),
                    "BracketRight" => "]".into(),
                    "Backquote" => "`".into(),
                    "Equal" => "=".into(),
                    "Minus" => "-".into(),
                    other => other.to_string(),
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_combinations() {
        let m = egui::Modifiers { alt: true, shift: true, ..Default::default() };
        assert_eq!(from_key_press(egui::Key::R, m).as_deref(), Some("alt+shift+KeyR"));
        let ctrl = egui::Modifiers { ctrl: true, ..Default::default() };
        assert_eq!(from_key_press(egui::Key::Num4, ctrl).as_deref(), Some("control+Digit4"));
        assert_eq!(from_key_press(egui::Key::F5, egui::Modifiers::NONE).as_deref(), Some("F5"));
    }

    #[test]
    fn rejects_bare_letters() {
        assert_eq!(from_key_press(egui::Key::R, egui::Modifiers::NONE), None);
    }

    #[test]
    fn keycaps_readable() {
        let caps = keycaps("control+PrintScreen");
        assert_eq!(caps.last().map(String::as_str), Some("Print Screen"));
        assert_eq!(keycaps("alt+shift+KeyR").last().map(String::as_str), Some("R"));
    }
}
