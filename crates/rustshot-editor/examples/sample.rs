use egui::{Color32, Pos2};
use rustshot_editor::document::{Document, Element, HighlightMode, Kind, Style};
fn main() {
    let out = std::env::args().nth(1).expect("output path");
    // A checkered "screenshot" with some text-like stripes.
    let img = image::RgbaImage::from_fn(640, 400, |x, y| {
        let c = if (x / 40 + y / 40) % 2 == 0 { 235 } else { 210 };
        if y % 50 < 6 && x > 30 && x < 610 { image::Rgba([60, 90, 160, 255]) } else { image::Rgba([c, c, c, 255]) }
    });
    let mut doc = Document::new(img);
    let st = Style::default();
    let mut rect = Element::new(Kind::Rect, Pos2::new(30.0, 30.0), st);
    rect.b = Pos2::new(200.0, 120.0);
    let mut arrow = Element::new(Kind::Arrow, Pos2::new(240.0, 200.0), st);
    arrow.b = Pos2::new(330.0, 130.0);
    let mut bubble = Element::new(Kind::Speech, Pos2::new(360.0, 40.0), Style { fill: Color32::WHITE, stroke: Color32::from_rgb(40, 40, 40), font_size: 20.0, ..st });
    bubble.b = Pos2::new(600.0, 110.0);
    bubble.text = "Click here first".into();
    bubble.tail = Pos2::new(400.0, 170.0);
    bubble.fit_content();
    let c1 = Element::counter(Pos2::new(60.0, 180.0), st);
    let c2 = Element::counter(Pos2::new(120.0, 180.0), st);
    let c3 = Element::counter(Pos2::new(180.0, 180.0), Style { stroke: Color32::from_rgb(255, 220, 0), ..st });
    let mut spot = Element::new(Kind::Highlight, Pos2::new(40.0, 240.0), Style { highlight_mode: HighlightMode::Magnify, ..st });
    spot.b = Pos2::new(200.0, 330.0);
    let mut marker = Element::new(Kind::Highlight, Pos2::new(240.0, 245.0), st);
    marker.b = Pos2::new(600.0, 260.0);
    let mut ellipse = Element::new(Kind::Ellipse, Pos2::new(420.0, 280.0), Style { fill: Color32::from_rgba_unmultiplied(0, 160, 80, 120), ..st });
    ellipse.b = Pos2::new(600.0, 380.0);
    doc.elements = vec![spot, marker, rect, arrow, bubble, c1, c2, c3, ellipse];
    rustshot_editor::export::flatten(&doc).save(&out).unwrap();
}
