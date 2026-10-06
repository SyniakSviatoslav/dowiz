//! PURE. THE INTEGER ARITHMETIC THE PHONE AND THE HUB RANK DISHES WITH (W-TASTE, report D.1 #6, #7).
//!
//! The device ranked the "For you" strip in floats (`Math.pow` decay, `Math.sqrt` cosine) while
//! the server folded the same guest in `i64` with `UNIT = 1000` -- and its half-life was an `f64`
//! `powf().round()`, so neither side was bit-reproducible on another machine. This module is the
//! one definition both now use; `workers/api/public/store/taste-int.js` is its line-for-line
//! JavaScript twin, and `rank/tests.rs` + `store/taste-int.test.mjs` hold the two equal on one
//! shared fixture (`fixtures/rank/strip.json`, 1000 guests x 4 menus).
//!
//! THE RULES, each a test:
//!   * NO FLOAT DECIDES ANYTHING. Every value is an integer below 2^53, so a JavaScript number
//!     holds it exactly and every `+`, `*` and truncating `/` agrees with Rust's. The one `sqrt`
//!     ([`isqrt`]) is a float GUESS corrected by integer comparisons: its answer is exact.
//!   * THE HALF-LIFE IS A TABLE. `fade(w, d)` = `w * 2^(-d/60)`, rounded half away from zero,
//!     from a 60-entry Q16 table built by a Q32 multiply chain (`STEP`), the same chain in both
//!     languages (BigInt in JS). Sixty days is EXACTLY half (`HALF[0] = 2^16`, one more shift).
//!   * COSINE IN PER MILLE. `cos_pm(a, b) = trunc(dot * 1000 / isqrt(|a|^2 |b|^2))`, 0 when either
//!     side is empty. A guest's vector is first scaled to per mille of its top key ([`per_mille`]),
//!     so every square stays below 2^53.
//!   * TIES BREAK BY THE ID'S BYTES, never by a locale's collation.

use std::collections::BTreeMap;

pub mod strip;

/// One portion, as an integer weight: the server's `customers::taste::UNIT`.
pub const UNIT: i64 = 1000;
/// Days for a weight to count half.
pub const HALF_LIFE_DAYS: i64 = 60;
/// round(2^32 * 2^(-1/60)): one day of the half-life, as a Q32 factor.
pub const STEP: u64 = 4_245_635_389;
/// A weight is clamped to this magnitude before it is faded, so `|w| * 2^16` stays below 2^53.
pub const W_MAX: i64 = (1 << 37) - 1;

/// `HALF[r]` = round(2^16 * 2^(-r/60)), r = 0..60, from the Q32 chain.
pub static HALF: [i64; 60] = half_table();

const fn half_table() -> [i64; 60] {
    let mut q32 = 1u64 << 32;
    let mut out = [0i64; 60];
    let mut r = 0;
    while r < 60 {
        out[r] = ((q32 + (1 << 15)) >> 16) as i64;
        q32 = ((q32 as u128 * STEP as u128 + (1u128 << 31)) >> 32) as u64;
        r += 1;
    }
    out
}

/// `w` after `days` of the half-life. A day not after (`days <= 0`) leaves it as it is.
pub fn fade(w: i64, days: i64) -> i64 {
    if days <= 0 {
        return w;
    }
    let s = 16 + days / HALF_LIFE_DAYS;
    if s > 52 {
        return 0;
    }
    let m = u128::from(w.unsigned_abs().min(W_MAX as u64)) * HALF[(days % HALF_LIFE_DAYS) as usize] as u128;
    let q = ((m + (1u128 << (s - 1))) >> s) as i64;
    if w < 0 {
        -q
    } else {
        q
    }
}

/// floor(sqrt(n)) for 0 <= n < 2^53, exactly; 0 for n <= 0.
pub fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    if n < 1 << 52 {
        // The hot path (every caller here): r < 2^26, so r*r and (r+1)^2 fit an i64.
        let mut r = (n as f64).sqrt() as i64;
        while r * r > n {
            r -= 1;
        }
        while (r + 1) * (r + 1) <= n {
            r += 1;
        }
        return r;
    }
    let n = i128::from(n);
    let mut r = (n as f64).sqrt() as i128;
    while r * r > n {
        r -= 1;
    }
    while (r + 1) * (r + 1) <= n {
        r += 1;
    }
    r as i64
}

/// A vector scaled to per mille of its strongest key (truncated); keys at 0 or below dropped.
pub fn per_mille(v: &BTreeMap<String, i64>) -> BTreeMap<String, i64> {
    let top = v.values().copied().max().unwrap_or(0).min(W_MAX);
    if top <= 0 {
        return BTreeMap::new();
    }
    v.iter().filter(|(_, w)| **w > 0).map(|(k, w)| (k.clone(), ((*w).min(W_MAX) * 1000 / top).min(1000))).filter(|(_, w)| *w > 0).collect()
}

/// Cosine of two sparse vectors, per mille, truncated toward zero; 0 when either is empty.
/// Callers pass vectors whose entries are at most 1000 in magnitude (`per_mille`, a dish's
/// `sense::vector`, a mood), so `|a|^2 * |b|^2` < 2^53 for the 27 keys of the vocabulary.
pub fn cos_pm(a: &BTreeMap<String, i64>, b: &BTreeMap<String, i64>) -> i64 {
    let (mut dot, mut na, mut nb) = (0i64, 0i64, 0i64);
    for (k, x) in a {
        na += x * x;
        if let Some(y) = b.get(k) {
            dot += x * y;
        }
    }
    for y in b.values() {
        nb += y * y;
    }
    if na == 0 || nb == 0 {
        return 0;
    }
    dot * 1000 / isqrt(na * nb)
}

#[cfg(test)]
#[path = "rank/tests.rs"]
mod tests;
