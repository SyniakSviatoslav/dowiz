//! COLUMNS -> BYTES, and the catalogue projection from product JSON.
//!
//! `encode` writes the canonical form `decode::check` accepts (§B.2): header,
//! descriptors, columns at the first 8-aligned byte after the previous one
//! with zero padding, crc32. It runs `check` on its own output before
//! returning, so the encoder never hands out a block a reader would refuse.
//!
//! `project` folds a catalogue (`Catalog::products()`: `(id, product JSON)`)
//! into the three §B.4 blocks, reading each product with the SAME readers the
//! JSON paths use -- `price` as the pricer reads it (a non-negative integer),
//! `modifiers::groups_of` for the deltas, `stock::bom_of` for the recipe -- so
//! the block and the JSON cannot disagree about what a product says.

use std::collections::HashMap;

use serde_json::Value;

use super::schema::{self, Schema};
use super::{header_size, ty, Block, Col, Refusal, ALIGN, COLDESC, CRC, FIXED, MAGIC, MAX_BLOCK, VERSION};

/// `tax_ppm` of a product with no `vat_ppm` of its own: the venue default applies.
/// Never a rate (`RatePpm` is `0..=1_000_000`), so it cannot be read as one.
pub const NO_RATE: i64 = -1;

/// The catalogue as the three §B.4 blocks. Rows of `menu_prices` and `bom`
/// are the same products in the same order; `mods_col` and `supply` are rows
/// of `names`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    pub menu_prices: Block,
    pub bom: Block,
    pub names: Block,
    /// Products left out because their record does not read (unparseable
    /// JSON, no non-negative integer `price`, a `vat_ppm` that is not a rate).
    /// Readers fall back to the JSON for them; the list is here so a caller
    /// can say so rather than lose them quietly.
    pub skipped: Vec<String>,
}

fn col_matches(typ: u8, col: &Col) -> bool {
    matches!(
        (typ, col),
        (ty::I64 | ty::CSR_VAL, Col::I64(_)) | (ty::I32, Col::I32(_)) | (ty::BYTES, Col::Bytes(_)) | (ty::OFFSETS | ty::ROW_PTR | ty::CSR_COL, Col::U32(_))
    )
}

fn put_col(out: &mut Vec<u8>, col: &Col) {
    match col {
        Col::I64(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
        Col::I32(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
        Col::Bytes(v) => out.extend_from_slice(v),
        Col::U32(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
    }
}

/// Write `b` as bytes (§B.2). Refuses what `decode::check` would refuse.
pub fn encode(b: &Block) -> Result<Vec<u8>, Refusal> {
    let known = schema::of(b.schema).ok_or(Refusal::UnknownSchema(schema::k64(b.schema.string.as_bytes())))?;
    let ncols = known.cols.len();
    if b.cols.len() != ncols {
        return Err(Refusal::BadNcols { got: b.cols.len(), want: ncols });
    }
    let mut out = vec![0u8; header_size(ncols)];
    out[..4].copy_from_slice(&MAGIC);
    out[4..6].copy_from_slice(&VERSION.to_le_bytes());
    out[6..8].copy_from_slice(&(ncols as u16).to_le_bytes());
    out[8..12].copy_from_slice(&b.n.to_le_bytes());
    out[12..16].copy_from_slice(&b.nnz.to_le_bytes());
    out[16..FIXED].copy_from_slice(&known.k64.to_le_bytes());
    for (i, (spec, col)) in known.cols.iter().zip(&b.cols).enumerate() {
        if !col_matches(spec.typ, col) {
            return Err(Refusal::BadType { col: spec.name });
        }
        out.resize(out.len().div_ceil(ALIGN) * ALIGN, 0);
        let off = out.len();
        put_col(&mut out, col);
        if out.len() + CRC > MAX_BLOCK {
            return Err(Refusal::TooBig { len: out.len() + CRC });
        }
        let at = header_size(i);
        out[at] = spec.typ;
        out[at + 2] = spec.unit;
        let clen = (out.len() - off) as u32;
        out[at + 4..at + 8].copy_from_slice(&clen.to_le_bytes());
        out[at + 8..at + COLDESC - 4].copy_from_slice(&(off as u32).to_le_bytes());
    }
    let crc = super::crc32(&out);
    out.extend_from_slice(&crc.to_le_bytes());
    super::decode::check(&out)?;
    Ok(out)
}

fn count(n: usize) -> Result<u32, Refusal> {
    u32::try_from(n).map_err(|_| Refusal::TooBig { len: n })
}

/// Strings interned by row, keyed by their `K64`; two strings with one key are refused (RT K-1).
#[derive(Default)]
struct Interner {
    strings: Vec<String>,
    rows: HashMap<String, u32>,
    keys: HashMap<u64, u32>,
}

impl Interner {
    fn row(&mut self, s: &str) -> Result<u32, Refusal> {
        if let Some(r) = self.rows.get(s) {
            return Ok(*r);
        }
        let k = schema::k64(s.as_bytes());
        let row = count(self.strings.len())?;
        if let Some(other) = self.keys.insert(k, row) {
            return Err(Refusal::KeyCollision { a: self.strings[other as usize].clone(), b: s.to_string() });
        }
        self.strings.push(s.to_string());
        self.rows.insert(s.to_string(), row);
        Ok(row)
    }

    fn block(self) -> Result<Block, Refusal> {
        let ids = self.strings.iter().map(|s| schema::k64(s.as_bytes()) as i64).collect();
        let mut bytes = Vec::new();
        let mut off = vec![0u32];
        for s in &self.strings {
            bytes.extend_from_slice(s.as_bytes());
            off.push(count(bytes.len())?);
        }
        let n = count(self.strings.len())?;
        Ok(Block { schema: &schema::NAMES, n, nnz: 0, cols: vec![Col::I64(ids), Col::Bytes(bytes), Col::U32(off)] })
    }
}

/// A product's price and rate as the pricer reads them, or `None` to skip it.
fn price_and_rate(json: &str) -> Option<(i64, i64)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let price = v.get("price").and_then(Value::as_i64).filter(|p| *p >= 0)?;
    let rate = match v.get("vat_ppm") {
        None | Some(Value::Null) => NO_RATE,
        Some(r) => r.as_i64().filter(|r| (0..=dowiz_core::tax::PPM).contains(r))?,
    };
    Some((price, rate))
}

/// The catalogue projection: `menu_prices`, `bom` and `names` (§B.4).
pub fn project(products: &[(String, String)]) -> Result<Projection, Refusal> {
    let mut names = Interner::default();
    let (mut dish, mut price, mut tax) = (Vec::new(), Vec::new(), Vec::new());
    let (mut mods_ptr, mut mods_col, mut mods_val) = (vec![0u32], Vec::new(), Vec::new());
    let (mut dish_ptr, mut supply, mut qty) = (vec![0u32], Vec::new(), Vec::new());
    let mut skipped = Vec::new();
    for (id, json) in products {
        let Some((p, rate)) = price_and_rate(json) else {
            skipped.push(id.clone());
            continue;
        };
        names.row(id)?;
        dish.push(schema::k64(id.as_bytes()) as i64);
        price.push(p);
        tax.push(rate);
        for option in crate::modifiers::groups_of(json).into_iter().flat_map(|g| g.options) {
            mods_col.push(names.row(&option.id)?);
            mods_val.push(option.price_delta);
        }
        mods_ptr.push(count(mods_col.len())?);
        for line in crate::stock::bom_of(json) {
            supply.push(names.row(&line.supply)?);
            qty.push(line.qty);
        }
        dish_ptr.push(count(supply.len())?);
    }
    let n = count(dish.len())?;
    let menu_prices = Block {
        schema: &schema::MENU_PRICES,
        n,
        nnz: count(mods_col.len())?,
        cols: vec![Col::I64(dish), Col::I64(price), Col::I64(tax), Col::U32(mods_ptr), Col::U32(mods_col), Col::I64(mods_val)],
    };
    let bom = Block { schema: &schema::BOM, n, nnz: count(supply.len())?, cols: vec![Col::U32(dish_ptr), Col::U32(supply), Col::I64(qty)] };
    Ok(Projection { menu_prices, bom, names: names.block()?, skipped })
}

/// A `stock_levels` block (§B.4 row 3) from `(supply K64, qty, generation)` rows.
pub fn stock_levels(levels: &[(i64, i64, i64)]) -> Block {
    let pick = |f: fn(&(i64, i64, i64)) -> i64| Col::I64(levels.iter().map(f).collect());
    let schema: &'static Schema = &schema::STOCK_LEVELS;
    Block { schema, n: levels.len() as u32, nnz: 0, cols: vec![pick(|l| l.0), pick(|l| l.1), pick(|l| l.2)] }
}
