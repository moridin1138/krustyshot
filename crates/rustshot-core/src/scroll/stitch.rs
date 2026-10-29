//! Joining the screenshots of a scrolling area into one tall image.
//!
//! Each new frame is compared with the previous one. First, the columns
//! that changed show which part scrolled: a selection often includes things
//! beside the page that stay put (a sidebar, another window). Rows are then
//! compared across just those columns. Rows that stay put at the top and
//! bottom are a fixed header and footer (a site's menu bar, a status bar)
//! and are kept once. In the rows between, the frame has moved up by some
//! number of pixels; the rows that scrolled into view are added. Rows of a
//! single colour say nothing about the position, so they don't count when
//! matching. If what scrolled is much narrower than the selection, the
//! finished image is trimmed to it.

use image::RgbaImage;

/// What adding a frame did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The view moved down by this many pixels and the new rows were added.
    Moved(u32),
    /// Nothing moved: the bottom of the page (or a stuck scroll).
    Unchanged,
    /// The frames don't line up (the view jumped, or the content changed).
    Lost,
}

/// Matching needs this share of the overlapping, non-blank rows to agree.
const MIN_AGREEMENT: f64 = 0.8;
/// At exactly the distance the page was asked to scroll, less will do.
const MIN_AGREEMENT_EXPECTED: f64 = 0.6;
/// Or, below both: a shift with at least this much agreement that beats
/// every other candidate by [`CLEAR_MARGIN`]. A page's last scroll stops
/// short at the bottom, and some of its content may have changed.
const MIN_AGREEMENT_CLEAR: f64 = 0.6;
const CLEAR_MARGIN: f64 = 0.2;
/// And at least this many of them.
const MIN_EVIDENCE: usize = 6;

pub struct Stitcher {
    width: u32,
    height: u32,
    /// The image so far, without the footer: packed RGBA rows.
    body: Vec<u8>,
    prev: RgbaImage,
    /// Rows at the bottom of the frame that stay put (shown once, at the
    /// end). Decided at the first step and then kept, so the body and the
    /// footer never overlap.
    footer: u32,
    footer_known: bool,
    /// The columns that scrolled, over all steps (first, last).
    scrolled: Option<(u32, u32)>,
    /// Why the last step came out as it did, for the log.
    pub detail: String,
}

/// A row's brightness profile: the compared columns in [`BINS`] groups,
/// averaged. Browsers redraw text slightly differently as it scrolls, so
/// rows are compared with a tolerance ([`ROW_TOLERANCE`]), not exactly.
#[derive(Clone, Copy)]
struct RowInfo {
    profile: [u8; BINS],
    /// Nothing but one colour (a blank line or margin).
    blank: bool,
}

const BINS: usize = 96;
/// Rows whose profiles differ by at most this much on average (out of 255)
/// count as the same.
const ROW_TOLERANCE: u32 = 3;
/// A pixel counts as changed when a channel differs by more than this.
const PIXEL_TOLERANCE: u8 = 24;

impl RowInfo {
    fn same(&self, other: &RowInfo) -> bool {
        let diff: u32 = self.profile.iter().zip(&other.profile).map(|(a, b)| a.abs_diff(*b) as u32).sum();
        diff <= ROW_TOLERANCE * BINS as u32
    }
}

/// Columns left out at each side when comparing rows. Scrollbars live
/// there, and macOS shows one whose thumb moves with every step; the edges
/// never decide the alignment anyway.
fn side_margin(width: u32) -> usize {
    (width / 12).clamp(4, 48) as usize
}

/// Row profiles over the columns in `mask`.
fn rows(image: &RgbaImage, mask: &[bool]) -> Vec<RowInfo> {
    let row = image.width() as usize * 4;
    let cols: Vec<usize> = mask.iter().enumerate().filter(|(_, m)| **m).map(|(x, _)| x).collect();
    let n = cols.len().max(1);
    image
        .as_raw()
        .chunks_exact(row)
        .map(|r| {
            let mut sums = [0u32; BINS];
            let mut counts = [0u32; BINS];
            let (mut lo, mut hi) = (255u8, 0u8);
            for (k, &x) in cols.iter().enumerate() {
                let p = &r[x * 4..x * 4 + 3];
                let grey = ((p[0] as u32 * 3 + p[1] as u32 * 6 + p[2] as u32) / 10) as u8;
                lo = lo.min(grey);
                hi = hi.max(grey);
                let bin = k * BINS / n;
                sums[bin] += grey as u32;
                counts[bin] += 1;
            }
            let mut profile = [0u8; BINS];
            for b in 0..BINS {
                profile[b] = sums[b].checked_div(counts[b]).unwrap_or(0) as u8;
            }
            RowInfo { profile, blank: hi - lo <= 8 }
        })
        .collect()
}

/// What scrolled between two frames of the same size: the span from the
/// first to the last column that differs in at least 5% of rows (a page's
/// blank margins scroll too, but look the same, so the span includes them),
/// and the columns within it to compare rows on. Those are the ones that
/// changed, so a sticky column inside the page ("On this page") and a
/// blinking caret are left out, less a margin at each side of the span
/// for the scrollbar.
fn moving_columns(a: &RgbaImage, b: &RgbaImage) -> Option<((u32, u32), Vec<bool>)> {
    let (w, h) = a.dimensions();
    let mut changes = vec![0u32; w as usize];
    let row = w as usize * 4;
    for (ra, rb) in a.as_raw().chunks_exact(row).zip(b.as_raw().chunks_exact(row)) {
        if ra == rb {
            continue;
        }
        for (x, (pa, pb)) in ra.as_chunks::<4>().0.iter().zip(rb.as_chunks::<4>().0).enumerate() {
            if (0..3).any(|c| pa[c].abs_diff(pb[c]) > PIXEL_TOLERANCE) {
                changes[x] += 1;
            }
        }
    }
    let enough = (h / 20).max(2);
    let first = changes.iter().position(|&c| c >= enough)?;
    let last = changes.iter().rposition(|&c| c >= enough)? + 1;
    if last - first < 16 {
        return None;
    }
    let side = side_margin((last - first) as u32).min((last - first) / 4);
    let mask: Vec<bool> = changes.iter().enumerate().map(|(x, &c)| x >= first + side && x < last - side && c >= enough).collect();
    mask.iter().any(|&m| m).then_some(((first as u32, last as u32), mask))
}

impl Stitcher {
    pub fn new(first: RgbaImage) -> Self {
        let (width, height) = first.dimensions();
        Self {
            width,
            height,
            body: first.as_raw().clone(),
            prev: first,
            footer: 0,
            footer_known: false,
            scrolled: None,
            detail: String::new(),
        }
    }

    /// Height of the image so far.
    pub fn height(&self) -> u32 {
        (self.body.len() / (self.width as usize * 4)) as u32 + self.footer
    }

    /// Add the next frame. `expected` is roughly how far the view was asked
    /// to move, in pixels; it settles ties on pages with repeating content.
    pub fn add(&mut self, frame: RgbaImage, expected: u32) -> Step {
        if frame.dimensions() != (self.width, self.height) {
            return Step::Lost;
        }
        let Some((band, mask)) = moving_columns(&self.prev, &frame) else {
            self.detail = "nothing scrolled".into();
            return Step::Unchanged;
        };
        let (prev_rows, cur) = (rows(&self.prev, &mask), rows(&frame, &mask));
        let h = self.height as usize;
        // A fixed header: rows identical in place. (Blank rows of the page
        // can match by chance too; that only narrows the search.)
        let top = (0..h).take_while(|&y| cur[y].same(&prev_rows[y])).count();
        let mut bottom = if self.footer_known {
            self.footer as usize
        } else {
            (0..h - top).take_while(|&i| cur[h - 1 - i].same(&prev_rows[h - 1 - i])).count()
        };
        if top + bottom + 2 > h {
            self.detail = format!("columns {}..{} changed, but no rows moved", band.0, band.1);
            return Step::Lost;
        }
        let (shift, best) = find_shift(&prev_rows[top..h - bottom], &cur[top..h - bottom], expected);
        self.detail = format!(
            "columns {}..{} ({} compared), fixed rows {top} at the top and {bottom} at the bottom, best match {:.0}% at {} px",
            band.0,
            band.1,
            mask.iter().filter(|&&m| m).count(),
            best.0 * 100.0,
            best.1
        );
        let Some(shift) = shift else {
            return Step::Lost;
        };
        if !self.footer_known && bottom > 0 {
            // Rows that match in place at the bottom may be a real footer,
            // or page (such as blank margin) that happens to look the same.
            // Page content moves with the scroll; a footer doesn't.
            let moved = (h - bottom..h).filter(|&y| y >= shift && cur[y - shift].same(&prev_rows[y])).count();
            if moved * 2 > bottom {
                bottom = 0;
            }
        }
        let row = self.width as usize * 4;
        if !self.footer_known {
            // Now the footer is known: it isn't part of the body.
            let keep = self.body.len() - bottom * row;
            self.body.truncate(keep);
            self.footer = bottom as u32;
            self.footer_known = true;
        }
        // The rows that scrolled into view, just above the footer.
        let new = &frame.as_raw()[(h - bottom - shift) * row..(h - bottom) * row];
        self.body.extend_from_slice(new);
        self.prev = frame;
        self.scrolled = Some(match self.scrolled {
            None => band,
            Some((a, b)) => (a.min(band.0), b.max(band.1)),
        });
        Step::Moved(shift as u32)
    }

    /// The finished image: the body, then the footer from the last frame.
    pub fn finish(self) -> RgbaImage {
        let row = self.width as usize * 4;
        let mut data = self.body;
        let footer = self.footer as usize;
        let last = self.prev.as_raw();
        data.extend_from_slice(&last[last.len() - footer * row..]);
        let height = (data.len() / row) as u32;
        let image = RgbaImage::from_raw(self.width, height, data).expect("whole rows");
        // Trim to what scrolled when the selection was much wider.
        match self.scrolled {
            Some((x0, x1)) if (x1 - x0) * 10 < self.width * 9 => image::imageops::crop_imm(&image, x0, 0, x1 - x0, height).to_image(),
            _ => image,
        }
    }
}

/// How far the content moved up between `before` and `after` (the same rows
/// of both frames), or `None` if no shift lines them up; and the best score
/// seen with its shift, for the log.
fn find_shift(before: &[RowInfo], after: &[RowInfo], expected: u32) -> (Option<usize>, (f64, usize)) {
    let h = before.len();
    let mut best: Option<(f64, usize)> = None;
    let mut best_any = (0.0, 0);
    let mut scores = Vec::with_capacity(h);
    for s in 1..h {
        // after[i] should equal before[i + s].
        let (mut agree, mut total) = (0usize, 0usize);
        for i in 0..h - s {
            let (a, b) = (after[i], before[i + s]);
            if a.blank && b.blank {
                continue;
            }
            total += 1;
            if a.same(&b) {
                agree += 1;
            }
        }
        if total < MIN_EVIDENCE {
            continue;
        }
        let score = agree as f64 / total as f64;
        scores.push((s, score));
        if score > best_any.0 {
            best_any = (score, s);
        }
        if score < if s == expected as usize { MIN_AGREEMENT_EXPECTED } else { MIN_AGREEMENT } {
            continue;
        }
        let better = match best {
            None => true,
            Some((best_score, best_s)) => {
                score > best_score + 1e-9
                    || ((score - best_score).abs() <= 1e-9 && s.abs_diff(expected as usize) < best_s.abs_diff(expected as usize))
            }
        };
        if better {
            best = Some((score, s));
        }
    }
    if best.is_none() && best_any.0 >= MIN_AGREEMENT_CLEAR {
        // Shifts next to the best one score well too; compare with the rest.
        let runner_up = scores.iter().filter(|(s, _)| s.abs_diff(best_any.1) > 4).map(|(_, sc)| *sc).fold(0.0, f64::max);
        if best_any.0 - runner_up >= CLEAR_MARGIN {
            best = Some(best_any);
        }
    }
    (best.map(|(_, s)| s), best_any)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// A tall "page": lines of varied text-like marks on white, with some
    /// blank gaps, so rows are distinctive but not all different.
    fn page(width: u32, height: u32) -> RgbaImage {
        RgbaImage::from_fn(width, height, |x, y| {
            let line = y / 14;
            let in_text = y % 14 < 9 && line % 7 != 3;
            let mark = (x.wrapping_mul(2654435761) ^ (line.wrapping_mul(40503) + (y % 14) * 7919)) % 5 == 0;
            if in_text && mark && x > 8 && x < width - 8 {
                Rgba([20, 20, (line * 37 % 200) as u8, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        })
    }

    /// What the screen shows: a header bar, the page from `offset`, a footer bar.
    fn view(page: &RgbaImage, offset: u32, height: u32, header: u32, footer: u32) -> RgbaImage {
        let w = page.width();
        RgbaImage::from_fn(w, height, |x, y| {
            if y < header {
                Rgba([30, 90, 200, 255]) // plain bar, with a logo
                    .map_with_logo(x, y)
            } else if y >= height - footer {
                Rgba([60, 60, 60, 255])
            } else {
                *page.get_pixel(x, offset + y - header)
            }
        })
    }

    trait Logo {
        fn map_with_logo(self, x: u32, y: u32) -> Rgba<u8>;
    }
    impl Logo for Rgba<u8> {
        fn map_with_logo(self, x: u32, y: u32) -> Rgba<u8> {
            if (10..30).contains(&x) && (5..15).contains(&y) { Rgba([255, 200, 0, 255]) } else { self }
        }
    }

    fn run(header: u32, footer: u32, steps: &[u32]) {
        let (w, vh) = (200, 300);
        let content_h = vh - header - footer;
        let total: u32 = steps.iter().sum();
        let page = page(w, content_h + total);
        let mut offset = 0;
        let mut s = Stitcher::new(view(&page, 0, vh, header, footer));
        for &d in steps {
            offset += d;
            assert_eq!(s.add(view(&page, offset, vh, header, footer), d), Step::Moved(d), "step {d} at {offset}");
        }
        // At the bottom, nothing moves any more.
        assert_eq!(s.add(view(&page, offset, vh, header, footer), 50), Step::Unchanged);
        let out = s.finish();
        let expected = RgbaImage::from_fn(w, header + page.height() + footer, |x, y| {
            if y < header {
                Rgba([30, 90, 200, 255]).map_with_logo(x, y)
            } else if y >= header + page.height() {
                Rgba([60, 60, 60, 255])
            } else {
                *page.get_pixel(x, y - header)
            }
        });
        assert_eq!(out.dimensions(), expected.dimensions());
        assert!(out == expected, "stitched image differs");
    }

    #[test]
    fn plain_page() {
        run(0, 0, &[120, 120, 120, 37]);
    }

    #[test]
    fn sticky_header_and_footer() {
        run(40, 25, &[150, 90, 150, 12]);
    }

    #[test]
    fn small_and_large_steps() {
        run(30, 0, &[5, 200, 1, 180]);
    }

    /// A scrollbar on the right whose thumb moves with every step, as
    /// macOS shows while scrolling: the rows it touches differ.
    #[test]
    fn ignores_a_moving_scrollbar() {
        let p = page(400, 1400);
        let thumb = |img: &mut RgbaImage, top: u32| {
            for y in top..top + 180 {
                for x in 386..396 {
                    img.put_pixel(x, y, Rgba([120, 120, 120, 255]));
                }
            }
        };
        let mut first = view(&p, 0, 600, 0, 0);
        thumb(&mut first, 10);
        let mut s = Stitcher::new(first);
        for (i, offset) in [300u32, 600, 800].into_iter().enumerate() {
            let mut frame = view(&p, offset, 600, 0, 0);
            thumb(&mut frame, 10 + 120 * (i as u32 + 1));
            assert!(matches!(s.add(frame, 300), Step::Moved(_)), "step {i}");
        }
        assert_eq!(s.finish().height(), 1400);
    }

    /// A wide selection: a static sidebar on the left and part of another
    /// window on the right, with the page scrolling between them. The
    /// result is the page alone, stitched correctly.
    #[test]
    fn static_sidebar_beside_the_page() {
        let (pw, vh) = (300u32, 400u32);
        let p = page(pw, 1000);
        let sidebar = |x: u32, y: u32| Rgba([(40 + (y * 3 % 50)) as u8, (x * 5 % 200) as u8, 90, 255]);
        let other = |x: u32, y: u32| Rgba([200, (y % 37 * 6) as u8, (x * 11 % 255) as u8, 255]);
        let frame = |offset: u32| {
            RgbaImage::from_fn(120 + pw + 80, vh, |x, y| {
                if x < 120 {
                    sidebar(x, y)
                } else if x < 120 + pw {
                    *p.get_pixel(x - 120, offset + y)
                } else {
                    other(x, y)
                }
            })
        };
        let mut s = Stitcher::new(frame(0));
        for offset in [240, 480, 600] {
            assert!(matches!(s.add(frame(offset), 240), Step::Moved(_)), "offset {offset}: {}", s.detail);
        }
        assert_eq!(s.add(frame(600), 240), Step::Unchanged);
        let out = s.finish();
        assert_eq!(out.height(), 1000);
        // Trimmed to the page's columns (it may keep a little blank margin).
        assert!(out.width() <= pw && out.width() > pw - 40, "width {}", out.width());
    }

    /// A sticky "On this page" column inside the page, as on documentation
    /// sites: it stays put while the text beside it scrolls.
    #[test]
    fn sticky_column_inside_the_page() {
        let (w, vh) = (400u32, 400u32);
        let p = page(w, 1100);
        let frame = |offset: u32| {
            RgbaImage::from_fn(w, vh, |x, y| {
                if (290..370).contains(&x) {
                    // The sticky column: text lines that don't move.
                    if y % 18 < 10 && (x * 7 + y / 18 * 13) % 4 == 0 { Rgba([60, 60, 60, 255]) } else { Rgba([245, 245, 245, 255]) }
                } else {
                    *p.get_pixel(x, offset + y)
                }
            })
        };
        let mut s = Stitcher::new(frame(0));
        for offset in [240, 480, 700] {
            assert!(matches!(s.add(frame(offset), 240), Step::Moved(_)), "offset {offset}: {}", s.detail);
        }
        assert_eq!(s.finish().height(), 1100);
    }

    /// Browsers redraw text slightly differently as it scrolls (fractional
    /// offsets on Retina displays), so the same row is rarely pixel-exact
    /// from one frame to the next. Every frame here gets its own small
    /// brightness noise.
    #[test]
    fn tolerates_rendering_differences() {
        let p = page(400, 1300);
        let noisy = |offset: u32, seed: u32| {
            let mut f = view(&p, offset, 500, 60, 0);
            for (i, px) in f.pixels_mut().enumerate() {
                let n = ((i as u32).wrapping_mul(2654435761).wrapping_add(seed.wrapping_mul(97)) >> 28) as u8 % 3; // 0..2
                px.0[0] = px.0[0].saturating_sub(n);
                px.0[1] = px.0[1].saturating_add(n);
            }
            f
        };
        let mut s = Stitcher::new(noisy(0, 1));
        for (i, offset) in [300u32, 600, 800].into_iter().enumerate() {
            assert!(matches!(s.add(noisy(offset, i as u32 + 2), 300), Step::Moved(_)), "offset {offset}: {}", s.detail);
        }
        let detail = s.detail.clone();
        // The header, then the page down to the last frame's bottom edge.
        assert_eq!(s.finish().height(), 60 + 800 + 440, "{detail}");
    }

    /// The last scroll stops early at the bottom of the page, so the shift
    /// isn't the distance asked for, and here a quarter of the overlapping
    /// rows also differ (something on the page changed). One shift is still
    /// clearly best, so it's taken.
    #[test]
    fn short_last_scroll_with_some_changes() {
        let p = page(400, 940);
        let mut s = Stitcher::new(view(&p, 0, 500, 0, 0));
        assert_eq!(s.add(view(&p, 300, 500, 0, 0), 300), Step::Moved(300));
        let mut last = view(&p, 440, 500, 0, 0);
        // Disturb every fourth row of the overlap (rows 0..360).
        for y in (0..360).step_by(4) {
            for x in 60..340 {
                last.put_pixel(x, y, Rgba([200, 0, 0, 255]));
            }
        }
        assert_eq!(s.add(last, 300), Step::Moved(140), "{}", s.detail);
        assert_eq!(s.finish().height(), 940);
    }

    /// A blinking text cursor changes a few rows between frames; the shift
    /// is still found from the rest.
    #[test]
    fn tolerates_a_blinking_cursor() {
        let p = page(200, 600);
        let mut s = Stitcher::new(view(&p, 0, 300, 0, 0));
        let mut next = view(&p, 100, 300, 0, 0);
        for y in 120..136 {
            next.put_pixel(50, y, Rgba([0, 0, 0, 255]));
        }
        assert_eq!(s.add(next, 100), Step::Moved(100));
    }

    #[test]
    fn unrelated_frame_is_lost() {
        let p = page(200, 900);
        let mut s = Stitcher::new(view(&p, 0, 300, 0, 0));
        let other = RgbaImage::from_fn(200, 300, |x, y| Rgba([(x * 7 + y * 13) as u8, (x * y) as u8, 9, 255]));
        assert_eq!(s.add(other, 100), Step::Lost);
        // The image so far is untouched.
        assert_eq!(s.finish().height(), 300);
    }
}

