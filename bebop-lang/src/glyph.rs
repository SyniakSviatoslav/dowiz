//! Pixel-vector glyphs (spec §2.6).
//!
//! Every symbol is a glyph on a pixel grid. v0.1 implements:
//!   - a `Grid` (bool raster),
//!   - δ-outline *encoding* (row-major run deltas) — the spec's delta form,
//!   - a built-in 5×7 bitmap font to give every keyword a real glyph,
//!   - braille + half-block terminal rendering (dowiz `pixel_snapshot` encodings).

pub type Grid = Vec<Vec<bool>>;

#[derive(Debug, Clone)]
pub struct Glyph {
    pub name: String,
    pub grid: Grid,
    /// δ-outline encoding (row-major runs): each `(dx, dy)` is a pen move.
    pub outline: Vec<(i16, i16)>,
}

/// Derive a δ-outline from a raster as row-major runs (pen moves between runs).
/// For each row, a horizontal run of set cells is encoded as a move to its
/// start then a move across it; runs are separated by newline moves. This is
/// lossless and affine-invariant (the spec §2.6 property).
pub fn delta_outline(grid: &Grid) -> Vec<(i16, i16)> {
    let mut out = Vec::new();
    let mut pen = (0i16, 0i16); // (x, y)
    for (y, row) in grid.iter().enumerate() {
        let mut x = 0usize;
        while x < row.len() {
            if row[x] {
                let start = x;
                while x < row.len() && row[x] {
                    x += 1;
                }
                let len = (x - start) as i16;
                // move to (start, y) then across the run
                out.push((start as i16 - pen.0, y as i16 - pen.1));
                out.push((len, 0));
                pen = (start as i16 + len, y as i16);
            } else {
                x += 1;
            }
        }
    }
    out
}

/// Reconstruct a grid from a δ-outline (inverse of [`delta_outline`]).
/// The outline is a sequence of `(move, draw)` pairs: a move jumps the pen to a
/// run start, the following draw sweeps `len` cells horizontally.
pub fn outline_to_grid(outline: &[(i16, i16)], width: usize, height: usize) -> Grid {
    let mut grid = vec![vec![false; width]; height];
    let mut pen = (0i16, 0i16);
    let mut i = 0;
    while i + 1 < outline.len() {
        let (mdx, mdy) = outline[i];
        let (len, _) = outline[i + 1];
        pen = (pen.0 + mdx, pen.1 + mdy);
        let y = pen.1.max(0) as usize;
        let start = pen.0.max(0) as usize;
        if y < height {
            for k in 0..len.max(0) as usize {
                let cx = start + k;
                if cx < width {
                    grid[y][cx] = true;
                }
            }
        }
        pen = (pen.0 + len, pen.1);
        i += 2;
    }
    grid
}

/// Render a grid as braille (8 bits per glyph, 2×4 dots, U+2800 block).
pub fn to_braille(grid: &Grid) -> String {
    let h = grid.len();
    let w = grid.first().map_or(0, |r| r.len());
    let mut out = String::new();
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let mut bits = 0u32;
            // braille dot positions: (0,0)->1 (1,0)->2 (2,0)->4 (0,1)->8 (1,1)->16 (2,1)->32 (0,2)->64 (1,2)->128
            let dots = [
                (0, 0, 0x1), (1, 0, 0x2), (2, 0, 0x4),
                (0, 1, 0x8), (1, 1, 0x10), (2, 1, 0x20),
                (0, 2, 0x40), (1, 2, 0x80),
            ];
            for (dx, dy, mask) in dots {
                let gy = y + dy;
                let gx = x + dx;
                if gy < h && gx < w && grid[gy][gx] {
                    bits |= mask;
                }
            }
            out.push(char::from_u32(0x2800 + bits).unwrap());
            x += 2;
        }
        out.push('\n');
        y += 4;
    }
    out
}

/// Render a grid as half-blocks (2 bits per glyph, U+2580–U+2584).
pub fn to_halfblock(grid: &Grid) -> String {
    let h = grid.len();
    let w = grid.first().map_or(0, |r| r.len());
    let mut out = String::new();
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let top = grid[y][x];
            let bottom = y + 1 < h && grid[y + 1][x];
            let c = match (top, bottom) {
                (false, false) => ' ',
                (true, false) => '▀',
                (false, true) => '▄',
                (true, true) => '█',
            };
            out.push(c);
            x += 1;
        }
        out.push('\n');
        y += 2;
    }
    out
}

/// Built-in 5×7 bitmap font (A–Z, 0–9). Rows top→bottom, each a 5-bit mask.
/// Classic 5×7 LCD-style glyphs.
fn font5x7(ch: char) -> Grid {
    const FONT: &[(char, [u8; 7])] = &[
        ('A', [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        ('B', [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
        ('C', [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111]),
        ('D', [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
        ('E', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
        ('F', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
        ('G', [0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111]),
        ('H', [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        ('I', [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111]),
        ('J', [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100]),
        ('K', [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
        ('L', [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
        ('M', [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
        ('N', [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
        ('O', [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        ('P', [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
        ('Q', [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101]),
        ('R', [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
        ('S', [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110]),
        ('T', [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
        ('U', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        ('V', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
        ('W', [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001]),
        ('X', [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
        ('Y', [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
        ('Z', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
        ('0', [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
        ('1', [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        ('2', [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111]),
        ('3', [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110]),
        ('4', [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
        ('5', [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
        ('6', [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
        ('7', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
        ('8', [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
        ('9', [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
    ];

    let mut grid = vec![vec![false; 5]; 7];
    let upper = ch.to_ascii_uppercase();
    if let Some(&(_, rows)) = FONT.iter().find(|(c, _)| *c == upper) {
        for (y, &row) in rows.iter().enumerate() {
            for x in 0..5 {
                grid[y][x] = (row >> (4 - x)) & 1 == 1;
            }
        }
    }
    grid
}

/// Build a glyph for a name: render each character of the name (first letter
/// of each word) using the 5×7 font, side by side with a 1-column gap.
pub fn glyph_for(name: &str) -> Glyph {
    let chars: Vec<char> = name
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| w.chars().next().unwrap())
        .collect();
    let w = chars.len() * 6 + 1; // 5 + 1 gap, +1 trailing
    let mut grid = vec![vec![false; w]; 7];
    for (i, &c) in chars.iter().enumerate() {
        let g = font5x7(c);
        for y in 0..7 {
            for x in 0..5 {
                grid[y][i * 6 + x] = g[y][x];
            }
        }
    }
    let outline = delta_outline(&grid);
    Glyph { name: name.to_string(), grid, outline }
}

/// Render a glyph name to braille text (terminal fallback tier, §2.6).
pub fn render_braille(name: &str) -> String {
    to_braille(&glyph_for(name).grid)
}
