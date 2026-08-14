//! Pixel-format snapshot rendering — render raw bytes as dense pixel glyphs
//! to cut token spend when an agent needs to "see" a buffer instead of reading
//! it as hex or raw text.
//!
//! Two encodings, both zero-dep and pure `std`:
//!
//! - **Braille** ([`braille`]): 8 bits per glyph (2×4 dot grid, U+2800 block).
//!   The densest text-safe encoding — ~1 glyph per byte, so a 4 KiB buffer
//!   renders in ~4 KiB of glyphs instead of ~8 KiB of hex or the raw bytes.
//! - **Half-block** ([`half_block`]): 2 bits per glyph (U+2580–U+2584). Reads
//!   like a 2-row bitmap; best for coarse visual structure (sparsity, runs,
//!   entropy) rather than exact recovery.
//!
//! This is the kernel-side primitive for the "internal commands provide
//! snapshot images in pixel format" optimization: the agent renders a buffer
//! once and reads the compact glyph grid instead of re-reading raw bytes.

use alloc::string::String;
use alloc::vec::Vec;

/// Render `bytes` as a Unicode braille grid (8 bits per glyph, 2×4 dots).
///
/// Layout: each byte's 8 bits map to the standard braille dot positions
/// (dot 1 = bit 0, … dot 8 = bit 7). `width` glyphs per row; rows are joined
/// with newlines. Purely lossless for the bit content (dot order is a fixed
/// permutation), so a reader can recover the byte sequence from the glyphs.
pub fn braille(bytes: &[u8], width: usize) -> String {
    let width = width.max(1);
    let mut out = String::with_capacity(bytes.len() * 4 + bytes.len() / width + 1);
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && i % width == 0 {
            out.push('\n');
        }
        out.push(braille_glyph(b));
    }
    out
}

/// Map one byte to its braille glyph (dots 1–8 = bits 0–7, MSB = dot 8).
fn braille_glyph(byte: u8) -> char {
    // Braille bit order (dots 1..8) → Unicode offsets (1-indexed).
    const DOT_OFFSET: [u32; 8] = [0x01, 0x02, 0x04, 0x40, 0x08, 0x10, 0x20, 0x80];
    let mut code = 0u32;
    for (i, &off) in DOT_OFFSET.iter().enumerate() {
        if (byte >> i) & 1 == 1 {
            code |= off;
        }
    }
    char::from_u32(0x2800 + code).unwrap_or('�')
}

/// Render `bytes` as a half-block bitmap (2 rows per pass, 2 bits per glyph).
///
/// Each glyph encodes the top pixel (bit 0) and bottom pixel (bit 1) of a
/// column; successive byte pairs fill the two rows. `width` columns per row.
/// Good for spotting runs, sparsity and entropy visually.
pub fn half_block(bytes: &[u8], width: usize) -> String {
    let width = width.max(1);
    let mut out = String::with_capacity(bytes.len() + bytes.len() / width + 1);
    // Pair rows: even-indexed byte = top row, odd-indexed = bottom row.
    let rows = bytes.chunks(width);
    let mut row_pairs = rows.collect::<Vec<_>>();
    // Walk pairs of rows (top, bottom).
    let mut i = 0;
    while i < row_pairs.len() {
        let top = row_pairs[i];
        let bottom = row_pairs.get(i + 1).copied().unwrap_or(&[]);
        if i > 0 {
            out.push('\n');
        }
        for col in 0..width {
            let t = top.get(col).copied().unwrap_or(0);
            let b = bottom.get(col).copied().unwrap_or(0);
            out.push(half_block_glyph(t, b));
        }
        i += 2;
    }
    out
}

/// Two-bit column → half-block glyph. Uses bit 0 of each byte.
fn half_block_glyph(top: u8, bottom: u8) -> char {
    match ((top & 1) != 0, (bottom & 1) != 0) {
        (false, false) => ' ',
        (true, false) => '▀',
        (false, true) => '▄',
        (true, true) => '█',
    }
}

// ─── f64 snapshots (lattice / eigen / tensor) ─────────────────────────

/// Render a 1-D `f64` series as a vertical sparkline: one glyph per value,
/// 8 intensity levels (` ▁▂▃▄▅▆▇█`). Values are min-max normalized, so a
/// spectrum (eigenvalues, a tensor row, a lattice coordinate series) reads as
/// a compact bar chart — ~1 token per value.
pub fn sparkline(values: &[f64], width: usize) -> String {
    const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let width = width.max(1);
    if values.is_empty() {
        return String::new();
    }
    let (min, max) = minmax(values);
    let span = (max - min).max(f64::EPSILON);
    let mut out = String::with_capacity(values.len() + values.len() / width);
    for (i, &v) in values.iter().enumerate() {
        if i > 0 && i % width == 0 {
            out.push('\n');
        }
        let t = ((v - min) / span).clamp(0.0, 1.0);
        let level = (t * 7.999) as usize;
        out.push(LEVELS[level]);
    }
    out
}

/// Render a 2-D `f64` matrix (row-major, `cols` columns) as a braille heatmap:
/// one glyph per value, intensity = dot fill (8 levels). For tensors,
/// eigenvector matrices, and lattice density maps.
pub fn heatmap(values: &[f64], cols: usize, width: usize) -> String {
    const DOTS: [u32; 8] = [0x01, 0x02, 0x04, 0x40, 0x08, 0x10, 0x20, 0x80];
    let width = width.max(1);
    if values.is_empty() {
        return String::new();
    }
    let (min, max) = minmax(values);
    let span = (max - min).max(f64::EPSILON);
    let mut out = String::with_capacity(values.len() * 2 + values.len() / width);
    for (i, &v) in values.iter().enumerate() {
        if i > 0 {
            if i % cols == 0 {
                out.push('\n');
            }
        }
        let t = ((v - min) / span).clamp(0.0, 1.0);
        let fill = (t * 7.999) as usize; // 0..=7 dots lit
        let mut code = 0u32;
        for (d, &off) in DOTS.iter().enumerate() {
            if d < fill {
                code |= off;
            }
        }
        out.push(char::from_u32(0x2800 + code).unwrap_or('�'));
    }
    out
}

/// Render a set of 2-D points `(x, y)` as a braille scatter plot.
/// Coordinates are min-max normalized into a `(width*2) × (height*4)` dot grid;
/// `width` cells wide, `height` cells tall. For lattice positions and
/// eigenvector scatter.
pub fn scatter(points: &[(f64, f64)], width: usize, height: usize) -> String {
    // Braille dot → (col, row) within a 2×4 cell.
    const DOT_XY: [(usize, usize); 8] = [
        (0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (0, 3), (1, 3),
    ];
    const DOT_CODE: [u32; 8] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80];
    let width = width.max(1);
    let height = height.max(1);
    let grid_w = width * 2;
    let grid_h = height * 4;
    let mut grid = vec![0u32; width * height];
    if points.is_empty() {
        return String::new();
    }
    let (min_x, max_x) = points.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), p| {
        (mn.min(p.0), mx.max(p.0))
    });
    let (min_y, max_y) = points.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), p| {
        (mn.min(p.1), mx.max(p.1))
    });
    let span_x = (max_x - min_x).max(f64::EPSILON);
    let span_y = (max_y - min_y).max(f64::EPSILON);
    for &(x, y) in points {
        let gx = crate::math::round(((x - min_x) / span_x) * (grid_w as f64 - 1.0)) as usize;
        // invert y so larger y is higher (screen-like)
        let gy = grid_h - 1 - crate::math::round(((y - min_y) / span_y) * (grid_h as f64 - 1.0)) as usize;
        let cell_x = gx / 2;
        let cell_y = gy / 4;
        let dot_col = gx % 2;
        let dot_row = gy % 4;
        for (d, &(dc, dr)) in DOT_XY.iter().enumerate() {
            if dc == dot_col && dr == dot_row {
                grid[cell_y * width + cell_x] |= DOT_CODE[d];
            }
        }
    }
    let mut out = String::with_capacity(width * height + height);
    for r in 0..height {
        if r > 0 {
            out.push('\n');
        }
        for c in 0..width {
            out.push(char::from_u32(0x2800 + grid[r * width + c]).unwrap_or('�'));
        }
    }
    out
}

fn minmax(values: &[f64]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &v in values {
        if v.is_finite() {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
    }
    if min == f64::INFINITY {
        (0.0, 0.0)
    } else {
        (min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braille_is_dense_and_lossless_per_byte() {
        let bytes = [0b1111_1111u8, 0b0000_0000u8, 0b1010_1010u8];
        let s = braille(&bytes, 3);
        // one glyph per byte, no newline (width not exceeded)
        assert_eq!(s.chars().count(), 3);
        // full byte = all dots = U+28FF
        assert_eq!(s.chars().next(), Some('\u{28FF}'));
        // zero byte = empty cell = U+2800
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars[1], '\u{2800}');
    }

    #[test]
    fn braille_wraps_at_width() {
        let bytes = [1u8; 5];
        let s = braille(&bytes, 2);
        assert_eq!(s.chars().filter(|&c| c == '\n').count(), 2); // 5 bytes / 2 = 2 breaks
    }

    #[test]
    fn half_block_encodes_two_rows_per_pass() {
        // top row = all 1s, bottom row = all 0s → all "▀"
        let bytes = [1u8, 1u8, 0u8, 0u8];
        let s = half_block(&bytes, 2);
        assert_eq!(s, "▀▀");
        // top = 0, bottom = 1 → "▄"
        let s2 = half_block(&[0u8, 0u8, 1u8, 1u8], 2);
        assert_eq!(s2, "▄▄");
    }

    #[test]
    fn half_block_mixed_quadrants() {
        // col0: top=1,bottom=1 → █ ; col1: top=1,bottom=0 → ▀
        let s = half_block(&[1u8, 1u8, 1u8, 0u8], 2);
        assert_eq!(s, "█▀");
    }

    #[test]
    fn sparkline_maps_values_to_8_levels() {
        // min→▁, max→█; equal values → all same (█ after normalization to span)
        let s = sparkline(&[0.0, 1.0], 2);
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars[0], '▁');
        assert_eq!(chars[1], '█');
        // constant series → flat baseline ▁ (t = 0 after span clamp)
        assert_eq!(sparkline(&[5.0, 5.0], 2), "▁▁");
    }

    #[test]
    fn heatmap_one_glyph_per_value() {
        let vals = [0.0, 0.5, 1.0];
        let s = heatmap(&vals, 3, 3);
        assert_eq!(s.chars().count(), 3);
        // min value = empty cell (no dots) = U+2800
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars[0], '\u{2800}');
        // max value = nearly full → differs from empty
        assert_ne!(chars[2], '\u{2800}');
    }

    #[test]
    fn scatter_places_distinct_points() {
        let points = [(0.0, 0.0), (1.0, 1.0)];
        let s = scatter(&points, 4, 4);
        // grid has 4 rows, 4 cells each
        assert_eq!(s.lines().count(), 4);
        assert!(s.lines().all(|l| l.chars().count() == 4));
        // at least one dot is lit (not all-empty cells)
        assert!(s.chars().any(|c| c != '\u{2800}' && c != '\n'));
    }

    #[test]
    fn empty_series_are_empty() {
        assert_eq!(sparkline(&[], 4), "");
        assert_eq!(heatmap(&[], 2, 4), "");
        assert_eq!(scatter(&[], 2, 2), "");
    }
}
