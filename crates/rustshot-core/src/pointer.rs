//! The mouse pointer: where it is, and drawing it into a capture.
//!
//! Screen-capture APIs don't include the pointer, so Rustshot reads its
//! position when the capture is taken and draws a standard arrow there.

use image::RgbaImage;
use tiny_skia::{FillRule, LineJoin, Paint, PathBuilder, PixmapMut, Stroke, Transform};

/// Pointer position in the platform's raw global coordinates: points on
/// macOS, physical pixels on Windows and X11. `None` if it can't be read
/// (for example under Wayland).
pub fn raw_position() -> Option<(f64, f64)> {
    imp::position()
}

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::c_void;

    #[repr(C)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventCreate(source: *const c_void) -> *const c_void;
        fn CGEventGetLocation(event: *const c_void) -> CGPoint;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(cf: *const c_void);
    }

    pub fn position() -> Option<(f64, f64)> {
        // A null-source event reports the current pointer location in global
        // top-left coordinates, the same space as display bounds.
        unsafe {
            let event = CGEventCreate(std::ptr::null());
            if event.is_null() {
                return None;
            }
            let p = CGEventGetLocation(event);
            CFRelease(event);
            Some((p.x, p.y))
        }
    }
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    pub fn position() -> Option<(f64, f64)> {
        let mut p = POINT { x: 0, y: 0 };
        (unsafe { GetCursorPos(&mut p) } != 0).then_some((p.x as f64, p.y as f64))
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::ConnectionExt;

    pub fn position() -> Option<(f64, f64)> {
        let (conn, screen) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        let reply = conn.query_pointer(root).ok()?.reply().ok()?;
        Some((reply.root_x as f64, reply.root_y as f64))
    }
}

#[cfg(not(any(target_os = "macos", windows, unix)))]
mod imp {
    pub fn position() -> Option<(f64, f64)> {
        None
    }
}

/// Outline of a classic arrow pointer, tip at (0, 0), in points.
const ARROW: [(f32, f32); 7] = [
    (0.0, 0.0),
    (0.0, 17.0),
    (4.2, 13.2),
    (7.0, 19.6),
    (9.8, 18.4),
    (7.0, 12.2),
    (12.4, 12.2),
];

/// Draw an arrow pointer with its tip at pixel (`x`, `y`). `scale` is the
/// monitor's pixels per point, so the pointer is the size users see.
pub fn draw(img: &mut RgbaImage, x: f32, y: f32, scale: f32) {
    let (w, h) = img.dimensions();
    let Some(mut pixmap) = PixmapMut::from_bytes(img.as_mut(), w, h) else { return };
    let mut pb = PathBuilder::new();
    for (i, (px, py)) in ARROW.iter().enumerate() {
        let (px, py) = (x + px * scale, y + py * scale);
        if i == 0 {
            pb.move_to(px, py);
        } else {
            pb.line_to(px, py);
        }
    }
    pb.close();
    let Some(path) = pb.finish() else { return };
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    // The capture is opaque, so straight and premultiplied RGBA coincide.
    paint.set_color_rgba8(255, 255, 255, 255);
    let stroke = Stroke { width: 2.0 * scale, line_join: LineJoin::Round, ..Stroke::default() };
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    paint.set_color_rgba8(0, 0, 0, 255);
    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_black_arrow_with_white_edge() {
        let mut img = RgbaImage::from_pixel(60, 60, image::Rgba([128, 128, 128, 255]));
        draw(&mut img, 10.0, 10.0, 2.0);
        // Inside the arrow body: black.
        assert_eq!(img.get_pixel(14, 30).0, [0, 0, 0, 255]);
        // Outside: untouched.
        assert_eq!(img.get_pixel(55, 5).0, [128, 128, 128, 255]);
        // Some white outline pixels exist.
        assert!(img.pixels().any(|p| p.0 == [255, 255, 255, 255]));
    }

    #[test]
    fn off_image_is_harmless() {
        let mut img = RgbaImage::new(10, 10);
        draw(&mut img, 500.0, 500.0, 1.0);
        draw(&mut img, -50.0, -50.0, 2.0);
    }

    #[test]
    fn position_does_not_panic() {
        let _ = raw_position();
    }
}
