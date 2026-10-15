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
    /// Open in another app (the default image app, or `Config::external_app`).
    OpenWith,
    Print,
    /// A new email with the image attached.
    Email,
    /// The system Share menu (macOS: AirDrop, Messages, Notes, …).
    Share,
    /// Recognise the text in the image and copy it.
    CopyText,
    /// Send to the custom uploader and copy the link.
    Upload,
}

impl Destination {
    pub const ALL: [Destination; 11] = [
        Destination::Picker,
        Destination::Editor,
        Destination::Clipboard,
        Destination::File,
        Destination::FileAs,
        Destination::OpenWith,
        Destination::Print,
        Destination::Email,
        Destination::Share,
        Destination::CopyText,
        Destination::Upload,
    ];

    /// Whether this platform offers the destination.
    pub fn available(self) -> bool {
        // The Share menu is macOS's.
        self != Destination::Share || cfg!(target_os = "macos")
    }

    pub fn label(self) -> String {
        match self {
            Destination::Picker => tr!("dest-picker"),
            Destination::Editor => tr!("dest-editor"),
            Destination::Clipboard => tr!("dest-clipboard"),
            Destination::File => tr!("dest-file"),
            Destination::FileAs => tr!("dest-file-as"),
            Destination::OpenWith => tr!("dest-open-with"),
            Destination::Print => tr!("dest-print"),
            Destination::Email => tr!("dest-email"),
            Destination::Share => tr!("dest-share"),
            Destination::CopyText => tr!("dest-copy-text"),
            Destination::Upload => tr!("dest-upload"),
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

/// Container for recordings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoFormat {
    /// H.264 video, using the operating system's encoder.
    Mp4,
    /// Animated GIF: works everywhere, best for short clips.
    Gif,
}

impl VideoFormat {
    pub fn extension(self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "mp4",
            VideoFormat::Gif => "gif",
        }
    }
}

/// Encoding quality; trades file size for sharpness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoQuality {
    Low,
    Medium,
    High,
}

impl VideoQuality {
    pub const ALL: [VideoQuality; 3] = [VideoQuality::Low, VideoQuality::Medium, VideoQuality::High];

    /// H.264 bits per pixel per frame. Screen content compresses well, so
    /// these are lower than typical camera-video figures.
    pub fn bits_per_pixel(self) -> f64 {
        match self {
            VideoQuality::Low => 0.04,
            VideoQuality::Medium => 0.08,
            VideoQuality::High => 0.15,
        }
    }
}

/// Recording settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordingSettings {
    pub format: VideoFormat,
    pub quality: VideoQuality,
    /// Record at full Retina resolution instead of half.
    pub full_resolution: bool,
    pub fps: u32,
    /// Draw a ring where the mouse is clicked.
    pub show_clicks: bool,
    /// Record the microphone (MP4 only).
    pub microphone: bool,
    /// Which microphone, by its system ID; `None` for the default one.
    pub microphone_device: Option<String>,
    /// Record what the computer plays (MP4 only; macOS 13 or later).
    pub system_audio: bool,
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            format: if cfg!(target_os = "macos") { VideoFormat::Mp4 } else { VideoFormat::Gif },
            quality: VideoQuality::Medium,
            full_resolution: false,
            fps: 30,
            show_clicks: false,
            microphone: false,
            microphone_device: None,
            system_audio: false,
        }
    }
}

impl RecordingSettings {
    /// GIFs are capped at 15 fps: more mostly just grows the file.
    pub const GIF_MAX_FPS: u32 = 15;
    /// GIF frames are scaled down to at most this width.
    pub const GIF_MAX_WIDTH: u32 = 1280;

    /// The sound track's bit rate (AAC), bits per second.
    pub const AUDIO_BIT_RATE: u64 = 128_000;

    /// Whether recordings get a sound track (MP4 only).
    pub fn has_sound(&self) -> bool {
        self.format == VideoFormat::Mp4 && (self.microphone || self.system_audio)
    }

    pub fn effective_fps(&self) -> u32 {
        match self.format {
            VideoFormat::Mp4 => self.fps.clamp(5, 60),
            VideoFormat::Gif => self.fps.clamp(5, Self::GIF_MAX_FPS),
        }
    }

    /// Output size for a recording area of `w`×`h` pixels on a display with
    /// `scale` pixels per point (2 on Retina). Always even, as H.264 needs.
    pub fn output_size(&self, w: u32, h: u32, scale: f64) -> (u32, u32) {
        let mut f = if self.full_resolution || scale <= 1.0 { 1.0 } else { 1.0 / scale };
        if self.format == VideoFormat::Gif && (w as f64 * f) > Self::GIF_MAX_WIDTH as f64 {
            f = Self::GIF_MAX_WIDTH as f64 / w as f64;
        }
        let even = |v: f64| ((v.round() as u32) & !1).max(2);
        (even(w as f64 * f), even(h as f64 * f))
    }

    /// H.264 target bit rate for an output of `w`×`h`.
    pub fn bit_rate(&self, w: u32, h: u32) -> u64 {
        (w as f64 * h as f64 * self.effective_fps() as f64 * self.quality.bits_per_pixel()) as u64
    }

    /// Rough bytes per minute for recording `w`×`h` pixels on a display with
    /// `scale` pixels per point, for showing people the cost up front. MP4
    /// follows the bit rate; GIF is a rough empirical figure for screen content.
    pub fn bytes_per_minute(&self, w: u32, h: u32, scale: f64) -> u64 {
        let (ow, oh) = self.output_size(w, h, scale);
        match self.format {
            VideoFormat::Mp4 => (self.bit_rate(ow, oh) + if self.has_sound() { Self::AUDIO_BIT_RATE } else { 0 }) * 60 / 8,
            VideoFormat::Gif => (ow as f64 * oh as f64 * self.effective_fps() as f64 * 60.0 * 0.12) as u64,
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
    /// Start recording (pressed again: stop).
    pub record: String,
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
                record: "alt+shift+KeyV".into(),
            }
        } else {
            // Greenshot's defaults.
            Self {
                region: "PrintScreen".into(),
                window: "alt+PrintScreen".into(),
                fullscreen: "control+PrintScreen".into(),
                last_region: "shift+PrintScreen".into(),
                record: "control+shift+PrintScreen".into(),
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
    pub recording: RecordingSettings,
    /// Interface language code (`en`, `fr`, …); `None` follows the system.
    pub language: Option<String>,
    /// Region used by "Capture last region".
    pub last_region: Option<Region>,
    /// The app "Open in another app" uses (`.app` on macOS, a program
    /// elsewhere); `None` for the system's default image app.
    pub external_app: Option<PathBuf>,
    /// Check GitHub for a newer version at start-up and once a day.
    pub check_for_updates: bool,
    /// The newest version the user has been told about, so each one is
    /// announced once.
    pub announced_version: Option<String>,
    /// The custom uploader (Settings → Uploads); unused until set up.
    pub uploader: crate::share::upload::Uploader,
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
            recording: RecordingSettings::default(),
            language: None,
            last_region: None,
            external_app: None,
            check_for_updates: true,
            announced_version: None,
            uploader: Default::default(),
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
    fn recording_sizes_and_rates() {
        let r = RecordingSettings { format: VideoFormat::Mp4, quality: VideoQuality::Medium, full_resolution: false, fps: 30, ..Default::default() };
        // Half resolution on Retina, even dimensions.
        assert_eq!(r.output_size(2940, 1913, 2.0), (1470, 956));
        assert_eq!(RecordingSettings { full_resolution: true, ..r.clone() }.output_size(2940, 1912, 2.0), (2940, 1912));
        // Medium at 1470x956x30fps is about 3.4 Mbit/s, about 25 MB a minute.
        let mb = r.bytes_per_minute(2940, 1912, 2.0) as f64 / 1e6;
        assert!((24.0..27.0).contains(&mb), "{mb}");
        // GIF: capped frame rate and width.
        let g = RecordingSettings { format: VideoFormat::Gif, fps: 60, full_resolution: true, ..r };
        assert_eq!(g.effective_fps(), 15);
        assert_eq!(g.output_size(2940, 1912, 2.0).0, 1280);
    }

    #[test]
    fn partial_file_uses_defaults() {
        let c: Config = toml::from_str("format = \"jpg\"").unwrap();
        assert_eq!(c.format, OutputFormat::Jpg);
        assert_eq!(c.jpeg_quality, 90);
    }
}
