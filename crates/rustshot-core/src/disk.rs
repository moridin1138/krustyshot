//! Free-space checks for the disk that screenshots and recordings go to.

use std::path::{Path, PathBuf};

/// Warn when the destination disk has this fraction of its space or less left.
pub const LOW_SPACE_FRACTION: f64 = 0.05;

/// Stop a recording before the disk is completely full: below this many
/// bytes, or below 1% of the disk, whichever is larger.
pub const RECORDING_FLOOR_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskSpace {
    /// Bytes this user can still write.
    pub available: u64,
    pub total: u64,
}

impl DiskSpace {
    pub fn fraction_free(&self) -> f64 {
        if self.total == 0 { 1.0 } else { self.available as f64 / self.total as f64 }
    }

    /// At or below the warning threshold (5% free).
    pub fn is_low(&self) -> bool {
        self.fraction_free() <= LOW_SPACE_FRACTION
    }

    /// So low that an ongoing recording should stop and save what it has.
    pub fn is_critical(&self) -> bool {
        self.available < RECORDING_FLOOR_BYTES.max(self.total / 100)
    }
}

/// The folder itself, or its nearest ancestor that exists (the output folder
/// may not have been created yet).
fn existing_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors().find(|p| p.exists()).map(Path::to_path_buf)
}

/// Space on the disk holding `path`. `None` if it can't be determined.
pub fn space_for(path: &Path) -> Option<DiskSpace> {
    let dir = existing_ancestor(path)?;
    let available = fs4::available_space(&dir).ok()?;
    let total = fs4::total_space(&dir).ok()?;
    Some(DiskSpace { available, total })
}

/// Human-readable size: "512 MB", "1.4 GB".
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit <= 2 || value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds() {
        let gb = 1_000_000_000;
        let s = |available, total| DiskSpace { available, total };
        assert!(s(5 * gb, 100 * gb).is_low());
        assert!(!s(6 * gb, 100 * gb).is_low());
        // 1% of a 1 TB disk is 10 GB: below that a recording stops.
        assert!(s(9 * gb, 1000 * gb).is_critical());
        assert!(!s(11 * gb, 1000 * gb).is_critical());
        // Small disk: the 512 MB floor applies.
        assert!(s(400_000_000, 20 * gb).is_critical());
    }

    #[test]
    fn real_disk_and_missing_folder() {
        let tmp = std::env::temp_dir();
        let space = space_for(&tmp.join("does/not/exist/yet")).expect("falls back to an existing parent");
        assert!(space.total > 0 && space.available <= space.total);
    }

    #[test]
    fn formatting() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(25_000_000), "25 MB");
        assert_eq!(format_bytes(1_400_000_000), "1.4 GB");
        assert_eq!(format_bytes(494_000_000_000), "494 GB");
    }
}
