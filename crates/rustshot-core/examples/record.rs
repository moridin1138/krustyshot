//! Dev tool: record the primary display without the app.
//!
//!   cargo run --release -p rustshot-core --example record -- out.mp4 [seconds] [low|medium|high] [full]
//!
//! The format follows the file extension (.mp4 or .gif).

use std::time::Duration;

use rustshot_core::recording::{CropRect, Recording};
use rustshot_core::{RecordingSettings, VideoFormat, VideoQuality};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = std::path::PathBuf::from(args.next().expect("output path"));
    let secs: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(5.0);
    let quality = match args.next().as_deref() {
        Some("low") => VideoQuality::Low,
        Some("high") => VideoQuality::High,
        _ => VideoQuality::Medium,
    };
    let full_resolution = args.next().as_deref() == Some("full");
    let format = if path.extension().is_some_and(|e| e == "gif") { VideoFormat::Gif } else { VideoFormat::Mp4 };
    let settings = RecordingSettings { format, quality, full_resolution, ..RecordingSettings::default() };

    let monitors = xcap::Monitor::all()?;
    let m = monitors.iter().find(|m| m.is_primary().unwrap_or(false)).unwrap_or(&monitors[0]);
    let (w, h, scale) = rustshot_core::capture::primary_display_pixels().expect("display");
    let crop = CropRect { x: 0, y: 0, width: w, height: h };
    let estimate = settings.bytes_per_minute(w, h, scale);
    println!("estimate: {} per minute", rustshot_core::disk::format_bytes(estimate));

    let rec = Recording::start(m.id()?, crop, scale, &settings, &path)?;
    std::thread::sleep(Duration::from_secs_f64(secs));
    let r = rec.finish(false);
    let size = std::fs::metadata(&r.path).map(|m| m.len()).unwrap_or(0);
    println!(
        "{:?}: {} frames, {} dropped, {:.1}s, {} (= {} per minute)",
        r.reason,
        r.frames,
        r.dropped,
        r.duration.as_secs_f64(),
        rustshot_core::disk::format_bytes(size),
        rustshot_core::disk::format_bytes((size as f64 / r.duration.as_secs_f64() * 60.0) as u64)
    );
    Ok(())
}
