//! User settings, stored as TOML in the platform config directory.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::geometry::Region;
use rustshot_i18n::tr;

/// Where a finished capture goes. Mirrors Greenshot's "destinations".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Destination {
    /// Show a small menu after every capture and let the user pick.
    Picker,
    Editor,
    Clipboard,
    /// Save straight to the output folder using the filename pattern.
    File,
    /// Ask for a filename with a Save dialog.
    FileAs,
}

impl Destination {
    pub const ALL: [Destination; 5] = [
        Destination::Picker,
        Destination::Editor,
        Destination::Clipboard,
        Destination::File,
        Destination::FileAs,
    ];

    pub fn label(self) -> String {
        match self {
            Destination::Picker => tr!("dest-picker"),
            Destination::Editor => tr!("dest-editor"),
            Destination::Clipboard => tr!("dest-clipboard"),
            Destination::File => tr!("dest-file"),
            Destination::FileAs => tr!("dest-file-as"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Png,
    Jpg,
    Bmp,
    Gif,
}

impl OutputFormat {
    pub const ALL: [OutputFormat; 4] =
        [OutputFormat::Png, OutputFormat::Jpg, OutputFormat::Bmp, OutputFormat::Gif];

    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Jpg => "jpg",
            OutputFormat::Bmp => "bmp",
            OutputFormat::Gif => "gif",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "png" => Some(OutputFormat::Png),
            "jpg" | "jpeg" => Some(OutputFormat::Jpg),
            "bmp" => Some(OutputFormat::Bmp),
            "gif" => Some(OutputFormat::Gif),
            _ => None,
        }
    }
}

/// What a left click on the tray / menu-bar icon does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayClick {
    /// Open the menu (the platform default).
    ShowMenu,
    CaptureRegion,
    CaptureWindow,
    CaptureFullscreen,
    CaptureLastRegion,
}

impl TrayClick {
    pub const ALL: [TrayClick; 5] = [
        TrayClick::ShowMenu,
        TrayClick::CaptureRegion,
        TrayClick::CaptureWindow,
        TrayClick::CaptureFullscreen,
        TrayClick::CaptureLastRegion,
    ];

    pub fn label(self) -> String {
        match self {
            TrayClick::ShowMenu => tr!("tray-click-menu"),
            TrayClick::CaptureRegion => tr!("capture-region"),
            TrayClick::CaptureWindow => tr!("capture-window"),
            TrayClick::CaptureFullscreen => tr!("capture-fullscreen"),
            TrayClick::CaptureLastRegion => tr!("capture-last-region"),
        }
    }
}

/// Global hotkeys in `global-hotkey` syntax, e.g. `"alt+shift+KeyR"`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkeys {
    pub region: String,
    pub window: String,
    pub fullscreen: String,
    pub last_region: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        if cfg!(target_os = "macos") {
            // macOS has no Print Screen key and reserves Cmd+Shift+3/4/5.
            Self {
                region: "alt+shift+KeyR".into(),
                window: "alt+shift+KeyW".into(),
                fullscreen: "alt+shift+KeyF".into(),
                last_region: "alt+shift+KeyL".into(),
            }
        } else {
            // Greenshot's defaults.
            Self {
                region: "PrintScreen".into(),
                window: "alt+PrintScreen".into(),
                fullscreen: "control+PrintScreen".into(),
                last_region: "shift+PrintScreen".into(),
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub hotkeys: Hotkeys,
    pub destinations: Vec<Destination>,
    pub output_dir: PathBuf,
    /// Greenshot-compatible pattern, see [`crate::filename`].
    pub filename_pattern: String,
    pub format: OutputFormat,
    pub jpeg_quality: u8,
    /// Show the magnifier while selecting a region.
    pub show_magnifier: bool,
    /// Copy the saved file's path to the clipboard after "Save directly".
    pub copy_path_after_save: bool,
    /// Draw the mouse pointer into captures.
    pub capture_pointer: bool,
    /// Seconds to wait between a capture request and grabbing the screen.
    pub capture_delay_secs: u32,
    /// Show a system notification after saving or copying.
    pub notifications: bool,
    /// Left click on the tray icon. (Right click always opens the menu.)
    pub tray_click: TrayClick,
    /// Interface language code (`en`, `fr`, …); `None` follows the system.
    pub language: Option<String>,
    /// Region used by "Capture last region".
    pub last_region: Option<Region>,
}

impl Default for Config {
    fn default() -> Self {
        let output_dir = directories::UserDirs::new()
            .and_then(|u| u.desktop_dir().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            hotkeys: Hotkeys::default(),
            destinations: vec![Destination::Picker],
            output_dir,
            filename_pattern: crate::filename::DEFAULT_PATTERN.into(),
            format: OutputFormat::Png,
            jpeg_quality: 90,
            show_magnifier: true,
            copy_path_after_save: false,
            capture_pointer: false,
            capture_delay_secs: 0,
            notifications: true,
            tray_click: TrayClick::ShowMenu,
            language: None,
            last_region: None,
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("org", "Rustshot", "Rustshot")
            .map(|d| d.config_dir().join("rustshot.toml"))
    }

    /// Load settings, falling back to defaults if the file is missing or invalid.
    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                log::warn!("ignoring invalid config {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path().context("no config directory on this platform")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, toml::to_string_pretty(self)?)
            .with_context(|| format!("writing {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_toml() {
        let c = Config {
            last_region: Some(Region::new(1.0, 2.0, 3.0, 4.0)),
            destinations: vec![Destination::Clipboard, Destination::File],
            ..Config::default()
        };
        let text = toml::to_string_pretty(&c).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), c);
    }

    #[test]
    fn partial_file_uses_defaults() {
        let c: Config = toml::from_str("format = \"jpg\"").unwrap();
        assert_eq!(c.format, OutputFormat::Jpg);
        assert_eq!(c.jpeg_quality, 90);
    }
}
