//! Whole-image effects (Greenshot's Effects menu). Each one changes the base
//! image and moves, rotates or scales the annotations to match, so they stay
//! editable afterwards.

use std::sync::Arc;

use egui::{Color32, Pos2, Rect, Vec2};
use image::{imageops, Rgba, RgbaImage};

use crate::document::{Document, Kind};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// Solid border around the image.
    Border { width: u32, color: Color32 },
    /// Soft shadow below and to the right; the new margin is transparent.
    DropShadow { offset: u32, blur: f32 },
    /// Jagged, torn-paper edges (made transparent).
    TornEdges { tooth: u32 },
    Grayscale,
    Invert,
    RotateClockwise,
    RotateCounterClockwise,
    /// Scale by a factor (0.5 = half size).
    Resize { scale: f32 },
}

impl Document {
    /// Apply an effect to the base image and keep the annotations aligned.
    pub fn apply_effect(&mut self, effect: Effect) {
        let base = self.base.as_ref();
        let (w, h) = (base.width() as f32, base.height() as f32);
        match effect {
            Effect::Border { width, color } => {
                let [r, g, b, a] = color.to_srgba_unmultiplied();
                let mut out = RgbaImage::from_pixel(base.width() + 2 * width, base.height() + 2 * width, Rgba([r, g, b, a]));
                imageops::replace(&mut out, base, width as i64, width as i64);
                self.base = Arc::new(out);
                self.map(|p| p + Vec2::splat(width as f32), 1.0);
            }
            Effect::DropShadow { offset, blur } => {
                let pad = (blur * 3.0).ceil() as u32;
                let (bw, bh) = base.dimensions();
                let (ow, oh) = (bw + 2 * pad + offset, bh + 2 * pad + offset);
                // The shadow is the image's opaque area, blurred and darkened.
                let mut silhouette = RgbaImage::new(ow, oh);
                for (x, y, p) in base.enumerate_pixels() {
                    silhouette.put_pixel(x + pad + offset, y + pad + offset, Rgba([0, 0, 0, p.0[3] / 2]));
                }
                let mut out = imageops::fast_blur(&silhouette, blur.max(0.5));
                imageops::overlay(&mut out, base, pad as i64, pad as i64);
                self.base = Arc::new(out);
                self.map(|p| p + Vec2::splat(pad as f32), 1.0);
            }
            Effect::TornEdges { tooth } => {
                let mut out = base.clone();
                torn_edges(&mut out, tooth.max(2));
                self.base = Arc::new(out);
            }
            Effect::Grayscale => {
                let mut out = base.clone();
                for p in out.pixels_mut() {
                    let [r, g, b, _] = p.0;
                    let y = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32).round() as u8;
                    p.0[..3].copy_from_slice(&[y, y, y]);
                }
                self.base = Arc::new(out);
            }
            Effect::Invert => {
                let mut out = base.clone();
                for p in out.pixels_mut() {
                    for c in &mut p.0[..3] {
                        *c = 255 - *c;
                    }
                }
                self.base = Arc::new(out);
            }
            Effect::RotateClockwise => {
                self.base = Arc::new(imageops::rotate90(base));
                self.map(|p| Pos2::new(h - p.y, p.x), 1.0);
            }
            Effect::RotateCounterClockwise => {
                self.base = Arc::new(imageops::rotate270(base));
                self.map(|p| Pos2::new(p.y, w - p.x), 1.0);
            }
            Effect::Resize { scale } => {
                let scale = scale.clamp(0.05, 8.0);
                let nw = ((w * scale).round() as u32).max(1);
                let nh = ((h * scale).round() as u32).max(1);
                self.base = Arc::new(imageops::resize(base, nw, nh, imageops::FilterType::Lanczos3));
                self.map(|p| (p.to_vec2() * scale).to_pos2(), scale);
            }
        }
    }

    /// Move every annotation point through `f`, and scale sizes by `size`.
    fn map(&mut self, f: impl Fn(Pos2) -> Pos2, size: f32) {
        for e in &mut self.elements {
            e.a = f(e.a);
            e.b = f(e.b);
            e.tail = f(e.tail);
            for p in &mut e.points {
                *p = f(*p);
            }
            if !matches!(e.kind, Kind::Line | Kind::Arrow | Kind::Freehand) {
                // Boxes stay axis-aligned after 90° turns; keep a = top-left.
                let r = Rect::from_two_pos(e.a, e.b);
                e.a = r.min;
                e.b = r.max;
            }
            e.style.width *= size;
            e.style.font_size *= size;
            e.style.strength *= size;
            e.fit_content();
        }
    }
}

/// Cut a deterministic saw-tooth into all four edges, making the part
/// outside it transparent.
fn torn_edges(img: &mut RgbaImage, tooth: u32) {
    let (w, h) = img.dimensions();
    if w < tooth * 4 || h < tooth * 4 {
        return;
    }
    // Depth of the tear at position `i` along an edge, between 0 and `tooth`.
    let depth = |i: u32, edge: u32| -> u32 {
        let period = tooth * 2;
        let phase = i % period;
        let saw = if phase < tooth { phase } else { period - phase };
        // A little variation per tooth so it doesn't look machine-made.
        let n = (i / period).wrapping_mul(2_654_435_761).wrapping_add(edge * 97) % 5;
        (saw * (5 + n) / 9).min(tooth)
    };
    for (x, y, p) in img.enumerate_pixels_mut() {
        let cut = y < depth(x, 0) || h - 1 - y < depth(x, 1) || x < depth(y, 2) || w - 1 - x < depth(y, 3);
        if cut {
            p.0[3] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Element, Style};

    fn doc_with_rect() -> Document {
        let mut doc = Document::new(RgbaImage::from_pixel(100, 50, Rgba([10, 20, 30, 255])));
        let mut r = Element::new(Kind::Rect, Pos2::new(10.0, 5.0), Style::default());
        r.b = Pos2::new(30.0, 15.0);
        doc.elements.push(r);
        doc
    }

    #[test]
    fn border_grows_image_and_shifts_annotations() {
        let mut doc = doc_with_rect();
        doc.apply_effect(Effect::Border { width: 4, color: Color32::WHITE });
        assert_eq!(doc.base.dimensions(), (108, 58));
        assert_eq!(doc.base.get_pixel(1, 1).0, [255, 255, 255, 255]);
        assert_eq!(doc.elements[0].a, Pos2::new(14.0, 9.0));
    }

    #[test]
    fn rotate_clockwise_moves_box_and_swaps_size() {
        let mut doc = doc_with_rect();
        doc.apply_effect(Effect::RotateClockwise);
        assert_eq!(doc.base.dimensions(), (50, 100));
        // (10,5)-(30,15) in a 100x50 image becomes (35,10)-(45,30).
        let r = doc.elements[0].bounds();
        assert_eq!((r.min, r.max), (Pos2::new(35.0, 10.0), Pos2::new(45.0, 30.0)));
        // Four turns get back where we started.
        for _ in 0..3 {
            doc.apply_effect(Effect::RotateClockwise);
        }
        assert_eq!(doc.elements[0].bounds().min, Pos2::new(10.0, 5.0));
    }

    #[test]
    fn rotate_back_and_forth_is_identity() {
        let mut doc = doc_with_rect();
        doc.apply_effect(Effect::RotateClockwise);
        doc.apply_effect(Effect::RotateCounterClockwise);
        assert_eq!(doc.base.dimensions(), (100, 50));
        assert_eq!(doc.elements[0].bounds().min, Pos2::new(10.0, 5.0));
    }

    #[test]
    fn resize_scales_image_points_and_line_width() {
        let mut doc = doc_with_rect();
        doc.apply_effect(Effect::Resize { scale: 2.0 });
        assert_eq!(doc.base.dimensions(), (200, 100));
        assert_eq!(doc.elements[0].b, Pos2::new(60.0, 30.0));
        assert_eq!(doc.elements[0].style.width, Style::default().width * 2.0);
    }

    #[test]
    fn grayscale_and_invert() {
        let mut doc = doc_with_rect();
        doc.apply_effect(Effect::Invert);
        assert_eq!(doc.base.get_pixel(0, 0).0, [245, 235, 225, 255]);
        doc.apply_effect(Effect::Grayscale);
        let p = doc.base.get_pixel(0, 0).0;
        assert!(p[0] == p[1] && p[1] == p[2]);
    }

    #[test]
    fn drop_shadow_adds_transparent_margin_with_shadow() {
        let mut doc = doc_with_rect();
        doc.apply_effect(Effect::DropShadow { offset: 6, blur: 3.0 });
        let (w, h) = doc.base.dimensions();
        assert_eq!((w, h), (100 + 18 + 6, 50 + 18 + 6));
        // Top-left corner of the margin: transparent. Below-right: shadow.
        assert_eq!(doc.base.get_pixel(0, 0).0[3], 0);
        assert!(doc.base.get_pixel(w - 12, h - 12).0[3] > 0);
        assert_eq!(doc.elements[0].a, Pos2::new(19.0, 14.0));
    }

    #[test]
    fn torn_edges_cut_the_border_but_keep_the_middle() {
        let mut doc = doc_with_rect();
        doc.apply_effect(Effect::TornEdges { tooth: 6 });
        assert_eq!(doc.base.dimensions(), (100, 50));
        assert_eq!(doc.base.get_pixel(50, 25).0[3], 255);
        let transparent_edge = (0..100).filter(|&x| doc.base.get_pixel(x, 0).0[3] == 0).count();
        assert!(transparent_edge > 30);
    }
}
