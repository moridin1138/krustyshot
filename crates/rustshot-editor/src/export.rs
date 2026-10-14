//! Flatten a document into a single image: raster effects first, then
//! vector annotations rendered with tiny-skia, then text via ab_glyph.

use ab_glyph::{Font, FontRef, OutlineCurve, PxScale, ScaleFont};
use egui::{Color32, Pos2};
use image::RgbaImage;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::document::{Document, Element, Kind, SPEECH_PADDING};
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
    for (i, e) in doc.elements.iter().enumerate() {
        let number = (e.kind == Kind::Counter).then(|| doc.counter_number(i));
        if e.style.shadow && e.kind.has_shadow() {
            draw_shadow(&mut pixmap, e, number);
        }
        draw_element(&mut pixmap, e, number, Ink::normal());
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

/// How to draw: where (transform), and optionally everything in one colour
/// (used to draw an element's silhouette for its shadow).
#[derive(Clone, Copy)]
pub struct Ink {
    pub transform: Transform,
    pub mono: Option<Color32>,
}

impl Ink {
    pub fn normal() -> Ink {
        Ink { transform: Transform::identity(), mono: None }
    }

    fn paint(&self, c: Color32) -> Paint<'static> {
        let c = match self.mono {
            Some(m) if c.a() > 0 => m,
            _ => c,
        };
        let mut p = Paint::default();
        let [r, g, b, a] = c.to_srgba_unmultiplied();
        p.set_color_rgba8(r, g, b, a);
        p.anti_alias = true;
        p
    }
}

fn stroke(width: f32) -> Stroke {
    Stroke { width: width.max(0.5), line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() }
}

fn ts_rect(r: egui::Rect) -> Option<tiny_skia::Rect> {
    tiny_skia::Rect::from_ltrb(r.min.x, r.min.y, r.max.x, r.max.y)
}

fn polygon(points: &[Pos2]) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let (first, rest) = points.split_first()?;
    pb.move_to(first.x, first.y);
    for p in rest {
        pb.line_to(p.x, p.y);
    }
    pb.close();
    pb.finish()
}

fn rounded_rect(r: egui::Rect, radius: f32) -> Option<tiny_skia::Path> {
    let rad = radius.min(r.width() / 2.0).min(r.height() / 2.0).max(0.0);
    let k = rad * 0.552_284_8;
    let (x0, y0, x1, y1) = (r.min.x, r.min.y, r.max.x, r.max.y);
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + rad, y0);
    pb.line_to(x1 - rad, y0);
    pb.cubic_to(x1 - rad + k, y0, x1, y0 + rad - k, x1, y0 + rad);
    pb.line_to(x1, y1 - rad);
    pb.cubic_to(x1, y1 - rad + k, x1 - rad + k, y1, x1 - rad, y1);
    pb.line_to(x0 + rad, y1);
    pb.cubic_to(x0 + rad - k, y1, x0, y1 - rad + k, x0, y1 - rad);
    pb.line_to(x0, y0 + rad);
    pb.cubic_to(x0, y0 + rad - k, x0 + rad - k, y0, x0 + rad, y0);
    pb.close();
    pb.finish()
}

/// Corner radius of a speech bubble.
pub fn speech_radius(r: egui::Rect) -> f32 {
    14.0_f32.min(r.width() / 4.0).min(r.height() / 4.0)
}

/// Where a speech bubble's tail meets the box: two points on the edge
/// facing the tail tip. Equal points mean there is no visible tail (the tip
/// is inside the box).
pub fn speech_tail_base(r: egui::Rect, tip: Pos2) -> [Pos2; 2] {
    if r.contains(tip) || r.width() <= 0.0 || r.height() <= 0.0 {
        let c = r.center();
        return [c, c];
    }
    let c = r.center();
    let d = tip - c;
    let half_base = (r.width().min(r.height()) * 0.15).clamp(4.0, 24.0);
    let margin = speech_radius(r) + half_base;
    let along = |v: f32, lo: f32, hi: f32| if lo <= hi { v.clamp(lo, hi) } else { (lo + hi) / 2.0 };
    if d.x.abs() / r.width() > d.y.abs() / r.height() {
        let x = if d.x > 0.0 { r.max.x } else { r.min.x };
        let y = along(tip.y, r.min.y + margin, r.max.y - margin);
        [Pos2::new(x, y - half_base), Pos2::new(x, y + half_base)]
    } else {
        let y = if d.y > 0.0 { r.max.y } else { r.min.y };
        let x = along(tip.x, r.min.x + margin, r.max.x - margin);
        [Pos2::new(x - half_base, y), Pos2::new(x + half_base, y)]
    }
}

/// Text colour that stays readable on `bg`.
pub fn contrasting_text(bg: Color32) -> Color32 {
    let [r, g, b, _] = bg.to_srgba_unmultiplied();
    let luma = 0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32;
    if luma > 150.0 { Color32::BLACK } else { Color32::WHITE }
}

/// Everything an element covers, including a speech bubble's tail.
pub fn visual_bounds(e: &Element) -> egui::Rect {
    let r = e.bounds();
    let r = if e.kind == Kind::Speech { r.union(egui::Rect::from_min_max(e.tail, e.tail)) } else { r };
    r.expand(e.style.width / 2.0 + 1.0)
}

const SHADOW_OFFSET: f32 = 4.0;
const SHADOW_BLUR: f32 = 3.0;

/// A soft shadow: the element's silhouette, blurred and offset.
fn draw_shadow(pm: &mut Pixmap, e: &Element, number: Option<usize>) {
    let bb = visual_bounds(e).expand(SHADOW_BLUR * 3.0);
    let (w, h) = (bb.width().ceil() as u32, bb.height().ceil() as u32);
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return;
    }
    let Some(mut layer) = Pixmap::new(w, h) else { return };
    let ink = Ink { transform: Transform::from_translate(-bb.min.x, -bb.min.y), mono: Some(Color32::BLACK) };
    draw_element(&mut layer, e, number, ink);
    // Blur the (premultiplied) silhouette; all colour channels are zero.
    let Some(img) = RgbaImage::from_raw(w, h, layer.take()) else { return };
    let blurred = image::imageops::fast_blur(&img, SHADOW_BLUR);
    let Some(layer) = Pixmap::from_vec(blurred.into_raw(), tiny_skia::IntSize::from_wh(w, h).unwrap()) else {
        return;
    };
    let paint = tiny_skia::PixmapPaint { opacity: 0.45, ..Default::default() };
    pm.draw_pixmap(
        (bb.min.x + SHADOW_OFFSET).round() as i32,
        (bb.min.y + SHADOW_OFFSET).round() as i32,
        layer.as_ref(),
        &paint,
        Transform::identity(),
        None,
    );
}

/// Draw one annotation. `number` is the step-counter number, if any.
pub fn draw_element(pm: &mut Pixmap, e: &Element, number: Option<usize>, ink: Ink) {
    let s = &e.style;
    let ts = ink.transform;
    match e.kind {
        Kind::Rect | Kind::Ellipse => {
            let Some(r) = ts_rect(e.bounds()) else { return };
            let path = if e.kind == Kind::Rect { Some(PathBuilder::from_rect(r)) } else { PathBuilder::from_oval(r) };
            let Some(path) = path else { return };
            if s.fill.a() > 0 {
                pm.fill_path(&path, &ink.paint(s.fill), FillRule::Winding, ts, None);
            }
            if s.width > 0.0 {
                pm.stroke_path(&path, &ink.paint(s.stroke), &stroke(s.width), ts, None);
            }
        }
        Kind::Line | Kind::Arrow => {
            let end = if e.kind == Kind::Arrow {
                let (head, shaft_end) = arrow_head(e.a, e.b, s.width);
                if let Some(p) = polygon(&head) {
                    pm.fill_path(&p, &ink.paint(s.stroke), FillRule::Winding, ts, None);
                }
                shaft_end
            } else {
                e.b
            };
            let mut pb = PathBuilder::new();
            pb.move_to(e.a.x, e.a.y);
            pb.line_to(end.x, end.y);
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &ink.paint(s.stroke), &stroke(s.width), ts, None);
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
                pm.stroke_path(&p, &ink.paint(s.stroke), &stroke(s.width), ts, None);
            }
        }
        Kind::Text => {
            if s.fill.a() > 0 {
                let size = measure_text(&e.text, s.font_size);
                if let Some(r) = tiny_skia::Rect::from_xywh(
                    e.a.x - TEXT_PADDING,
                    e.a.y - TEXT_PADDING,
                    size.x + 2.0 * TEXT_PADDING,
                    size.y + 2.0 * TEXT_PADDING,
                ) {
                    pm.fill_rect(r, &ink.paint(s.fill), ts, None);
                }
            }
            fill_text(pm, &e.text, e.a, s.font_size, &ink.paint(s.stroke), ts);
        }
        Kind::Speech => {
            let r = e.bounds();
            let base = speech_tail_base(r, e.tail);
            let body = rounded_rect(r, speech_radius(r));
            let tail = (base[0] != base[1]).then(|| polygon(&[base[0], e.tail, base[1]])).flatten();
            let shapes: Vec<&tiny_skia::Path> = body.iter().chain(tail.iter()).collect();
            if s.fill.a() > 0 {
                // Stroke at double width, then fill over it: the outer half of
                // the stroke remains as the outline, and the line where the
                // tail joins the box disappears under the fill.
                if s.width > 0.0 {
                    for p in &shapes {
                        pm.stroke_path(p, &ink.paint(s.stroke), &stroke(s.width * 2.0), ts, None);
                    }
                }
                for p in &shapes {
                    pm.fill_path(p, &ink.paint(s.fill), FillRule::Winding, ts, None);
                }
            } else if s.width > 0.0 {
                for p in &shapes {
                    pm.stroke_path(p, &ink.paint(s.stroke), &stroke(s.width), ts, None);
                }
            }
            let origin = r.min + egui::Vec2::splat(SPEECH_PADDING);
            fill_text(pm, &e.text, origin, s.font_size, &ink.paint(s.stroke), ts);
        }
        Kind::Counter => {
            let r = e.bounds();
            let c = r.center();
            let mut pb = PathBuilder::new();
            pb.push_circle(c.x, c.y, r.width() / 2.0);
            if let Some(p) = pb.finish() {
                pm.fill_path(&p, &ink.paint(s.stroke), FillRule::Winding, ts, None);
            }
            let label = number.unwrap_or(1).to_string();
            let size = measure_text(&label, s.font_size);
            let origin = c - size / 2.0;
            fill_text(pm, &label, origin, s.font_size, &ink.paint(contrasting_text(s.stroke)), ts);
        }
        Kind::Highlight | Kind::Obfuscate => {}
    }
}

/// Glyph outlines for `text` with its top-left at `origin`.
pub fn text_path(text: &str, origin: Pos2, size: f32) -> Option<tiny_skia::Path> {
    let font = text_font();
    let scaled = font.as_scaled(PxScale::from(size));
    let (hs, vs) = (scaled.h_scale_factor(), scaled.v_scale_factor());
    let line_h = scaled.height() + scaled.line_gap();
    let mut pb = PathBuilder::new();
    for (i, line) in text.split('\n').enumerate() {
        let baseline = origin.y + scaled.ascent() + line_h * i as f32;
        let mut pen = origin.x;
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
    pb.finish()
}

fn fill_text(pm: &mut Pixmap, text: &str, origin: Pos2, size: f32, paint: &Paint, ts: Transform) {
    if let Some(path) = text_path(text, origin, size) {
        pm.fill_path(&path, paint, FillRule::Winding, ts, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Style;

    #[test]
    fn flatten_draws_rect_and_text() {
        let mut doc = Document::new(RgbaImage::from_pixel(100, 60, image::Rgba([255, 255, 255, 255])));
        let mut r = Element::new(Kind::Rect, Pos2::new(10.0, 10.0), Style { shadow: false, ..Style::default() });
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

    fn white(w: u32, h: u32) -> Document {
        Document::new(RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, 255])))
    }

    #[test]
    fn speech_bubble_has_fill_outline_text_and_tail() {
        let mut doc = white(200, 160);
        let mut e = Element::new(Kind::Speech, Pos2::new(20.0, 20.0), Style { shadow: false, fill: Color32::from_rgb(0, 0, 255), ..Style::default() });
        e.b = Pos2::new(180.0, 80.0);
        e.tail = Pos2::new(60.0, 140.0);
        e.text = "Hi".into();
        doc.elements.push(e);
        let out = flatten(&doc);
        // Interior is the fill colour.
        assert_eq!(out.get_pixel(150, 60).0, [0, 0, 255, 255]);
        // Inside the tail, just below the box: also fill (no line across the join).
        assert_eq!(out.get_pixel(60, 84).0, [0, 0, 255, 255]);
        // The tail tip region is drawn; far away is untouched.
        assert_ne!(out.get_pixel(60, 136).0, [255, 255, 255, 255]);
        assert_eq!(out.get_pixel(190, 150).0, [255, 255, 255, 255]);
    }

    #[test]
    fn tail_base_sits_on_the_edge_facing_the_tip() {
        let r = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 50.0));
        let below = speech_tail_base(r, Pos2::new(30.0, 120.0));
        assert!(below.iter().all(|p| p.y == 50.0));
        let right = speech_tail_base(r, Pos2::new(300.0, 25.0));
        assert!(right.iter().all(|p| p.x == 100.0));
        let inside = speech_tail_base(r, Pos2::new(50.0, 25.0));
        assert_eq!(inside[0], inside[1]);
    }

    #[test]
    fn counter_draws_number_in_contrasting_colour() {
        let mut doc = white(100, 100);
        doc.elements.push(Element::counter(Pos2::new(50.0, 50.0), Style { shadow: false, ..Style::default() }));
        let out = flatten(&doc);
        let red = Style::default().stroke.to_srgba_unmultiplied();
        // Circle edge is the counter colour; the number adds white pixels inside.
        assert_eq!(out.get_pixel(50, 33).0, red);
        // Thin, anti-aliased glyph: count clearly light pixels inside the circle.
        let inner_light = (38..62).flat_map(|x| (36..64).map(move |y| (x, y))).filter(|&(x, y)| out.get_pixel(x, y).0[1] > 150).count();
        assert!(inner_light > 5, "number not drawn ({inner_light} light pixels)");
    }

    #[test]
    fn shadow_darkens_below_right_only_when_enabled() {
        let make = |shadow| {
            let mut doc = white(120, 120);
            let mut r = Element::new(Kind::Rect, Pos2::new(20.0, 20.0), Style { shadow, fill: Color32::RED, ..Style::default() });
            r.b = Pos2::new(80.0, 80.0);
            doc.elements.push(r);
            flatten(&doc)
        };
        let with = make(true);
        let without = make(false);
        // Just outside the bottom-right corner the shadow shows.
        assert!(with.get_pixel(84, 84).0[0] < 250);
        assert_eq!(without.get_pixel(84, 84).0, [255, 255, 255, 255]);
        // Above-left of the shape there is no shadow.
        assert_eq!(with.get_pixel(12, 12).0, [255, 255, 255, 255]);
    }

    #[test]
    fn measure_multiline() {
        let one = measure_text("abc", 20.0);
        let two = measure_text("abc\nabcdef", 20.0);
        assert!(two.x > one.x && (two.y - 2.0 * one.y).abs() < 0.01);
    }
}
