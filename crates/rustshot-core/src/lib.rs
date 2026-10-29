//! Platform-independent logic for Rustshot: screen capture, configuration,
//! Greenshot-style filename patterns and image output.

pub mod autostart;
pub mod capture;
pub mod config;
pub mod disk;
pub mod filename;
pub mod geometry;
pub mod output;
pub mod permissions;
pub mod pointer;
pub mod recording;
#[cfg(target_os = "macos")]
pub mod sck;
pub mod scroll;
pub mod share;
pub mod update;

pub use capture::{MonitorShot, Snapshot, WindowInfo};
pub use config::{Config, Destination, OutputFormat, RecordingSettings, TrayClick, VideoFormat, VideoQuality};
pub use geometry::Region;
pub use image;
pub use image::RgbaImage;
