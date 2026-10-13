//! Pixel effects for highlight and obfuscate elements.

use egui::{Color32, Rect};
use image::{imageops, Rgba, RgbaImage};

use crate::document::{Element, Kind, ObfuscateMode};

/// Clamp an image-space rect to whole pixels inside the image.
fn pixel_bounds(img: &RgbaImage, r: Rect) -> Option<(u32, u32, u32, u32)> {
    let x0 = r.min.x.round().max(0.0) as u32;
    let y0 = r.min.y.round().max(0.0) as u32;
    let x1 = (r.max.x.round().max(0.0) as u32).min(img.width());
    let y1 = (r.max.y.round().max(0.0) as u32).min(img.height());
    (x1 > x0 && y1 > y0).then(|| (x0, y0, x1 - x0, y1 - y0))
}

pub fn pixelate(img: &mut RgbaImage, r: Rect, block: u32) {
    let Some((x0, y0, w, h)) = pixel_bounds(img, r) else { return };
    let block = block.max(2);
    let mut by = y0;
    while by < y0 + h {
        let bh = block.min(y0 + h - by);
        let mut bx = x0;
        while bx < x0 + w {
            let bw = block.min(x0 + w - bx);
            let mut sum = [0u64; 4];
            for y in by..by + bh {
                for x in bx..bx + bw {
                    let p = img.get_pixel(x, y).0;
                    for c in 0..4 {
                        sum[c] += p[c] as u64;
                    }
                }
            }
            let n = (bw * bh) as u64;
            let avg = Rgba([(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, (sum[3] / n) as u8]);
            for y in by..by + bh {
                for x in bx..bx + bw {
                    img.put_pixel(x, y, avg);
                }
            }
            bx += block;
        }
        by += block;
    }
}

pub fn blur(img: &mut RgbaImage, r: Rect, radius: f32) {
    let Some((x0, y0, w, h)) = pixel_bounds(img, r) else { return };
    let sub = imageops::crop_imm(img, x0, y0, w, h).to_image();
    let blurred = imageops::fast_blur(&sub, radius.max(1.0));
    imageops::replace(img, &blurred, x0 as i64, y0 as i64);
}

/// Multiply blend, like a highlighter pen.
pub fn highlight(img: &mut RgbaImage, r: Rect, color: Color32) {
    let Some((x0, y0, w, h)) = pixel_bounds(img, r) else { return };
    let m = [color.r() as u32, color.g() as u32, color.b() as u32];
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let p = img.get_pixel_mut(x, y);
            for (ch, mul) in p.0.iter_mut().zip(m) {
                *ch = ((*ch as u32 * mul) / 255) as u8;
            }
        }
    }
}

pub fn apply(img: &mut RgbaImage, e: &Element) {
    let r = e.bounds();
    match e.kind {
        Kind::Highlight => highlight(img, r, e.style.highlight),
        Kind::Obfuscate => match e.style.obfuscate {
            ObfuscateMode::Pixelate => pixelate(img, r, e.style.strength.round() as u32),
            ObfuscateMode::Blur => blur(img, r, e.style.strength),
        },
        _ => {}
    }
}

/// The base image with every raster element applied, in stacking order.
pub fn composite<'a>(base: &RgbaImage, elements: impl IntoIterator<Item = &'a Element>) -> RgbaImage {
    let mut img = base.clone();
    for e in elements {
        if e.kind.is_raster() {
            apply(&mut img, e);
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;

    #[test]
    fn pixelate_averages_blocks() {
        let mut img = RgbaImage::new(4, 2);
        img.put_pixel(0, 0, Rgba([200, 0, 0, 255]));
        img.put_pixel(1, 1, Rgba([0, 0, 0, 255]));
        pixelate(&mut img, Rect::from_min_max(Pos2::ZERO, Pos2::new(4.0, 2.0)), 2);
        assert_eq!(img.get_pixel(0, 0), img.get_pixel(1, 1));
        assert_eq!(img.get_pixel(0, 0).0[0], 50);
    }

    #[test]
    fn highlight_multiplies() {
        let mut img = RgbaImage::from_pixel(2, 2, Rgba([255, 255, 255, 255]));
        highlight(&mut img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::from_rgb(255, 255, 0));
        assert_eq!(img.get_pixel(0, 0).0, [255, 255, 0, 255]);
        assert_eq!(img.get_pixel(1, 1).0, [255, 255, 255, 255]);
    }

    #[test]
    fn out_of_bounds_is_ignored() {
        let mut img = RgbaImage::new(4, 4);
        blur(&mut img, Rect::from_min_max(Pos2::new(10.0, 10.0), Pos2::new(20.0, 20.0)), 3.0);
        pixelate(&mut img, Rect::from_min_max(Pos2::new(-5.0, -5.0), Pos2::new(2.0, 2.0)), 4);
    }
}
