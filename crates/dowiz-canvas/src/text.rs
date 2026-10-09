//! TEXT WIDTHS, measured by the host once and kept.
//!
//! The host's Canvas2D shapes and rasterises every glyph (system fonts: no font download, Cyrillic
//! and Albanian diacritics as the phone draws them -- research §4.4). What Rust needs is only the
//! WIDTH of a run, to lay out and to ellipsise; asking the host per frame would cost a JS call per
//! word per frame. So each (run, size, weight) is measured once into this table and read back
//! from it. A full table forgets everything and starts again (counted in `resets`): a correct
//! re-measure is cheaper than a wrong width.

/// Who answers a width. In wasm it is the loader (`ffi.rs`); in tests a fixed-advance fake.
pub trait Measure {
    /// Width of `s` at `px` CSS pixels and `weight` (400/700), rounded UP to whole pixels.
    fn measure(&mut self, s: &str, px: i32, weight: i32) -> i32;
}

const SLOTS: usize = 4096;

pub struct Widths {
    keys: [u32; SLOTS],
    vals: [i32; SLOTS],
    used: usize,
    pub misses: u32,
    pub resets: u32,
}

fn key(s: &str, px: i32, weight: i32) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for &b in s.as_bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h ^= (px as u32) << 8 | (weight as u32 / 100);
    h = h.wrapping_mul(0x0100_0193);
    if h == 0 {
        1
    } else {
        h
    }
}

impl Widths {
    pub const fn new() -> Widths {
        Widths { keys: [0; SLOTS], vals: [0; SLOTS], used: 0, misses: 0, resets: 0 }
    }

    /// Forget every width (a font or devicePixelRatio change).
    pub fn reset(&mut self) {
        self.keys = [0; SLOTS];
        self.used = 0;
        self.resets += 1;
    }

    /// The width of `s`, asking `host` only the first time.
    pub fn get(&mut self, host: &mut dyn Measure, s: &str, px: i32, weight: i32) -> i32 {
        if s.is_empty() {
            return 0;
        }
        let k = key(s, px, weight);
        let mut i = k as usize & (SLOTS - 1);
        loop {
            if self.keys[i] == k {
                return self.vals[i];
            }
            if self.keys[i] == 0 {
                break;
            }
            i = (i + 1) & (SLOTS - 1);
        }
        self.misses += 1;
        let w = host.measure(s, px, weight).max(0);
        // Three quarters full: start again rather than probe forever.
        if self.used * 4 >= SLOTS * 3 {
            self.reset();
            return self.get(host, s, px, weight);
        }
        self.keys[i] = k;
        self.vals[i] = w;
        self.used += 1;
        w
    }
}

/// The longest prefix of `s` (on a char boundary) whose width plus an ellipsis fits `max`.
/// Returns (prefix, cut): `cut` says an ellipsis must follow.
pub fn fit<'a>(w: &mut Widths, host: &mut dyn Measure, s: &'a str, px: i32, weight: i32, max: i32) -> (&'a str, bool) {
    if w.get(host, s, px, weight) <= max {
        return (s, false);
    }
    let ell = w.get(host, "…", px, weight);
    // Binary search over CHARACTER counts (widths are monotone in the prefix length); `lo` chars
    // always fit, `hi` is the most that might.
    // Characters = bytes that do not continue one (`chars().count()` links core's counting
    // routines: a kilobyte of wasm for this one call).
    let (mut lo, mut hi) = (0usize, s.bytes().filter(|b| b & 0xC0 != 0x80).count());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if w.get(host, head(s, mid), px, weight) + ell <= max {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    (head(s, lo), true)
}

/// The first `n` characters of `s` (all of it when shorter). `get`, never `&s[..i]`: a str index
/// links core's char-boundary panic message and its Unicode tables -- kilobytes of wasm for a
/// path that cannot run (the offset comes from `char_indices`).
fn head(s: &str, n: usize) -> &str {
    let i = s.char_indices().nth(n).map_or(s.len(), |(i, _)| i);
    s.get(..i).unwrap_or("")
}
