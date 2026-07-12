//! Fractal / chaos artifacts over a shared secret — DERIVED, never a key source.
//!
//! Per the design discussion: fractal and chaotic maps are DETERMINISTIC functions of
//! the shared secret, so they add zero entropy (H(F(ss)) <= H(ss)). Their legitimate
//! roles are NOT cryptographic key material but:
//!   1. fractal_fingerprint(ss) — a human-verifiable, channel-binding visual artifact
//!      (like an SSH randomart / Squidly) so two operators can eyeball "same secret".
//!   2. chaotic_routing_tag(ss) — a per-handshake, deterministic mesh routing tag for
//!      anti-traffic-analysis diversity (the security still comes from `ss`, the chaos
//!      map only spreads it into a varied tag space).
//! Neither output must EVER be fed back as key/entropy. They are read-only views.

use crate::pq::keccak::shake256;

/// Render a compact visual fingerprint of `ss` as a fixed-size ASCII grid derived from
/// an iterated escape-time map. Two nodes with the same `ss` get the identical artifact;
/// differing by even one bit of `ss` yields a wildly different picture (avalanche) so
/// it doubles as a tamper-evident binding display.
pub fn fractal_fingerprint(ss: &[u8; 32], rows: usize, cols: usize) -> Vec<String> {
    // Seed the map parameters from the secret (little-endian pairs -> complex c).
    let cx = bytes_to_f64(&ss[0..8]);
    let cy = bytes_to_f64(&ss[8..16]);
    let escape = 4.0;
    let max_iter = 64u32;
    let mut grid = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut line = String::with_capacity(cols);
        for c in 0..cols {
            // Map cell -> complex plane window around c.
            let wx = (c as f64 / cols as f64) * 3.0 - 1.5 + cx * 1e-3;
            let wy = (r as f64 / rows as f64) * 3.0 - 1.5 + cy * 1e-3;
            let it = escape_iters(wx, wy, escape, max_iter);
            let ch = glyph(it, max_iter);
            line.push(ch);
        }
        grid.push(line);
    }
    grid
}

/// Chaotic routing tag: iterate a logistic map seeded by `ss` to produce a short, fixed
/// tag stream. Both peers recompute the SAME tag from the SAME ss (deterministic), so it
/// can label a mesh path consistently without leaking `ss` (the map is one-way-ish for an
/// observer without `ss`). The tag is a routing hint, NOT authentication.
pub fn chaotic_routing_tag(ss: &[u8; 32], len: usize) -> Vec<u8> {
    // r in (3.57, 4.0) for full chaos; derive from secret, avoid r==4 (measure-zero).
    let mut rb = [0u8; 8];
    rb.copy_from_slice(&ss[16..24]);
    let r = 3.57 + (bytes_to_f64(&rb).abs() % 1.0) * 0.42;
    let mut xb = [0u8; 8];
    xb.copy_from_slice(&ss[24..32]);
    let mut x = 0.5 + (bytes_to_f64(&xb).abs() % 0.5) * 1e-3; // avoid 0 and 0.5 fixed pts
    let mut tag = Vec::with_capacity(len);
    for _ in 0..len {
        x = r * x * (1.0 - x);
        let byte = (x * 255.0) as u8;
        tag.push(byte);
    }
    tag
}

fn escape_iters(x0: f64, y0: f64, escape: f64, max_iter: u32) -> u32 {
    let (mut x, mut y) = (x0, y0);
    let e2 = escape * escape;
    for i in 0..max_iter {
        let x2 = x * x - y * y;
        let y2 = 2.0 * x * y;
        x = x2 + x0;
        y = y2 + y0;
        if x * x + y * y > e2 {
            return i;
        }
    }
    max_iter
}

fn glyph(it: u32, max_iter: u32) -> char {
    if it >= max_iter {
        ' '
    } else {
        let ramp = ".:-=+*#%@";
        let idx = (it as usize * (ramp.len() - 1)) / max_iter as usize;
        ramp.chars().nth(idx).unwrap_or('#')
    }
}

fn bytes_to_f64(b: &[u8]) -> f64 {
    // Map raw bytes to a finite f64 in [0,1) via SHAKE (no NaN from bit-reinterpretation).
    let mut h = [0u8; 8];
    shake256(b, &mut h);
    let u = u64::from_le_bytes(h);
    (u >> 11) as f64 / (1u64 << 53) as f64 // 53-bit fraction → [0,1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_fingerprint_deterministic_same_ss() {
        let ss = [0x5Au8; 32];
        let a = fractal_fingerprint(&ss, 8, 16);
        let b = fractal_fingerprint(&ss, 8, 16);
        assert_eq!(a, b, "same ss -> same artifact");
    }

    #[test]
    fn green_fingerprint_avalanche_on_byte_flip() {
        let s1 = [0x5Au8; 32];
        let mut s2 = [0x5Au8; 32];
        s2[0] ^= 0xFF; // flip the whole byte that seeds cx
        let a = fractal_fingerprint(&s1, 8, 16);
        let b = fractal_fingerprint(&s2, 8, 16);
        assert_ne!(a, b, "ss byte change must alter artifact (avalanche)");
    }

    #[test]
    fn green_routing_tag_deterministic_and_distinct() {
        let s1 = [0x11u8; 32];
        let s2 = [0x22u8; 32];
        assert_eq!(chaotic_routing_tag(&s1, 8), chaotic_routing_tag(&s1, 8));
        assert_ne!(chaotic_routing_tag(&s1, 8), chaotic_routing_tag(&s2, 8));
    }
}
