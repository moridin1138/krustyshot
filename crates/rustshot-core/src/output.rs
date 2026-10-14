//! Writing captures to disk and to the clipboard.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::{DynamicImage, RgbaImage};

use crate::config::{Config, OutputFormat};
use crate::filename::{self, PatternContext};

/// Encode `img` to `path` in the given format.
pub fn save_image(img: &RgbaImage, path: &Path, format: OutputFormat, jpeg_quality: u8) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let file = std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut w = std::io::BufWriter::new(file);
    match format {
        OutputFormat::Png => DynamicImage::ImageRgba8(img.clone()).write_to(&mut w, image::ImageFormat::Png)?,
        OutputFormat::Jpg => {
            let rgb = on_white(img);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut w, jpeg_quality.clamp(1, 100))
                .encode_image(&rgb)?;
        }
        OutputFormat::Bmp => on_white(img).write_to(&mut w, image::ImageFormat::Bmp)?,
        OutputFormat::Gif => DynamicImage::ImageRgba8(img.clone()).write_to(&mut w, image::ImageFormat::Gif)?,
    }
    Ok(())
}

/// Formats without transparency: put transparent areas (from effects such
/// as drop shadow or torn edges) on white instead of letting them turn black.
fn on_white(img: &RgbaImage) -> image::RgbImage {
    image::RgbImage::from_fn(img.width(), img.height(), |x, y| {
        let [r, g, b, a] = img.get_pixel(x, y).0;
        let a = a as u16;
        let blend = |c: u8| ((c as u16 * a + 255 * (255 - a) + 127) / 255) as u8;
        image::Rgb([blend(r), blend(g), blend(b)])
    })
}

/// Path for "Save directly": output folder + expanded pattern + extension,
/// made unique by appending a counter.
pub fn auto_path(config: &Config, title: &str, time: chrono::DateTime<chrono::Local>) -> PathBuf {
    let stem = filename::expand(&config.filename_pattern, &PatternContext { time, title });
    unique_path(&config.output_dir, &stem, config.format.extension())
}

/// Suggested file name (without folder) for a Save dialog.
pub fn suggested_name(config: &Config, title: &str, time: chrono::DateTime<chrono::Local>) -> String {
    let stem = filename::expand(&config.filename_pattern, &PatternContext { time, title });
    format!("{stem}.{}", config.format.extension())
}

pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut path = dir.join(format!("{stem}.{ext}"));
    let mut n = 1;
    while path.exists() {
        path = dir.join(format!("{stem} ({n}).{ext}"));
        n += 1;
    }
    path
}

/// Save using the format implied by the path's extension, or the configured one.
pub fn save_to(img: &RgbaImage, path: &Path, config: &Config) -> Result<()> {
    let format = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(OutputFormat::from_extension)
        .unwrap_or(config.format);
    save_image(img, path, format, config.jpeg_quality)
}

pub fn copy_image_to_clipboard(img: &RgbaImage) -> Result<()> {
    let mut cb = arboard::Clipboard::new().context("opening clipboard")?;
    cb.set_image(arboard::ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: std::borrow::Cow::Borrowed(img.as_raw()),
    })
    .context("copying image to clipboard")
}

pub fn copy_text_to_clipboard(text: &str) -> Result<()> {
    arboard::Clipboard::new()?.set_text(text.to_string())?;
    Ok(())
}

/// Read an image from the clipboard, if there is one.
pub fn image_from_clipboard() -> Option<RgbaImage> {
    let data = arboard::Clipboard::new().ok()?.get_image().ok()?;
    RgbaImage::from_raw(data.width as u32, data.height as u32, data.bytes.into_owned())
}

pub fn load_image(path: &Path) -> Result<RgbaImage> {
    Ok(image::open(path).with_context(|| format!("opening {}", path.display()))?.to_rgba8())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_path_counts_up() {
        let dir = std::env::temp_dir().join(format!("rustshot-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = unique_path(&dir, "a", "png");
        std::fs::write(&first, b"x").unwrap();
        assert_eq!(unique_path(&dir, "a", "png"), dir.join("a (1).png"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn transparent_pixels_become_white_in_jpg() {
        let img = RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 0]));
        assert_eq!(on_white(&img).get_pixel(0, 0).0, [255, 255, 255]);
        let half = RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 128]));
        assert_eq!(on_white(&half).get_pixel(0, 0).0, [127, 127, 127]);
    }

    #[test]
    fn save_all_formats() {
        let dir = std::env::temp_dir().join(format!("rustshot-fmt-{}", std::process::id()));
        let img = RgbaImage::from_pixel(4, 3, image::Rgba([10, 20, 30, 255]));
        for f in OutputFormat::ALL {
            let p = dir.join(format!("t.{}", f.extension()));
            save_image(&img, &p, f, 80).unwrap();
            let back = image::open(&p).unwrap();
            assert_eq!((back.width(), back.height()), (4, 3));
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
