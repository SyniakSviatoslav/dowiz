//! The DG7 block VIEW, twinned for wasm32 (row BN3, menu half): a block read IN PLACE.
//!
//! `dowiz_hub::block::view::View` is how the hub reads a block: validate once (`check`),
//! then every field is one bounds check and one `from_le_bytes`, nothing copied out. This is
//! the same shape on this crate's own checker (`block::check`, DG9's independent reader), so
//! the browser can read the `.dwb` blocks the venue publishes (`menu_prices`, `names`, BN2)
//! the way the hub does -- without `decode`'s copy of every column.
//!
//! WHAT gate.sh COMPARES. `line(b)` prints the decode line's first half,
//!   block <schema> n=<n> nnz=<nnz> vals=<fnv64>      or      block refused=<code> col=<col>
//! but `vals` is folded from IN-PLACE reads (`View::at`), never from `decode`'s vectors, so
//! gate.sh can hold it against the four decode readers' agreed line (minus ` rt=.. k256=..`)
//! on every fixture, 4,000 random blocks and every corrupted block of `--prove`.
//!
//! `Menu` is the hub's `Catalogue::row_of` for the two PUBLISHED blocks (the venue's `bom`
//! is not public): a product's row found by its `K64` and CONFIRMED by its bytes in `names`
//! (RT K-1), so a key collision answers `None`, never another dish's price.
//! `menu_line` folds every row that resolves; gate.sh compares it with `oracle.py --menu`.
//! EVERYTHING REFUSES, NOTHING PANICS. Compiled only for tests and `--cfg bw_block`, like
//! `block.rs`, so the Worker's module and `bytes.baseline` carry none of it.

use crate::block::{check, k64, table, Refused, Spec};
use std::collections::BTreeMap;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
/// Column types (`block.rs`): the widths a view reads.
const I64: u8 = 1;
const I32: u8 = 2;
const BYTES: u8 = 3;
const CSR_VAL: u8 = 7;
/// `menu_prices` columns, schema order (DG7 `view.rs`).
pub const DISH: usize = 0;
pub const PRICE: usize = 1;
pub const TAX_PPM: usize = 2;
pub const MODS_COL: usize = 4;
/// `names` columns.
pub const NAME_BYTES: usize = 1;
pub const NAME_OFF: usize = 2;

fn fnv(h: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(h, |h, x| (h ^ u64::from(*x)).wrapping_mul(FNV_PRIME))
}

/// A block `check` accepted, read in place.
pub struct View<'a> {
    pub bytes: &'a [u8],
    pub spec: &'a Spec,
    pub n: u32,
    pub nnz: u32,
    lay: Vec<(usize, usize)>,
}

impl<'a> View<'a> {
    /// Validate once; every read after is a bounds check.
    pub fn new(bytes: &'a [u8], t: &'a [Spec]) -> Result<View<'a>, Refused> {
        let (si, n, nnz, lay) = check(bytes, t)?;
        Ok(View { bytes, spec: &t[si], n, nnz, lay })
    }

    pub fn name(&self) -> &'static str {
        self.spec.name
    }

    /// How many elements column `col` holds, or `None` past the last column.
    pub fn len(&self, col: usize) -> Option<usize> {
        let &(_, typ, _) = self.spec.cols.get(col)?;
        let (_, bytes) = self.lay[col];
        Some(bytes / width(typ))
    }

    /// Element `i` of column `col` as the value `decode` would have copied out, or `None`.
    pub fn at(&self, col: usize, i: usize) -> Option<i64> {
        let &(_, typ, _) = self.spec.cols.get(col)?;
        let (off, _) = self.lay[col];
        let w = width(typ);
        if i >= self.len(col)? {
            return None;
        }
        let raw = self.bytes.get(off + w * i..off + w * (i + 1))?;
        Some(match typ {
            I64 | CSR_VAL => i64::from_le_bytes(raw.try_into().ok()?),
            I32 => i64::from(i32::from_le_bytes(raw.try_into().ok()?)),
            BYTES => i64::from(raw[0]),
            _ => i64::from(u32::from_le_bytes(raw.try_into().ok()?)),
        })
    }

    /// String `i` of a `names` block, or `None` (not a text block, or past its end).
    pub fn str_at(&self, i: usize) -> Option<&'a str> {
        if self.spec.name != "names" {
            return None;
        }
        let (boff, blen) = self.lay[NAME_BYTES];
        let (s, e) = (self.at(NAME_OFF, i)? as usize, self.at(NAME_OFF, i + 1)? as usize);
        if s > e || e > blen {
            return None;
        }
        core::str::from_utf8(self.bytes.get(boff + s..boff + e)?).ok()
    }

    /// FNV-1a 64 over each column's count, then its elements, 8 LE bytes each -- `block::vals`
    /// over the same values, read in place.
    pub fn vals(&self) -> u64 {
        let mut h = FNV_OFFSET;
        for c in 0..self.spec.cols.len() {
            let len = self.len(c).unwrap_or(0);
            h = fnv(h, &(len as u64).to_le_bytes());
            for i in 0..len {
                h = fnv(h, &self.at(c, i).unwrap_or(0).to_le_bytes());
            }
        }
        h
    }
}

fn width(typ: u8) -> usize {
    match typ {
        I64 | CSR_VAL => 8,
        BYTES => 1,
        _ => 4,
    }
}

/// gate.sh's view line for one block.
pub fn line(b: &[u8]) -> String {
    let t = table();
    match View::new(b, &t) {
        Ok(v) => format!("block {} n={} nnz={} vals={:016x}", v.name(), v.n, v.nnz, v.vals()),
        Err(r) => format!("block refused={} col={}", r.code, r.col),
    }
}

/// The two published catalogue blocks, read together.
pub struct Menu<'a> {
    pub prices: View<'a>,
    pub names: View<'a>,
    /// `K64` -> row, built once per menu.
    dish_rows: BTreeMap<i64, usize>,
    name_rows: BTreeMap<i64, usize>,
}

impl<'a> Menu<'a> {
    /// Validate both blocks, their schemas, and that every modifier names a row of `names`.
    pub fn new(prices: View<'a>, names: View<'a>) -> Result<Menu<'a>, Refused> {
        if prices.name() != "menu_prices" || names.name() != "names" {
            return Err(Refused { code: "mismatch", col: "-" });
        }
        let rows = names.n as usize;
        if (0..prices.nnz as usize).any(|j| prices.at(MODS_COL, j).is_none_or(|r| r < 0 || r as usize >= rows)) {
            return Err(Refused { code: "bad_offsets", col: "mods_col" });
        }
        let keyed = |v: &View, col: usize| (0..v.n as usize).filter_map(|r| Some((v.at(col, r)?, r))).collect::<BTreeMap<_, _>>();
        let (dish_rows, name_rows) = (keyed(&prices, DISH), keyed(&names, 0));
        Ok(Menu { prices, names, dish_rows, name_rows })
    }

    /// The name row a dish key resolves to, confirmed by its bytes (RT K-1).
    fn named(&self, key: i64) -> Option<&'a str> {
        let s = self.names.str_at(*self.name_rows.get(&key)?)?;
        (k64(s.as_bytes()) as i64 == key).then_some(s)
    }

    /// The row of `product_id`: found by `K64`, CONFIRMED by its bytes.
    pub fn row_of(&self, product_id: &str) -> Option<usize> {
        let k = k64(product_id.as_bytes()) as i64;
        let row = *self.dish_rows.get(&k)?;
        (self.named(k)? == product_id).then_some(row)
    }

    /// `(price, tax_ppm)` of `product_id` in minor units, as published; `None` when absent.
    pub fn price_of(&self, product_id: &str) -> Option<(i64, i64)> {
        let row = self.row_of(product_id)?;
        Some((self.prices.at(PRICE, row)?, self.prices.at(TAX_PPM, row)?))
    }
}

/// `menu rows=<n> resolved=<k> fold=<fnv64>`: per resolved row in order, FNV over the id's
/// length (8 LE), its bytes, its price and its rate (8 LE each). `oracle.py --menu` mirrors it.
pub fn menu_line(prices: &[u8], names: &[u8]) -> String {
    let t = table();
    let menu = View::new(prices, &t).and_then(|p| Menu::new(p, View::new(names, &t)?));
    let m = match menu {
        Ok(m) => m,
        Err(r) => return format!("menu refused={} col={}", r.code, r.col),
    };
    let (mut resolved, mut h) = (0u32, FNV_OFFSET);
    for r in 0..m.prices.n as usize {
        let Some(id) = m.prices.at(DISH, r).and_then(|k| m.named(k)) else { continue };
        if m.row_of(id) != Some(r) {
            continue;
        }
        resolved += 1;
        h = fnv(h, &(id.len() as u64).to_le_bytes());
        h = fnv(h, id.as_bytes());
        h = fnv(h, &m.prices.at(PRICE, r).unwrap_or(0).to_le_bytes());
        h = fnv(h, &m.prices.at(TAX_PPM, r).unwrap_or(0).to_le_bytes());
    }
    format!("menu rows={} resolved={resolved} fold={h:016x}", m.prices.n)
}

/// Copy `s` into `out` (at most `cap` bytes): its length, or -1 when it does not fit.
unsafe fn put(s: &str, out: *mut u8, cap: usize) -> i32 {
    if s.len() > cap {
        return -1;
    }
    core::ptr::copy_nonoverlapping(s.as_ptr(), out, s.len());
    s.len() as i32
}

/// The bytes at `ptr..ptr+len`, or `None` for a null pointer with a length.
unsafe fn slice<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    match (ptr.is_null(), len) {
        (_, 0) => Some(&[]),
        (true, _) => None,
        (false, _) => Some(core::slice::from_raw_parts(ptr, len)),
    }
}

/// Write `line(ptr..ptr+len)` into `out`; its length, `-abi::NULL_ARG`, or -1 (short `cap`).
///
/// # Safety
/// `ptr..ptr+len` is readable (or `len` is 0) and `out..out+cap` is writable.
#[cfg_attr(bw_block, no_mangle)]
pub unsafe extern "C" fn bw_view(ptr: *const u8, len: usize, out: *mut u8, cap: usize) -> i32 {
    match (out.is_null(), slice(ptr, len)) {
        (false, Some(b)) => put(&line(b), out, cap),
        _ => -crate::abi::NULL_ARG,
    }
}

/// Write `menu_line(prices, names)` into `out`; statuses as `bw_view`.
///
/// # Safety
/// Both inputs are readable (or have length 0) and `out..out+cap` is writable.
#[cfg_attr(bw_block, no_mangle)]
pub unsafe extern "C" fn bw_menu(pp: *const u8, pl: usize, np: *const u8, nl: usize, out: *mut u8, cap: usize) -> i32 {
    match (out.is_null(), slice(pp, pl), slice(np, nl)) {
        (false, Some(p), Some(n)) => put(&menu_line(p, n), out, cap),
        _ => -crate::abi::NULL_ARG,
    }
}

#[cfg(test)]
mod tests;
