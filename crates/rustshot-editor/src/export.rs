//! Flatten a document into a single image: raster effects first, then
//! vector annotations rendered with tiny-skia, then text via ab_glyph.

use ab_glyph::{Font, FontRef, OutlineCurve, PxScale, ScaleFont};
use egui::{Color32, Pos2};
use image::RgbaImage;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::document::{Document, Element, Kind};
use crate::raster;

/// Padding around text when it has a background fill (image pixels).
pub const TEXT_PADDING: f32 = 4.0;

/// Tip, left and right corners of an arrow head, plus where the shaft ends.
pub fn arrow_head(a: Pos2, b: Pos2, width: f32) -> ([Pos2; 3], Pos2) {
    let len = (width * 4.0).max(12.0);
    let dir = (b - a).normalized();
    if !dir.x.is_finite() {
        return ([b, b, b], b);
    }
    let perp = egui::vec2(-dir.y, dir.x);
    let base = b - dir * len;
    ([b, base + perp * len * 0.5, base - perp * len * 0.5], base)
}

pub fn text_font() -> FontRef<'static> {
    FontRef::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT).expect("bundled font is valid")
}

/// Width and height of a text block at `size` pixels.
pub fn measure_text(text: &str, size: f32) -> egui::Vec2 {
    let font = text_font();
    let scaled = font.as_scaled(PxScale::from(size));
    let line_h = scaled.height() + scaled.line_gap();
    let mut w: f32 = 0.0;
    let lines: Vec<&str> = text.split('\n').collect();
    for line in &lines {
        let lw: f32 = line.chars().map(|c| scaled.h_advance(font.glyph_id(c))).sum();
        w = w.max(lw);
    }
    egui::vec2(w, line_h * lines.len() as f32)
}

pub fn flatten(doc: &Document) -> RgbaImage {
    let base = raster::composite(&doc.base, &doc.elements);
    let (w, h) = base.dimensions();
    let mut data = base.into_raw();
    premultiply(&mut data);
    let Some(mut pixmap) = Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(w, h).unwrap()) else {
        return doc.base.as_ref().clone();
    };
    for e in &doc.elements {
        draw_element(&mut pixmap, e);
    }
    RgbaImage::from_raw(w, h, pixmap.take_demultiplied()).expect("pixmap size matches")
}

fn premultiply(data: &mut [u8]) {
    for px in data.as_chunks_mut::<4>().0 {
        let a = px[3] as u16;
        for c in &mut px[..3] {
            *c = ((*c as u16 * a + 127) / 255) as u8;
        }
    }
}

fn paint(c: Color32) -> Paint<'static> {
    let mut p = Paint::default();
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    p.set_color_rgba8(r, g, b, a);
    p.anti_alias = true;
    p
}

fn stroke(width: f32) -> Stroke {
    Stroke { width: width.max(0.5), line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() }
}

fn rect_of(e: &Element) -> Option<tiny_skia::Rect> {
    let r = e.bounds();
    tiny_skia::Rect::from_ltrb(r.min.x, r.min.y, r.max.x, r.max.y)
}

pub fn draw_element(pm: &mut Pixmap, e: &Element) {
    let s = &e.style;
    let id = Transform::identity();
    match e.kind {
        Kind::Rect | Kind::Ellipse => {
            let Some(r) = rect_of(e) else { return };
            let path = if e.kind == Kind::Rect {
                Some(PathBuilder::from_rect(r))
            } else {
                PathBuilder::from_oval(r)
            };
            let Some(path) = path else { return };
            if s.fill.a() > 0 {
                pm.fill_path(&path, &paint(s.fill), FillRule::Winding, id, None);
            }
            if s.width > 0.0 {
                pm.stroke_path(&path, &paint(s.stroke), &stroke(s.width), id, None);
            }
        }
        Kind::Line | Kind::Arrow => {
            let end = if e.kind == Kind::Arrow {
                let (head, shaft_end) = arrow_head(e.a, e.b, s.width);
                let mut pb = PathBuilder::new();
                pb.move_to(head[0].x, head[0].y);
                pb.line_to(head[1].x, head[1].y);
                pb.line_to(head[2].x, head[2].y);
                pb.close();
                if let Some(p) = pb.finish() {
                    pm.fill_path(&p, &paint(s.stroke), FillRule::Winding, id, None);
                }
                shaft_end
            } else {
                e.b
            };
            let mut pb = PathBuilder::new();
            pb.move_to(e.a.x, e.a.y);
            pb.line_to(end.x, end.y);
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint(s.stroke), &stroke(s.width), id, None);
            }
        }
        Kind::Freehand => {
            let mut pb = PathBuilder::new();
            if let Some(first) = e.points.first() {
                pb.move_to(first.x, first.y);
                for p in &e.points[1..] {
                    pb.line_to(p.x, p.y);
                }
            }
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint(s.stroke), &stroke(s.width), id, None);
            }
        }
        Kind::Text => draw_text(pm, e),
        Kind::Highlight | Kind::Obfuscate => {}
    }
}

fn draw_text(pm: &mut Pixmap, e: &Element) {
    let font = text_font();
    let scaled = font.as_scaled(PxScale::from(e.style.font_size));
    if e.style.fill.a() > 0 {
        let size = measure_text(&e.text, e.style.font_size);
        if let Some(r) = tiny_skia::Rect::from_xywh(
            e.a.x - TEXT_PADDING,
            e.a.y - TEXT_PADDING,
            size.x + 2.0 * TEXT_PADDING,
            size.y + 2.0 * TEXT_PADDING,
        ) {
            pm.fill_rect(r, &paint(e.style.fill), Transform::identity(), None);
        }
    }
    let (hs, vs) = (scaled.h_scale_factor(), scaled.v_scale_factor());
    let line_h = scaled.height() + scaled.line_gap();
    let mut pb = PathBuilder::new();
    for (i, line) in e.text.split('\n').enumerate() {
        let baseline = e.a.y + scaled.ascent() + line_h * i as f32;
        let mut pen = e.a.x;
        for c in line.chars() {
            let gid = font.glyph_id(c);
            if let Some(outline) = font.outline(gid) {
                let tx = |p: ab_glyph::Point| (pen + p.x * hs, baseline - p.y * vs);
                let mut last_end: Option<ab_glyph::Point> = None;
                for curve in &outline.curves {
                    let (start, end) = match curve {
                        OutlineCurve::Line(a, b) => (*a, *b),
                        OutlineCurve::Quad(a, _, c) => (*a, *c),
                        OutlineCurve::Cubic(a, _, _, d) => (*a, *d),
                    };
                    if last_end != Some(start) {
                        if last_end.is_some() {
                            pb.close();
                        }
                        let (x, y) = tx(start);
                        pb.move_to(x, y);
                    }
                    match curve {
                        OutlineCurve::Line(_, b) => {
                            let (x, y) = tx(*b);
                            pb.line_to(x, y);
                        }
                        OutlineCurve::Quad(_, b, c) => {
                            let ((x1, y1), (x, y)) = (tx(*b), tx(*c));
                            pb.quad_to(x1, y1, x, y);
                        }
                        OutlineCurve::Cubic(_, b, c, d) => {
                            let ((x1, y1), (x2, y2), (x, y)) = (tx(*b), tx(*c), tx(*d));
                            pb.cubic_to(x1, y1, x2, y2, x, y);
                        }
                    }
                    last_end = Some(end);
                }
                if last_end.is_some() {
                    pb.close();
                }
            }
            pen += scaled.h_advance(gid);
        }
    }
    if let Some(path) = pb.finish() {
        pm.fill_path(&path, &paint(e.style.stroke), FillRule::Winding, Transform::identity(), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Style;

    #[test]
    fn flatten_draws_rect_and_text() {
        let mut doc = Document::new(RgbaImage::from_pixel(100, 60, image::Rgba([255, 255, 255, 255])));
        let mut r = Element::new(Kind::Rect, Pos2::new(10.0, 10.0), Style::default());
        r.b = Pos2::new(50.0, 50.0);
        let mut t = Element::new(Kind::Text, Pos2::new(55.0, 5.0), Style::default());
        t.text = "Hi".into();
        doc.elements = vec![r, t];
        let out = flatten(&doc);
        assert_eq!(out.dimensions(), (100, 60));
        // Rectangle border is red, interior untouched.
        assert!(out.get_pixel(10, 30).0[1] < 100);
        assert_eq!(out.get_pixel(30, 30).0, [255, 255, 255, 255]);
        // Some text pixels were drawn.
        let inked = (55..100).flat_map(|x| (5..40).map(move |y| (x, y))).filter(|&(x, y)| out.get_pixel(x, y).0[1] < 128).count();
        assert!(inked > 10, "text not rendered");
    }

    #[test]
    fn measure_multiline() {
        let one = measure_text("abc", 20.0);
        let two = measure_text("abc\nabcdef", 20.0);
        assert!(two.x > one.x && (two.y - 2.0 * one.y).abs() < 0.01);
    }
}
