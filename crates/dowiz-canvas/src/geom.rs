//! Integer geometry in CSS pixels. Nothing here knows about devicePixelRatio: the loader scales
//! the backing store once (memory sea-canvas-doubles-every-frame: a size read back from the
//! element and multiplied again is how a canvas grew to 3.3e7 px).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const ZERO: Rect = Rect { x: 0, y: 0, w: 0, h: 0 };

    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
    pub const fn right(&self) -> i32 {
        self.x + self.w
    }
    pub const fn bottom(&self) -> i32 {
        self.y + self.h
    }
    /// Half-open on the far edges, so two touching rects never both claim a point.
    pub const fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.right() && py < self.bottom()
    }
    pub const fn intersects(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
    /// Shrunk by `d` on every side (never below zero size).
    pub const fn inset(&self, d: i32) -> Rect {
        let w = if self.w > 2 * d { self.w - 2 * d } else { 0 };
        let h = if self.h > 2 * d { self.h - 2 * d } else { 0 };
        Rect { x: self.x + d, y: self.y + d, w, h }
    }
    /// The top `h` pixels, and what is left below them.
    pub const fn split_top(&self, h: i32) -> (Rect, Rect) {
        let h = if h > self.h { self.h } else { h };
        (Rect { x: self.x, y: self.y, w: self.w, h }, Rect { x: self.x, y: self.y + h, w: self.w, h: self.h - h })
    }
    /// The bottom `h` pixels, and what is left above them.
    pub const fn split_bottom(&self, h: i32) -> (Rect, Rect) {
        let h = if h > self.h { self.h } else { h };
        (Rect { x: self.x, y: self.y, w: self.w, h: self.h - h }, Rect { x: self.x, y: self.bottom() - h, w: self.w, h })
    }
    /// The right `w` pixels, and what is left to their left.
    pub const fn split_right(&self, w: i32) -> (Rect, Rect) {
        let w = if w > self.w { self.w } else { w };
        (Rect { x: self.x, y: self.y, w: self.w - w, h: self.h }, Rect { x: self.right() - w, y: self.y, w, h: self.h })
    }
    /// Moved down by `dy` (a scrolled list's rows).
    pub const fn shifted(&self, dy: i32) -> Rect {
        Rect { x: self.x, y: self.y + dy, w: self.w, h: self.h }
    }
}
