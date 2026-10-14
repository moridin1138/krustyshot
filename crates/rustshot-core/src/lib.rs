//! Platform-independent logic for Rustshot: screen capture, configuration,
//! Greenshot-style filename patterns and image output.

pub mod capture;
pub mod config;
pub mod filename;
pub mod geometry;
pub mod output;
pub mod permissions;
pub mod pointer;

pub use capture::{MonitorShot, Snapshot, WindowInfo};
pub use config::{Config, Destination, OutputFormat, TrayClick};
pub use geometry::Region;
pub use image;
pub use image::RgbaImage;
