//! Project maintenance tasks: `cargo xtask <task>`.
//!
//! - `icons`: render the app icon (PNG sizes for bundles, plus a Windows .ico)
//!   into `assets/icons/`. Re-run after changing the design below.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("icons") => icons(),
        _ => bail!("usage: cargo xtask icons"),
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

const SIZES: [u32; 7] = [16, 32, 64, 128, 256, 512, 1024];

fn icons() -> Result<()> {
    let dir = root().join("assets/icons");
    std::fs::create_dir_all(&dir)?;
    for size in SIZES {
        let path = dir.join(format!("icon_{size}.png"));
        render(size).save_png(&path).with_context(|| format!("writing {}", path.display()))?;
        println!("wrote {}", path.display());
    }
    // Windows icon with the sizes Explorer uses.
    let ico = dir.join("rustshot.ico");
    let file = std::fs::File::create(&ico)?;
    let frames = [16u32, 32, 64, 128, 256]
        .iter()
        .map(|&s| {
            let px = render(s);
            let img = image::RgbaImage::from_raw(s, s, px.clone().take_demultiplied()).unwrap();
            image::codecs::ico::IcoFrame::as_png(img.as_raw(), s, s, image::ExtendedColorType::Rgba8)
        })
        .collect::<Result<Vec<_>, _>>()?;
    image::codecs::ico::IcoEncoder::new(file).encode_images(&frames)?;
    println!("wrote {}", ico.display());
    Ok(())
}

/// The menu-bar glyph (selection corners + crosshair) in white on a blue
/// rounded tile, laid out on Apple's 1024 grid (824 px tile, 100 px margin).
fn render(size: u32) -> Pixmap {
    let mut pm = Pixmap::new(size, size).unwrap();
    let s = size as f32 / 1024.0;
    let t = Transform::from_scale(s, s);

    let mut blue = Paint::default();
    blue.set_color(Color::from_rgba8(0, 140, 230, 255));
    blue.anti_alias = true;
    let tile = rounded_rect(100.0, 100.0, 824.0, 824.0, 185.0);
    pm.fill_path(&tile, &blue, FillRule::Winding, t, None);

    let mut white = Paint::default();
    white.set_color(Color::WHITE);
    white.anti_alias = true;
    // Corner brackets.
    let (lo, hi, arm, w) = (250.0, 774.0, 190.0, 64.0);
    for (x, y, dx, dy) in [(lo, lo, 1.0, 1.0), (hi, lo, -1.0, 1.0), (lo, hi, 1.0, -1.0), (hi, hi, -1.0, -1.0)] {
        let horiz = rect_from(x, y, x + dx * arm, y + dy * w);
        let vert = rect_from(x, y, x + dx * w, y + dy * arm);
        pm.fill_rect(horiz, &white, t, None);
        pm.fill_rect(vert, &white, t, None);
    }
    // Crosshair.
    let (c, len, cw) = (512.0, 150.0, 48.0);
    pm.fill_rect(rect_from(c - len, c - cw / 2.0, c + len, c + cw / 2.0), &white, t, None);
    pm.fill_rect(rect_from(c - cw / 2.0, c - len, c + cw / 2.0, c + len), &white, t, None);
    pm
}

fn rect_from(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::from_ltrb(x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)).unwrap()
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> tiny_skia::Path {
    // Cubic approximation of quarter circles.
    let k = r * 0.552_284_8;
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish().unwrap()
}
