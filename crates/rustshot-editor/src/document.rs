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
    /// Rounded box with text and a tail pointing at something.
    Speech,
    /// Numbered circle; numbers follow the order counters were added.
    Counter,
    /// Highlight an area (see [`HighlightMode`]).
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
        matches!(self, Kind::Rect | Kind::Ellipse | Kind::Text | Kind::Speech)
    }

    /// Kinds whose content is text typed by the user.
    pub fn has_text(self) -> bool {
        matches!(self, Kind::Text | Kind::Speech)
    }

    /// Kinds that can cast a drop shadow.
    pub fn has_shadow(self) -> bool {
        !self.is_raster() && self != Kind::Text
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObfuscateMode {
    Pixelate,
    Blur,
}

/// The four highlighter styles from Greenshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HighlightMode {
    /// Marker pen: multiply the area with the colour.
    Text,
    /// Darken everything outside the area.
    Area,
    /// Turn everything outside the area grey.
    Grayscale,
    /// Show the middle of the area magnified.
    Magnify,
}

impl HighlightMode {
    pub const ALL: [HighlightMode; 4] =
        [HighlightMode::Text, HighlightMode::Area, HighlightMode::Grayscale, HighlightMode::Magnify];

    pub fn label(self) -> &'static str {
        match self {
            HighlightMode::Text => "Marker",
            HighlightMode::Area => "Spotlight",
            HighlightMode::Grayscale => "Grey out the rest",
            HighlightMode::Magnify => "Magnify",
        }
    }
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
    pub highlight_mode: HighlightMode,
    pub obfuscate: ObfuscateMode,
    /// Block size for pixelate, radius for blur (image pixels).
    pub strength: f32,
    /// Soft drop shadow under shapes.
    pub shadow: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            stroke: Color32::from_rgb(230, 30, 30),
            width: 3.0,
            fill: Color32::TRANSPARENT,
            font_size: 24.0,
            highlight: Color32::from_rgb(255, 235, 0),
            highlight_mode: HighlightMode::Text,
            obfuscate: ObfuscateMode::Pixelate,
            strength: 12.0,
            shadow: true,
        }
    }
}

/// Space between a speech bubble's edge and its text (image pixels).
pub const SPEECH_PADDING: f32 = 12.0;

/// One annotation. Positions are in image pixel coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub kind: Kind,
    pub a: Pos2,
    pub b: Pos2,
    /// Speech bubble: where the tail points.
    pub tail: Pos2,
    /// Freehand stroke points.
    pub points: Vec<Pos2>,
    pub text: String,
    pub style: Style,
}

impl Element {
    pub fn new(kind: Kind, at: Pos2, style: Style) -> Self {
        Self { kind, a: at, b: at, tail: at, points: Vec::new(), text: String::new(), style }
    }

    /// A step counter centred on `center`.
    pub fn counter(center: Pos2, style: Style) -> Self {
        let mut e = Self::new(Kind::Counter, center, style);
        e.a = center;
        e.fit_content();
        e
    }

    /// Diameter of a step counter for its font size.
    pub fn counter_diameter(&self) -> f32 {
        (self.style.font_size * 1.6).max(16.0)
    }

    /// Keep size-derived geometry in step with the content: text boxes fit
    /// their text, counters follow the font size, speech bubbles grow to
    /// fit their text.
    pub fn fit_content(&mut self) {
        match self.kind {
            Kind::Text => self.b = self.a + crate::export::measure_text(&self.text, self.style.font_size),
            Kind::Counter => {
                let c = self.bounds().center();
                let r = self.counter_diameter() / 2.0;
                self.a = c - Vec2::splat(r);
                self.b = c + Vec2::splat(r);
            }
            Kind::Speech => {
                let r = self.bounds();
                let need = crate::export::measure_text(&self.text, self.style.font_size) + Vec2::splat(2.0 * SPEECH_PADDING);
                let (a, b) = (r.min, r.max.max(r.min + need));
                self.a = a;
                self.b = b;
            }
            _ => {}
        }
    }

    /// Default tail position for a new speech bubble: below its left side.
    pub fn default_tail(&mut self) {
        let r = self.bounds();
        self.tail = Pos2::new(r.min.x + r.width() * 0.25, r.max.y + r.height() * 0.6 + 20.0);
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
        self.tail += d;
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
            Kind::Ellipse | Kind::Counter => {
                let r = self.bounds().expand(tol);
                let c = r.center();
                let (rx, ry) = (r.width() / 2.0, r.height() / 2.0);
                if rx <= 0.0 || ry <= 0.0 {
                    return false;
                }
                let (dx, dy) = ((p.x - c.x) / rx, (p.y - c.y) / ry);
                dx * dx + dy * dy <= 1.0
            }
            Kind::Speech => {
                self.bounds().expand(tol).contains(p) || {
                    let base = crate::export::speech_tail_base(self.bounds(), self.tail);
                    dist_to_segment(p, base[0], self.tail) <= tol + 4.0
                        || dist_to_segment(p, base[1], self.tail) <= tol + 4.0
                }
            }
            _ => self.bounds().expand(tol).contains(p),
        }
    }

    /// Handles the user can drag on the selected element.
    pub fn handles(&self) -> Vec<Handle> {
        let corner = |x_from_b, y_from_b| Handle::Corner { x_from_b, y_from_b };
        match self.kind {
            Kind::Line | Kind::Arrow => vec![corner(false, false), corner(true, true)],
            Kind::Freehand | Kind::Text | Kind::Counter => Vec::new(),
            Kind::Speech => vec![
                corner(false, false),
                corner(true, false),
                corner(false, true),
                corner(true, true),
                Handle::Tail,
            ],
            _ => vec![corner(false, false), corner(true, false), corner(false, true), corner(true, true)],
        }
    }

    pub fn handle_pos(&self, h: Handle) -> Pos2 {
        match h {
            Handle::Corner { x_from_b, y_from_b } => Pos2::new(
                if x_from_b { self.b.x } else { self.a.x },
                if y_from_b { self.b.y } else { self.a.y },
            ),
            Handle::Tail => self.tail,
        }
    }

    pub fn set_handle(&mut self, h: Handle, p: Pos2) {
        match h {
            Handle::Corner { x_from_b, y_from_b } => {
                if x_from_b { self.b.x = p.x } else { self.a.x = p.x }
                if y_from_b { self.b.y = p.y } else { self.a.y = p.y }
            }
            Handle::Tail => self.tail = p,
        }
    }

    /// Too small to keep after creation (e.g. a click without a drag).
    pub fn is_degenerate(&self) -> bool {
        match self.kind {
            Kind::Freehand => self.points.len() < 2,
            Kind::Text => self.text.trim().is_empty(),
            Kind::Counter => false,
            Kind::Line | Kind::Arrow => self.a.distance(self.b) < 3.0,
            _ => {
                let r = self.bounds();
                r.width() < 3.0 || r.height() < 3.0
            }
        }
    }
}

/// Something the user can drag to reshape the selected element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    /// A corner or line end: which of `a`/`b` its x and y come from.
    Corner { x_from_b: bool, y_from_b: bool },
    /// The tip of a speech bubble's tail.
    Tail,
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

    /// The number shown on the step counter at `idx` (1-based, in the order
    /// counters appear in the document, so deleting one renumbers the rest).
    pub fn counter_number(&self, idx: usize) -> usize {
        self.elements[..idx].iter().filter(|e| e.kind == Kind::Counter).count() + 1
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
    fn counters_number_in_order_and_renumber() {
        let mut doc = Document::new(RgbaImage::new(100, 100));
        doc.elements.push(Element::counter(Pos2::new(10.0, 10.0), Style::default()));
        doc.elements.push(rect_el((0.0, 0.0), (5.0, 5.0)));
        doc.elements.push(Element::counter(Pos2::new(50.0, 50.0), Style::default()));
        doc.elements.push(Element::counter(Pos2::new(80.0, 80.0), Style::default()));
        assert_eq!(doc.counter_number(3), 3);
        doc.elements.remove(0);
        assert_eq!(doc.counter_number(1), 1);
        assert_eq!(doc.counter_number(2), 2);
    }

    #[test]
    fn counter_follows_font_size_around_its_centre() {
        let mut c = Element::counter(Pos2::new(40.0, 40.0), Style::default());
        c.style.font_size = 50.0;
        c.fit_content();
        assert_eq!(c.bounds().center(), Pos2::new(40.0, 40.0));
        assert_eq!(c.bounds().width(), 80.0);
    }

    #[test]
    fn speech_tail_handle_moves_tail_and_hit_includes_it() {
        let mut e = Element::new(Kind::Speech, Pos2::new(0.0, 0.0), Style::default());
        e.b = Pos2::new(100.0, 50.0);
        e.default_tail();
        assert!(e.handles().contains(&Handle::Tail));
        e.set_handle(Handle::Tail, Pos2::new(50.0, 150.0));
        assert_eq!(e.tail, Pos2::new(50.0, 150.0));
        assert!(e.hit(Pos2::new(50.0, 148.0), 2.0), "tail tip is clickable");
        e.translate(Vec2::new(10.0, 0.0));
        assert_eq!(e.tail, Pos2::new(60.0, 150.0));
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
