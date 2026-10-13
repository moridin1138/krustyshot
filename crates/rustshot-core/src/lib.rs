//! Platform-independent logic for Rustshot: screen capture, configuration,
//! Greenshot-style filename patterns and image output.

pub mod capture;
pub mod config;
pub mod filename;
pub mod geometry;
pub mod output;
pub mod permissions;

pub use capture::{MonitorShot, Snapshot, WindowInfo};
pub use config::{Config, Destination, OutputFormat};
pub use geometry::Region;
pub use image::RgbaImage;
