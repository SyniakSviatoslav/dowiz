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
}
