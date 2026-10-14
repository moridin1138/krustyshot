//! Text recognition ("copy text from the image") with the OS's own engine:
//! Vision on macOS, Windows.Media.Ocr on Windows, Tesseract (if installed)
//! on Linux. It can take a second or two, so call it off the UI thread.

use anyhow::Result;
use image::RgbaImage;

/// The text in `image`, one line per line of text, top to bottom.
pub fn recognize(image: &RgbaImage) -> Result<String> {
    #[cfg(target_os = "macos")]
    return macos::recognize(image);
    #[cfg(windows)]
    return windows_ocr::recognize(image);
    #[cfg(all(unix, not(target_os = "macos")))]
    return tesseract::recognize(image);
}

/// The recognition languages to ask for, best first: the app's language,
/// then English.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn languages() -> Vec<&'static str> {
    let first = match rustshot_i18n::current_language() {
        "fr" => Some("fr-FR"),
        "de" => Some("de-DE"),
        "es" => Some("es-ES"),
        _ => None,
    };
    first.into_iter().chain(["en-US"]).collect()
}

/// Lines of text with where they are (top edge, left edge; any units where
/// larger `top` means lower down), joined in reading order.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn join_lines(mut lines: Vec<(f64, f64, String)>) -> String {
    lines.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    lines.into_iter().map(|(_, _, t)| t).collect::<Vec<_>>().join("\n")
}

#[cfg(target_os = "macos")]
mod macos {
    use std::io::Cursor;
    use std::ptr::null_mut;

    use anyhow::{anyhow, bail, Context, Result};
    use image::RgbaImage;
    use objc2::rc::{Allocated, Retained};
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2::msg_send;
    use objc2_foundation::{NSArray, NSData, NSDictionary, NSError, NSRect, NSString};

    #[link(name = "Vision", kind = "framework")]
    unsafe extern "C" {}

    fn class(name: &std::ffi::CStr) -> Result<&'static AnyClass> {
        AnyClass::get(name).ok_or_else(|| anyhow!("{} is missing", name.to_string_lossy()))
    }

    pub fn recognize(image: &RgbaImage) -> Result<String> {
        let mut png = Vec::new();
        image.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)?;
        // Some languages aren't available on every macOS version; fall back
        // to the system's default languages if asking for them fails.
        match run(&png, Some(&super::languages())) {
            Ok(text) => Ok(text),
            Err(e) => {
                log::info!("text recognition with chosen languages failed ({e:#}); trying the defaults");
                run(&png, None)
            }
        }
    }

    fn run(png: &[u8], languages: Option<&[&str]>) -> Result<String> {
        unsafe {
            let data = NSData::with_bytes(png);
            let options = NSDictionary::<NSString, AnyObject>::new();
            let handler: Allocated<AnyObject> = msg_send![class(c"VNImageRequestHandler")?, alloc];
            let handler: Retained<AnyObject> = msg_send![handler, initWithData: &*data, options: &*options];

            let request: Retained<AnyObject> = msg_send![class(c"VNRecognizeTextRequest")?, new];
            // VNRequestTextRecognitionLevelAccurate
            let _: () = msg_send![&request, setRecognitionLevel: 0isize];
            let _: () = msg_send![&request, setUsesLanguageCorrection: true];
            if let Some(languages) = languages {
                let names: Vec<Retained<NSString>> = languages.iter().map(|l| NSString::from_str(l)).collect();
                let list = NSArray::from_retained_slice(&names);
                let _: () = msg_send![&request, setRecognitionLanguages: &*list];
            }

            let requests = NSArray::from_retained_slice(std::slice::from_ref(&request));
            let mut error: *mut NSError = null_mut();
            let ok: bool = msg_send![&handler, performRequests: &*requests, error: &mut error];
            if !ok {
                let why = error.as_ref().map(|e| e.localizedDescription().to_string()).unwrap_or_default();
                bail!("text recognition failed: {why}");
            }

            let results: *mut AnyObject = msg_send![&request, results];
            let results = results.as_ref().context("no results")?;
            let count: usize = msg_send![results, count];
            let mut lines = Vec::with_capacity(count);
            for i in 0..count {
                let observation: *mut AnyObject = msg_send![results, objectAtIndex: i];
                let candidates: *mut AnyObject = msg_send![observation, topCandidates: 1usize];
                let best: *mut AnyObject = msg_send![candidates, firstObject];
                let Some(best) = best.as_ref() else { continue };
                let text: *mut NSString = msg_send![best, string];
                let Some(text) = text.as_ref() else { continue };
                // Normalised, with the origin at the bottom left.
                let bounds: NSRect = msg_send![observation, boundingBox];
                let top = 1.0 - (bounds.origin.y + bounds.size.height);
                lines.push((top, bounds.origin.x, text.to_string()));
            }
            Ok(super::join_lines(lines))
        }
    }
}

#[cfg(windows)]
mod windows_ocr {
    use anyhow::{Context, Result};
    use image::{imageops, RgbaImage};
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Security::Cryptography::CryptographicBuffer;

    pub fn recognize(image: &RgbaImage) -> Result<String> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()
            .context("Windows has no text recognition for your languages (add a language with OCR in Settings → Time & language)")?;
        // The engine refuses images over a size limit; scale those down.
        let max = OcrEngine::MaxImageDimension().unwrap_or(2600);
        let (w, h) = image.dimensions();
        let scaled;
        let image = if w.max(h) > max {
            let f = max as f64 / w.max(h) as f64;
            scaled = imageops::resize(image, (w as f64 * f) as u32, (h as f64 * f) as u32, imageops::FilterType::Triangle);
            &scaled
        } else {
            image
        };
        let (w, h) = image.dimensions();
        let bgra: Vec<u8> = image.as_raw().as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
        let buffer = CryptographicBuffer::CreateFromByteArray(&bgra)?;
        let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(&buffer, BitmapPixelFormat::Bgra8, w as i32, h as i32, BitmapAlphaMode::Premultiplied)?;
        let result = engine.RecognizeAsync(&bitmap)?.join().context("text recognition failed")?;
        let lines = result.Lines()?;
        let text: Vec<String> = lines.into_iter().filter_map(|l| l.Text().ok()).map(|t| t.to_string_lossy()).collect();
        Ok(text.join("\n"))
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod tesseract {
    use std::io::{Cursor, Write};
    use std::process::{Command, Stdio};

    use anyhow::{bail, Context, Result};
    use image::RgbaImage;

    pub fn recognize(image: &RgbaImage) -> Result<String> {
        let mut png = Vec::new();
        image.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)?;
        let mut child = Command::new("tesseract")
            .args(["stdin", "stdout"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("copying text needs Tesseract (install the `tesseract-ocr` package)")?;
        child.stdin.take().context("no stdin")?.write_all(&png)?;
        let out = child.wait_with_output()?;
        if !out.status.success() {
            bail!("Tesseract failed ({})", out.status);
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_in_reading_order() {
        let lines = vec![(0.5, 0.1, "second".to_string()), (0.1, 0.6, "first b".to_string()), (0.1, 0.1, "first a".to_string())];
        assert_eq!(join_lines(lines), "first a\nfirst b\nsecond");
    }

    /// Renders text and reads it back with the real engine (macOS only:
    /// CI's Linux has no Tesseract and Windows needs a language pack).
    #[test]
    #[cfg(target_os = "macos")]
    fn reads_rendered_text() {
        use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
        let Ok(bytes) = std::fs::read("/System/Library/Fonts/Supplemental/Arial.ttf") else { return };
        let font = FontRef::try_from_slice(&bytes).unwrap();
        let scaled = font.as_scaled(PxScale::from(48.0));
        let mut image = RgbaImage::from_pixel(420, 100, image::Rgba([255, 255, 255, 255]));
        let mut x = 20.0;
        for c in "Rustshot 42".chars() {
            let id = font.glyph_id(c);
            let glyph = id.with_scale_and_position(48.0, ab_glyph::point(x, 65.0));
            x += scaled.h_advance(id);
            if let Some(outline) = font.outline_glyph(glyph) {
                let b = outline.px_bounds();
                outline.draw(|gx, gy, cov| {
                    let (px, py) = (b.min.x as u32 + gx, b.min.y as u32 + gy);
                    if px < 420 && py < 100 && cov > 0.0 {
                        let v = (255.0 * (1.0 - cov)) as u8;
                        image.put_pixel(px, py, image::Rgba([v, v, v, 255]));
                    }
                });
            }
        }
        let text = recognize(&image).unwrap();
        assert!(text.contains("Rustshot 42"), "{text:?}");
    }
}
