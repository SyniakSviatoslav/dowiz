//! INTEGER LAYOUT: rows, columns and a wrapping flow, in whole CSS pixels.
//!
//! No fractions anywhere: a remainder is handed out one pixel at a time from the left, so the
//! cells of a row always sum to exactly the space they were given (`tests.rs` checks it for every
//! width from 1 to 2000). That is what keeps two frames at the same size byte-identical.

use crate::geom::Rect;

/// `n` equal columns across `area` with `gap` between them, written into `out[..n]`.
/// Returns how many were written (0 when `n` is 0 or `out` is short).
pub fn columns(area: Rect, n: usize, gap: i32, out: &mut [Rect]) -> usize {
    if n == 0 || out.len() < n {
        return 0;
    }
    let free = (area.w - gap * (n as i32 - 1)).max(0);
    let base = free / n as i32;
    let extra = free - base * n as i32;
    let mut x = area.x;
    for (i, slot) in out.iter_mut().take(n).enumerate() {
        let w = base + if (i as i32) < extra { 1 } else { 0 };
        *slot = Rect::new(x, area.y, w, area.h);
        x += w + gap;
    }
    n
}

/// Cells of the given widths left to right, wrapping to a new line of height `line_h` when the
/// next one would cross `area`'s right edge. Writes `out[i]` for each width; returns the total
/// height used.
pub fn flow(area: Rect, widths: &[i32], line_h: i32, gap: i32, out: &mut [Rect]) -> i32 {
    let (mut x, mut y) = (area.x, area.y);
    for (i, &w) in widths.iter().enumerate() {
        if i >= out.len() {
            break;
        }
        let w = w.min(area.w);
        if x > area.x && x + w > area.right() {
            x = area.x;
            y += line_h + gap;
        }
        out[i] = Rect::new(x, y, w, line_h);
        x += w + gap;
    }
    if widths.is_empty() {
        0
    } else {
        y + line_h - area.y
    }
}

/// A vertical stack: each height in turn below the last, `gap` between. Returns the total height.
pub fn stack(area: Rect, heights: &[i32], gap: i32, out: &mut [Rect]) -> i32 {
    let mut y = area.y;
    for (i, &h) in heights.iter().enumerate() {
        if i >= out.len() {
            break;
        }
        out[i] = Rect::new(area.x, y, area.w, h);
        y += h + gap;
    }
    if heights.is_empty() {
        0
    } else {
        y - gap - area.y
    }
}
