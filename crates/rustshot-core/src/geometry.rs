use serde::{Deserialize, Serialize};

/// A rectangle in global desktop coordinates (logical points, origin at the
/// top-left of the primary monitor).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Region {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// Build a normalised region from two corner points in any order.
    pub fn from_points(a: (f64, f64), b: (f64, f64)) -> Self {
        let x = a.0.min(b.0);
        let y = a.1.min(b.1);
        Self { x, y, w: (a.0 - b.0).abs(), h: (a.1 - b.1).abs() }
    }

    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    pub fn is_empty(&self) -> bool {
        self.w < 1.0 || self.h < 1.0
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    pub fn intersect(&self, other: &Region) -> Option<Region> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = self.right().min(other.right());
        let b = self.bottom().min(other.bottom());
        (r > x && b > y).then(|| Region::new(x, y, r - x, b - y))
    }

    pub fn area(&self) -> f64 {
        self.w * self.h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_points_normalises() {
        let r = Region::from_points((10.0, 20.0), (5.0, 2.0));
        assert_eq!(r, Region::new(5.0, 2.0, 5.0, 18.0));
    }

    #[test]
    fn intersect() {
        let a = Region::new(0.0, 0.0, 100.0, 100.0);
        let b = Region::new(50.0, 50.0, 100.0, 100.0);
        assert_eq!(a.intersect(&b), Some(Region::new(50.0, 50.0, 50.0, 50.0)));
        assert_eq!(a.intersect(&Region::new(200.0, 0.0, 10.0, 10.0)), None);
    }
}
