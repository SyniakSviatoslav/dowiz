//! `taste.dwb` -- THE DISHES' SENSE AS INTEGER COLUMNS, and an exact top-k over it (W-TASTE row 2;
//! report D.1 #7, C.1, C.11; operator 2026-10-05: C-1 lifted NARROWLY for integer columnar dish
//! vectors in a DG7 block -- prices and stock stay out of any tensor).
//!
//! WHAT IT IS. One `schema::TASTE` block per catalogue generation, folded in the same pass as the
//! other §B.4 blocks (`workers/api/src/fold/menu.rs`), read in place through `View`. Per dish: the
//! six taste axes, texture and aroma intensities as two bit planes each, the EU14 allergens as a
//! mask, and "on sale" -- every value from the SAME readers the JSON paths use
//! (`sense::of_product`, `allergens::read`), so the block and the JSON cannot disagree.
//!
//! THE SCORER. `top_k(view, want, avoid, k)`: every dish on sale whose allergens do not meet
//! `avoid`, scored by `rank::cos_pm(want, dish vector)`, kept when the score is above 0, best first,
//! ties by the id's bytes. EXACT (a full scan: 165 rows is below every vector store's own
//! full-scan threshold, report C.1) and DETERMINISTIC (integers only). `top_k_json` is the same
//! question over the product JSON -- the path the block replaces -- and the tests hold the two
//! byte-equal on every fixture.
//!
//! ALLERGENS EXCLUDE BY MASK, BEFORE ANY SCORE: `avoid_mask(codes)` sets bit 15 (undeclared) as
//! soon as one code is asked, because "nobody said" is not "safe" (store/avoid.js rule 1). Scalar
//! code, no SIMD (operator C-1, `view.rs`).

use std::collections::BTreeMap;

use serde_json::Value;

use super::schema;
use super::view::View;
use super::{Block, Col, Refusal};
use crate::sense::{self, Dim, TAG_MAX, TASTE_MAX};

/// `taste` columns, in schema order.
pub const DISH: usize = 0;
pub const AXIS0: usize = 3;
pub const TEX_LO: usize = 9;
pub const TEX_HI: usize = 10;
pub const ARO_LO: usize = 11;
pub const ARO_HI: usize = 12;
pub const ALLERGENS: usize = 13;
pub const FLAGS: usize = 14;
/// `allergens` bit 15: the dish declares nothing.
pub const UNDECLARED: i64 = 1 << 15;
/// `flags` bit 0: on sale.
pub const ON_SALE: i64 = 1;

/// The allergen mask of a product (`allergens::read`): bit i = EU14[i], UNDECLARED when nobody said.
pub fn allergen_mask(product_json: &str) -> i64 {
    match crate::allergens::read(product_json) {
        crate::allergens::Declaration::Undeclared => UNDECLARED,
        d => d.codes().iter().filter_map(|c| crate::allergens::EU14.iter().position(|e| e == c)).fold(0, |m, i| m | 1 << i),
    }
}

/// What a guest asked to avoid, as a mask. Any code asked also avoids UNDECLARED dishes.
pub fn avoid_mask(codes: &[&str]) -> i64 {
    let m = codes.iter().filter_map(|c| crate::allergens::EU14.iter().position(|e| e == c)).fold(0i64, |m, i| m | 1 << i);
    if m == 0 {
        0
    } else {
        m | UNDECLARED
    }
}

/// The two bit planes of one dimension's 1..=3 intensities.
fn planes(s: &sense::Sense, d: Dim) -> (i64, i64) {
    d.words().iter().enumerate().fold((0, 0), |(lo, hi), (i, w)| {
        let n = s.map(d).get(*w).copied().unwrap_or(0);
        (lo | (n & 1) << i, hi | (n >> 1 & 1) << i)
    })
}

/// The `taste` block of a catalogue (`Catalog::products()`: `(id, product JSON)`). A product whose
/// JSON does not parse is left out and named in the second value (readers fall back to JSON).
pub fn project(products: &[(String, String)]) -> Result<(Block, Vec<String>), Refusal> {
    let mut cols: Vec<Vec<i64>> = vec![Vec::new(); 12];
    let (mut dish, mut bytes, mut off, mut skipped) = (Vec::new(), Vec::new(), vec![0u32], Vec::new());
    for (id, json) in products {
        let Ok(v) = serde_json::from_str::<Value>(json) else {
            skipped.push(id.clone());
            continue;
        };
        let s = sense::of_product(&v).unwrap_or_default();
        dish.push(schema::k64(id.as_bytes()) as i64);
        bytes.extend_from_slice(id.as_bytes());
        off.push(u32::try_from(bytes.len()).map_err(|_| Refusal::TooBig { len: bytes.len() })?);
        for (a, w) in sense::TASTE.iter().enumerate() {
            cols[a].push(s.taste.get(*w).copied().unwrap_or(0));
        }
        let ((tl, th), (al, ah)) = (planes(&s, Dim::Texture), planes(&s, Dim::Aroma));
        for (c, x) in [tl, th, al, ah, allergen_mask(json), i64::from(v.get("available") != Some(&Value::Bool(false)))].into_iter().enumerate() {
            cols[6 + c].push(x);
        }
    }
    let n = u32::try_from(dish.len()).map_err(|_| Refusal::TooBig { len: dish.len() })?;
    let mut all = vec![Col::I64(dish), Col::Bytes(bytes), Col::U32(off)];
    all.extend(cols.into_iter().map(Col::I64));
    Ok((Block { schema: &schema::TASTE, n, nnz: 0, cols: all }, skipped))
}

/// The dish vector as `sense::vector` writes it (`t:` level*1000/5, `x:`/`a:` intensity*1000/3).
pub fn vector_at(v: &View, row: usize) -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    for (a, w) in sense::TASTE.iter().enumerate() {
        let n = v.i64_at(AXIS0 + a, row).unwrap_or(0);
        if n > 0 {
            out.insert(format!("t:{w}"), n * sense::SCALE / TASTE_MAX);
        }
    }
    for (d, lo, hi) in [(Dim::Texture, TEX_LO, TEX_HI), (Dim::Aroma, ARO_LO, ARO_HI)] {
        let (l, h) = (v.i64_at(lo, row).unwrap_or(0), v.i64_at(hi, row).unwrap_or(0));
        for (i, w) in d.words().iter().enumerate() {
            let n = (l >> i & 1) + 2 * (h >> i & 1);
            if n > 0 {
                out.insert(format!("{}:{w}", d.prefix()), n * sense::SCALE / TAG_MAX);
            }
        }
    }
    out
}

/// Best first, ties by the id's bytes; the first `k`.
fn best(mut v: Vec<(String, i64)>, k: usize) -> Vec<(String, i64)> {
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v.truncate(k);
    v
}

/// `want` as dense integers in `sense::all_keys()` order: 6 taste, 9 texture, 12 aroma. Keys
/// outside the vocabulary have no slot (they still count in `|want|^2`, see `top_k`).
pub fn dense(want: &BTreeMap<String, i64>) -> [i64; 27] {
    let mut out = [0i64; 27];
    for (k, x) in want {
        let Some((p, w)) = k.split_once(':') else { continue };
        let (base, words): (usize, &[&str]) = match p {
            "t" => (0, &sense::TASTE),
            "x" => (6, &sense::TEXTURE),
            "a" => (15, &sense::AROMA),
            _ => continue,
        };
        if let Some(i) = words.iter().position(|v| *v == w) {
            out[base + i] = *x;
        }
    }
    out
}

/// The block's i64 columns as byte slices, read once per query (`decode::check` already held
/// every one to `n * 8` bytes).
struct Cols<'a>([&'a [u8]; 15]);

impl Cols<'_> {
    #[inline]
    fn at(&self, col: usize, row: usize) -> i64 {
        let b = &self.0[col][row * 8..row * 8 + 8];
        i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    }
}

/// `rank::cos_pm(want, vector_at(v, row))` without building the vector: the same three integer
/// sums (`dot`, `|want|^2`, `|dish|^2`), read straight from the columns.
fn cos_row(c: &Cols, row: usize, w: &[i64; 27], na: i64) -> i64 {
    let (mut dot, mut nb) = (0i64, 0i64);
    for a in 0..6 {
        let n = c.at(AXIS0 + a, row);
        if n > 0 {
            let x = n * sense::SCALE / TASTE_MAX;
            dot += w[a] * x;
            nb += x * x;
        }
    }
    for (base, lo, hi, len) in [(6, TEX_LO, TEX_HI, 9), (15, ARO_LO, ARO_HI, 12)] {
        let (l, h) = (c.at(lo, row), c.at(hi, row));
        let mut bits = (l | h) & ((1 << len) - 1);
        while bits != 0 {
            let i = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            let x = ((l >> i & 1) + 2 * (h >> i & 1)) * sense::SCALE / TAG_MAX;
            dot += w[base + i] * x;
            nb += x * x;
        }
    }
    if nb == 0 {
        return 0;
    }
    dot * 1000 / crate::rank::isqrt(na * nb)
}

/// The top `k` dishes of a `taste` block for the vector `want`, none meeting `avoid`.
pub fn top_k(v: &View, want: &BTreeMap<String, i64>, avoid: i64, k: usize) -> Result<Vec<(String, i64)>, Refusal> {
    if v.layout.known.schema != &schema::TASTE {
        return Err(Refusal::Mismatch("schema: want taste"));
    }
    let w = dense(want);
    // |want|^2 over EVERY key of `want`, as `cos_pm` sums it (a key outside the vocabulary counts too).
    let na: i64 = want.values().map(|x| x * x).sum();
    if na == 0 || k == 0 {
        return Ok(Vec::new());
    }
    let n = v.n();
    let mut slices: [&[u8]; 15] = [&[]; 15];
    for (i, sl) in slices.iter_mut().enumerate() {
        let &(off, len) = v.layout.cols.get(i).ok_or(Refusal::Mismatch("taste columns"))?;
        if i != 1 && i != 2 && len != n * 8 {
            return Err(Refusal::BadLength { col: "taste" });
        }
        *sl = &v.bytes[off..off + len];
    }
    let c = Cols(slices);
    let mut hits: Vec<(i64, usize)> = Vec::with_capacity(n);
    for row in 0..n {
        if c.at(FLAGS, row) & ON_SALE == 0 || c.at(ALLERGENS, row) & avoid != 0 {
            continue;
        }
        let s = cos_row(&c, row, &w, na);
        if s > 0 {
            hits.push((s, row));
        }
    }
    let id = |r: usize| v.str_at(r).unwrap_or("");
    hits.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| id(a.1).cmp(id(b.1))));
    hits.truncate(k);
    hits.into_iter().map(|(s, r)| Ok((v.str_at(r).ok_or(Refusal::BadOffsets { col: "ids_off" })?.to_string(), s))).collect()
}

/// The same question over the product JSON -- what the block replaces, and its oracle.
pub fn top_k_json(products: &[(String, String)], want: &BTreeMap<String, i64>, avoid: i64, k: usize) -> Vec<(String, i64)> {
    let out = products
        .iter()
        .filter_map(|(id, json)| {
            let v: Value = serde_json::from_str(json).ok()?;
            if v.get("available") == Some(&Value::Bool(false)) || allergen_mask(json) & avoid != 0 {
                return None;
            }
            let s = crate::rank::cos_pm(want, &sense::of_product(&v).map(|s| sense::vector(&s)).unwrap_or_default());
            (s > 0).then(|| (id.clone(), s))
        })
        .collect();
    best(out, k)
}

#[cfg(test)]
#[path = "taste_tests.rs"]
mod tests;
