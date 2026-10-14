//! Animated GIF encoding (pure Rust, every platform).
//!
//! Screen recordings change little from frame to frame, so after the first
//! frame only the rectangle that changed is stored (the rest of the picture
//! stays from earlier frames), and frames where nothing changed just extend
//! how long the previous one is shown. That keeps GIFs a fraction of the size
//! of storing every frame in full.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use gif::{DisposalMethod, Encoder, Frame, Repeat};
use image::{imageops, RgbaImage};

use super::{VideoEncoder, VideoFrame};

/// Colour quantisation speed, 1 (best) to 30 (fastest).
const SPEED: i32 = 10;

struct Pending {
    image: RgbaImage,
    left: u32,
    top: u32,
    shown_at: Duration,
}

pub struct GifEncoder {
    encoder: Encoder<BufWriter<File>>,
    size: (u32, u32),
    /// The full picture as of the last frame stored.
    previous: Option<RgbaImage>,
    /// Each frame is written once the next change arrives, so we know how
    /// long to show it.
    pending: Option<Pending>,
    last_pts: Duration,
}

impl GifEncoder {
    pub fn create(path: &Path, width: u32, height: u32) -> Result<Self> {
        let file = BufWriter::new(File::create(path)?);
        let mut encoder = Encoder::new(file, width as u16, height as u16, &[])?;
        encoder.set_repeat(Repeat::Infinite)?;
        Ok(Self { encoder, size: (width, height), previous: None, pending: None, last_pts: Duration::ZERO })
    }

    fn write(&mut self, pending: Pending, until: Duration) -> Result<()> {
        let (w, h) = pending.image.dimensions();
        let mut pixels = pending.image.into_raw();
        let mut frame = Frame::from_rgba_speed(w as u16, h as u16, &mut pixels, SPEED);
        frame.left = pending.left as u16;
        frame.top = pending.top as u16;
        frame.dispose = DisposalMethod::Keep;
        // Hundredths of a second, at least 2 (browsers treat less as 10).
        frame.delay = (until.saturating_sub(pending.shown_at).as_millis() / 10).clamp(2, u16::MAX as u128) as u16;
        self.encoder.write_frame(&frame)?;
        Ok(())
    }
}

/// The smallest rectangle containing every pixel that differs, as
/// (left, top, width, height), or `None` if the images are identical.
fn changed_area(a: &RgbaImage, b: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (w, h) = a.dimensions();
    let row = w as usize * 4;
    let (ra, rb) = (a.as_raw(), b.as_raw());
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..h as usize {
        let (la, lb) = (&ra[y * row..][..row], &rb[y * row..][..row]);
        if la == lb {
            continue;
        }
        let first = la.as_chunks::<4>().0.iter().zip(lb.as_chunks::<4>().0.iter()).position(|(p, q)| p != q).unwrap_or(0) as u32;
        let last = w - 1 - la.as_chunks::<4>().0.iter().rev().zip(lb.as_chunks::<4>().0.iter().rev()).position(|(p, q)| p != q).unwrap_or(0) as u32;
        x0 = x0.min(first);
        x1 = x1.max(last);
        y0 = y0.min(y as u32);
        y1 = y as u32;
    }
    (x0 != u32::MAX).then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

impl VideoEncoder for GifEncoder {
    fn push(&mut self, frame: &VideoFrame) -> Result<()> {
        let Some(full) = RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone()) else { return Ok(()) };
        let img = if (frame.width, frame.height) == self.size {
            full
        } else {
            // `thumbnail` averages areas: fast and smooth for downscaling.
            imageops::thumbnail(&full, self.size.0, self.size.1)
        };
        self.last_pts = frame.pts;
        let next = match &self.previous {
            None => Pending { image: img.clone(), left: 0, top: 0, shown_at: frame.pts },
            Some(prev) => match changed_area(prev, &img) {
                // Nothing changed: the frame on screen just stays longer.
                None => return Ok(()),
                Some((x, y, w, h)) => {
                    Pending { image: imageops::crop_imm(&img, x, y, w, h).to_image(), left: x, top: y, shown_at: frame.pts }
                }
            },
        };
        if let Some(pending) = self.pending.take() {
            self.write(pending, frame.pts)?;
        }
        self.pending = Some(next);
        self.previous = Some(img);
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<()> {
        if let Some(pending) = self.pending.take() {
            // Show the last change until the recording ended (at least 0.1 s).
            let until = self.last_pts.max(pending.shown_at + Duration::from_millis(100));
            self.write(pending, until)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::AnimationDecoder;

    fn frame(w: u32, h: u32, pts_ms: u64, paint: impl Fn(u32, u32) -> [u8; 4]) -> VideoFrame {
        let rgba = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).flat_map(|(x, y)| paint(x, y)).collect();
        VideoFrame { width: w, height: h, rgba, pts: Duration::from_millis(pts_ms) }
    }

    #[test]
    fn changed_area_finds_the_bounding_box() {
        let a = RgbaImage::new(10, 8);
        let mut b = a.clone();
        b.put_pixel(2, 3, image::Rgba([1, 0, 0, 0]));
        b.put_pixel(6, 5, image::Rgba([1, 0, 0, 0]));
        assert_eq!(changed_area(&a, &b), Some((2, 3, 5, 3)));
        assert_eq!(changed_area(&a, &a), None);
    }

    #[test]
    fn stores_changes_only_and_merges_still_frames() {
        let path = std::env::temp_dir().join(format!("rustshot-gif-{}.gif", std::process::id()));
        let mut enc: Box<dyn VideoEncoder> = Box::new(GifEncoder::create(&path, 40, 20).unwrap());
        let blue = |_, _| [0, 0, 255, 255];
        // 0 ms: all blue. 100 and 200 ms: unchanged. 300 ms: a red square.
        for t in [0, 100, 200] {
            enc.push(&frame(40, 20, t, blue)).unwrap();
        }
        enc.push(&frame(40, 20, 300, |x, y| if (10..14).contains(&x) && (5..9).contains(&y) { [255, 0, 0, 255] } else { [0, 0, 255, 255] }))
            .unwrap();
        enc.finish().unwrap();

        let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(File::open(&path).unwrap())).unwrap();
        let frames = decoder.into_frames().collect_frames().unwrap();
        // The three identical frames became one, shown for 300 ms.
        assert_eq!(frames.len(), 2);
        let (n, d) = frames[0].delay().numer_denom_ms();
        assert_eq!(n / d, 300);
        // The decoded second frame is still full size with the change applied.
        let last = frames[1].buffer();
        assert_eq!(last.dimensions(), (40, 20));
        assert_eq!(last.get_pixel(11, 6).0[0], 255);
        assert_eq!(last.get_pixel(30, 15).0[2], 255);
        std::fs::remove_file(&path).unwrap();
    }
}
