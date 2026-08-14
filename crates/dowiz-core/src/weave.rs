//! weave.rs — procedural pattern generators rendered as glyphs.
//!
//! Item #18 (Figma Weave): the reusable principle is *procedural geometry* —
//! describe a pattern, get a deterministic visual. The post's examples
//! (Cross-Stitch Chart, Spiral Symbol Weave, Brick Wall, Glitch Bloom) map
//! directly to the kernel's glyph architecture: generate a 2D field, render it
//! as braille/half-block via `pixel_snapshot`.
//!
//! Zero-dep, deterministic (seeded PRNG where randomness is wanted).

use alloc::string::String;
use alloc::vec::Vec;

/// Archimedean spiral points: `r = radius * t`, angle advances `turns` times.
/// Returns `steps` (x, y) points in [−r, r]².
pub fn spiral_points(radius: f64, turns: f64, steps: usize) -> Vec<(f64, f64)> {
    let mut pts = Vec::with_capacity(steps);
    for i in 0..steps {
        let t = i as f64 / (steps.max(1) - 1) as f64;
        let r = radius * t;
        let theta = t * turns * 2.0 * core::f64::consts::PI;
        pts.push((r * crate::math::cos(theta), r * crate::math::sin(theta)));
    }
    pts
}

/// Render a spiral as a braille scatter plot.
pub fn render_spiral(radius: f64, turns: f64, steps: usize, width: usize, height: usize) -> String {
    let pts = spiral_points(radius, turns, steps);
    crate::glyph_dashboard::render_scatter(&pts, width, height)
}

/// A bool grid: `true` = filled dot.
pub type Grid = Vec<Vec<bool>>;

/// Brick wall pattern (offset every other row by half a brick).
pub fn brick_wall(rows: usize, cols: usize) -> Grid {
    let mut g = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut row = Vec::with_capacity(cols);
        let offset = if r % 2 == 1 { cols / 4 } else { 0 };
        for c in 0..cols {
            // Brick joints every `cols/2` cells; offset rows shift the joint.
            let brick = ((c + offset) / (cols / 2)) % 2 == 0;
            let mortar = r % 2 == 0 && c % 2 == 0; // simple joint line
            row.push(brick && !mortar);
        }
        g.push(row);
    }
    g
}

/// Cross-stitch (X) pattern on an n×n grid.
pub fn cross_stitch(n: usize) -> Grid {
    let mut g = vec![vec![false; n]; n];
    for r in 0..n {
        for c in 0..n {
            let diag = r == c || r + c == n - 1;
            let center = r == n / 2 || c == n / 2;
            g[r][c] = diag || center;
        }
    }
    g
}

/// Deterministic glitch pattern: horizontal tear lines + sparse noise,
/// seeded by `seed` (splitmix64, the kernel's canonical PRNG).
pub fn glitch(rows: usize, cols: usize, seed: u64) -> Grid {
    let mut g = vec![vec![false; cols]; rows];
    let mut s = seed;
    let mut next = move || {
        s = s.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    };
    // Tear lines: a few full rows are "shifted" (bright).
    let tears = 3;
    for _t in 0..tears {
        let row = (next() as usize) % rows;
        for c in 0..cols {
            g[row][c] = (next() & 3) != 0;
        }
    }
    // Sparse noise.
    for _ in 0..(rows * cols / 5) {
        let r = (next() as usize) % rows;
        let c = (next() as usize) % cols;
        g[r][c] = true;
    }
    g
}

/// Render a bool grid as braille glyphs (each glyph = 2×4 dots).
pub fn render_grid(g: &Grid) -> String {
    if g.is_empty() {
        return String::new();
    }
    let rows = g.len();
    let cols = g[0].len();
    // Pack 8 bits (2 rows × 4 cols) per braille cell.
    let mut out = String::new();
    const DOT_OFFSET: [u32; 8] = [0x01, 0x08, 0x02, 0x10, 0x04, 0x20, 0x40, 0x80];
    for br in (0..rows).step_by(4) {
        for bc in (0..cols).step_by(2) {
            let mut code = 0u32;
            for dr in 0..4 {
                for dc in 0..2 {
                    let r = br + dr;
                    let c = bc + dc;
                    if r < rows && c < cols && g[r][c] {
                        // Braille dot layout: dots 1-4 down the left column,
                        // dots 5-8 down the right column.
                        let dot = if dc == 0 { dr } else { dr + 4 };
                        code |= DOT_OFFSET[dot];
                    }
                }
            }
            out.push(char::from_u32(0x2800 + code).unwrap_or(' '));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spiral_points_are_bounded() {
        let pts = spiral_points(1.0, 3.0, 100);
        assert_eq!(pts.len(), 100);
        for &(x, y) in &pts {
            assert!(x.abs() <= 1.0 + 1e-9);
            assert!(y.abs() <= 1.0 + 1e-9);
        }
    }

    #[test]
    fn render_spiral_emits_braille() {
        let s = render_spiral(1.0, 3.0, 50, 16, 16);
        assert!(!s.is_empty());
        assert!(s.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c)));
    }

    #[test]
    fn brick_wall_shape_and_offset() {
        let g = brick_wall(4, 8);
        assert_eq!(g.len(), 4);
        assert_eq!(g[0].len(), 8);
        assert_ne!(g[0], g[1], "offset rows must differ");
    }

    #[test]
    fn cross_stitch_has_diagonals() {
        let g = cross_stitch(5);
        assert!(g[0][0] && g[0][4] && g[4][0] && g[4][4], "corners on diagonals");
        assert!(g[2][2], "center");
    }

    #[test]
    fn glitch_is_deterministic() {
        let a = glitch(16, 16, 42);
        let b = glitch(16, 16, 42);
        assert_eq!(a, b, "same seed must yield identical pattern");
        let c = glitch(16, 16, 43);
        assert_ne!(a, c, "different seed must differ");
    }

    #[test]
    fn render_grid_emits_braille() {
        let g = brick_wall(8, 8);
        let s = render_grid(&g);
        assert!(!s.is_empty());
        assert!(s.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c)));
    }
}
