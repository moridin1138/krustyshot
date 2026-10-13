//! The editable document: a base image plus a stack of annotation elements,
//! with snapshot-based undo/redo.

use std::sync::Arc;

use egui::{Color32, Pos2, Rect, Vec2};
use image::RgbaImage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rect,
    Ellipse,
    Line,
    Arrow,
    Freehand,
    Text,
    /// Marker-style highlight (multiplies the area with the colour).
    Highlight,
    /// Pixelate or blur the area.
    Obfuscate,
}

impl Kind {
    /// Raster elements change pixels underneath them instead of being drawn on top.
    pub fn is_raster(self) -> bool {
        matches!(self, Kind::Highlight | Kind::Obfuscate)
    }

    pub fn is_line(self) -> bool {
        matches!(self, Kind::Line | Kind::Arrow)
    }

    pub fn has_fill(self) -> bool {
        matches!(self, Kind::Rect | Kind::Ellipse | Kind::Text)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObfuscateMode {
    Pixelate,
    Blur,
}

/// Visual properties shared by all element kinds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub stroke: Color32,
    pub width: f32,
    /// Transparent means "no fill". For text this is the background box.
    pub fill: Color32,
    pub font_size: f32,
    pub highlight: Color32,
    pub obfuscate: ObfuscateMode,
    /// Block size for pixelate, radius for blur (image pixels).
    pub strength: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            stroke: Color32::from_rgb(230, 30, 30),
            width: 3.0,
            fill: Color32::TRANSPARENT,
            font_size: 24.0,
            highlight: Color32::from_rgb(255, 235, 0),
            obfuscate: ObfuscateMode::Pixelate,
            strength: 12.0,
        }
    }
}

/// One annotation. Positions are in image pixel coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub kind: Kind,
    pub a: Pos2,
    pub b: Pos2,
    /// Freehand stroke points.
    pub points: Vec<Pos2>,
    pub text: String,
    pub style: Style,
}

impl Element {
    pub fn new(kind: Kind, at: Pos2, style: Style) -> Self {
        Self { kind, a: at, b: at, points: Vec::new(), text: String::new(), style }
    }

    pub fn bounds(&self) -> Rect {
        if self.kind == Kind::Freehand && !self.points.is_empty() {
            Rect::from_points(&self.points)
        } else {
            Rect::from_two_pos(self.a, self.b)
        }
    }

    pub fn translate(&mut self, d: Vec2) {
        self.a += d;
        self.b += d;
        for p in &mut self.points {
            *p += d;
        }
    }

    /// Is `p` on this element? `tol` is in image pixels.
    pub fn hit(&self, p: Pos2, tol: f32) -> bool {
        let tol = tol + self.style.width / 2.0;
        match self.kind {
            Kind::Line | Kind::Arrow => dist_to_segment(p, self.a, self.b) <= tol,
            Kind::Freehand => self
                .points
                .windows(2)
                .any(|w| dist_to_segment(p, w[0], w[1]) <= tol),
            Kind::Ellipse => {
                let r = self.bounds().expand(tol);
                let c = r.center();
                let (rx, ry) = (r.width() / 2.0, r.height() / 2.0);
                if rx <= 0.0 || ry <= 0.0 {
                    return false;
                }
                let (dx, dy) = ((p.x - c.x) / rx, (p.y - c.y) / ry);
                dx * dx + dy * dy <= 1.0
            }
            _ => self.bounds().expand(tol).contains(p),
        }
    }

    /// Resize handles: which of `a`/`b` each handle's x and y come from.
    pub fn handles(&self) -> Vec<Handle> {
        match self.kind {
            Kind::Line | Kind::Arrow => vec![Handle { x_from_b: false, y_from_b: false }, Handle { x_from_b: true, y_from_b: true }],
            Kind::Freehand | Kind::Text => Vec::new(),
            _ => vec![
                Handle { x_from_b: false, y_from_b: false },
                Handle { x_from_b: true, y_from_b: false },
                Handle { x_from_b: false, y_from_b: true },
                Handle { x_from_b: true, y_from_b: true },
            ],
        }
    }

    pub fn handle_pos(&self, h: Handle) -> Pos2 {
        Pos2::new(
            if h.x_from_b { self.b.x } else { self.a.x },
            if h.y_from_b { self.b.y } else { self.a.y },
        )
    }

    pub fn set_handle(&mut self, h: Handle, p: Pos2) {
        if h.x_from_b { self.b.x = p.x } else { self.a.x = p.x }
        if h.y_from_b { self.b.y = p.y } else { self.a.y = p.y }
    }

    /// Too small to keep after creation (e.g. a click without a drag).
    pub fn is_degenerate(&self) -> bool {
        match self.kind {
            Kind::Freehand => self.points.len() < 2,
            Kind::Text => self.text.trim().is_empty(),
            Kind::Line | Kind::Arrow => self.a.distance(self.b) < 3.0,
            _ => {
                let r = self.bounds();
                r.width() < 3.0 || r.height() < 3.0
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    pub x_from_b: bool,
    pub y_from_b: bool,
}

pub fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 == 0.0 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

#[derive(Clone)]
pub struct Document {
    pub base: Arc<RgbaImage>,
    pub elements: Vec<Element>,
}

impl Document {
    pub fn new(image: RgbaImage) -> Self {
        Self { base: Arc::new(image), elements: Vec::new() }
    }

    pub fn size(&self) -> Vec2 {
        Vec2::new(self.base.width() as f32, self.base.height() as f32)
    }

    /// Index of the top-most element under `p`.
    pub fn hit(&self, p: Pos2, tol: f32) -> Option<usize> {
        self.elements.iter().rposition(|e| e.hit(p, tol))
    }

    /// Crop the base image to `rect` (image pixels) and shift all elements.
    pub fn crop(&mut self, rect: Rect) -> bool {
        let r = rect.intersect(Rect::from_min_size(Pos2::ZERO, self.size()));
        let r = Rect::from_min_max(r.min.round(), r.max.round());
        if r.width() < 1.0 || r.height() < 1.0 {
            return false;
        }
        let cropped = image::imageops::crop_imm(
            self.base.as_ref(),
            r.min.x as u32,
            r.min.y as u32,
            r.width() as u32,
            r.height() as u32,
        )
        .to_image();
        self.base = Arc::new(cropped);
        let d = -r.min.to_vec2();
        for e in &mut self.elements {
            e.translate(d);
        }
        true
    }
}

/// Snapshot-based undo history. Snapshots are cheap because the base image
/// is shared through an `Arc`.
#[derive(Default)]
pub struct History {
    undo: Vec<Document>,
    redo: Vec<Document>,
}

impl History {
    const LIMIT: usize = 100;

    /// Record `doc` as the state to return to on undo.
    pub fn push(&mut self, doc: &Document) {
        self.undo.push(doc.clone());
        if self.undo.len() > Self::LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, doc: &mut Document) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(doc, prev));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self, doc: &mut Document) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(doc, next));
                true
            }
            None => false,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_el(a: (f32, f32), b: (f32, f32)) -> Element {
        let mut e = Element::new(Kind::Rect, Pos2::new(a.0, a.1), Style::default());
        e.b = Pos2::new(b.0, b.1);
        e
    }

    #[test]
    fn hit_line_and_rect() {
        let mut line = Element::new(Kind::Line, Pos2::new(0.0, 0.0), Style::default());
        line.b = Pos2::new(100.0, 0.0);
        assert!(line.hit(Pos2::new(50.0, 2.0), 2.0));
        assert!(!line.hit(Pos2::new(50.0, 20.0), 2.0));
        assert!(rect_el((10.0, 10.0), (20.0, 20.0)).hit(Pos2::new(15.0, 15.0), 0.0));
    }

    #[test]
    fn topmost_hit_wins() {
        let mut doc = Document::new(RgbaImage::new(100, 100));
        doc.elements.push(rect_el((0.0, 0.0), (50.0, 50.0)));
        doc.elements.push(rect_el((10.0, 10.0), (30.0, 30.0)));
        assert_eq!(doc.hit(Pos2::new(20.0, 20.0), 0.0), Some(1));
        assert_eq!(doc.hit(Pos2::new(45.0, 45.0), 0.0), Some(0));
        assert_eq!(doc.hit(Pos2::new(90.0, 90.0), 0.0), None);
    }

    #[test]
    fn undo_redo() {
        let mut doc = Document::new(RgbaImage::new(10, 10));
        let mut h = History::default();
        h.push(&doc);
        doc.elements.push(rect_el((0.0, 0.0), (5.0, 5.0)));
        assert!(h.undo(&mut doc));
        assert!(doc.elements.is_empty());
        assert!(h.redo(&mut doc));
        assert_eq!(doc.elements.len(), 1);
        assert!(!h.redo(&mut doc));
    }

    #[test]
    fn crop_shifts_elements() {
        let mut doc = Document::new(RgbaImage::new(100, 80));
        doc.elements.push(rect_el((20.0, 20.0), (40.0, 40.0)));
        assert!(doc.crop(Rect::from_min_max(Pos2::new(10.0, 5.0), Pos2::new(60.0, 200.0))));
        assert_eq!((doc.base.width(), doc.base.height()), (50, 75));
        assert_eq!(doc.elements[0].a, Pos2::new(10.0, 15.0));
    }
}
