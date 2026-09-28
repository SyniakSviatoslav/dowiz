//! ZERO-COPY READS: a field is one bounds check and one `from_le_bytes` (§B.1).
//!
//! `View` is a block `decode::check` accepted, read in place. `Catalogue` is the
//! three §B.4 catalogue blocks read together: `bom_of` from the block (what
//! `stock::bom_of_product` prefers over the JSON), checked pricing (B-3), and
//! the availability kernel with its scalar oracle (B-4, B-5, B-8).

use std::collections::HashMap;

use super::decode::{check, i64_at, u32_at, Layout};
use super::encode::NO_RATE;
use super::{schema, Refusal};
use crate::stock::BomLine;

/// `menu_prices` columns, in schema order (`schema_table_agrees` holds the names).
pub const DISH: usize = 0;
pub const PRICE: usize = 1;
pub const TAX_PPM: usize = 2;
pub const MODS_PTR: usize = 3;
/// `bom` columns.
pub const DISH_PTR: usize = 0;
pub const SUPPLY: usize = 1;
pub const QTY: usize = 2;
/// `names` columns.
pub const ID: usize = 0;
pub const BYTES: usize = 1;
pub const OFF: usize = 2;

/// A validated block, read in place.
#[derive(Debug, Clone)]
pub struct View<'a> {
    pub bytes: &'a [u8],
    pub layout: Layout,
}

impl<'a> View<'a> {
    /// Validate once (`decode::check`); every read after is a bounds check.
    pub fn new(bytes: &'a [u8]) -> Result<View<'a>, Refusal> {
        Ok(View { bytes, layout: check(bytes)? })
    }

    pub fn n(&self) -> usize {
        self.layout.n as usize
    }

    /// Element `i` of an 8-byte column, or `None` past its end.
    pub fn i64_at(&self, col: usize, i: usize) -> Option<i64> {
        let &(off, len) = self.layout.cols.get(col)?;
        (i < len / 8).then(|| i64_at(self.bytes, off + 8 * i))
    }

    /// Element `i` of a 4-byte column, or `None` past its end.
    pub fn u32_at(&self, col: usize, i: usize) -> Option<u32> {
        let &(off, len) = self.layout.cols.get(col)?;
        (i < len / 4).then(|| u32_at(self.bytes, off + 4 * i))
    }

    /// String `i` of a text block (`names`: bytes + offsets).
    pub fn str_at(&self, i: usize) -> Option<&'a str> {
        let (boff, blen) = *self.layout.cols.get(BYTES)?;
        let (s, e) = (self.u32_at(OFF, i)? as usize, self.u32_at(OFF, i + 1)? as usize);
        if s > e || e > blen {
            return None;
        }
        std::str::from_utf8(&self.bytes[boff + s..boff + e]).ok()
    }

    fn is(&self, s: &schema::Schema) -> bool {
        self.layout.known.schema == s
    }
}

/// The catalogue projection's three blocks, read together.
#[derive(Debug, Clone)]
pub struct Catalogue<'a> {
    pub menu: View<'a>,
    pub bom: View<'a>,
    pub names: View<'a>,
    /// dish `K64` -> row; names `K64` -> row. Built once per catalogue.
    dish_rows: HashMap<u64, usize>,
    name_rows: HashMap<u64, usize>,
}

/// Every value of a row-index column is a row of `names` (a cross-block offset check).
fn rows_in(v: &View, col: usize, names: usize, name: &'static str) -> Result<(), Refusal> {
    let count = v.layout.nnz as usize;
    match (0..count).all(|j| v.u32_at(col, j).is_some_and(|r| (r as usize) < names)) {
        true => Ok(()),
        false => Err(Refusal::BadOffsets { col: name }),
    }
}

impl<'a> Catalogue<'a> {
    /// Validate the three blocks and that they describe the same rows.
    pub fn new(menu: &'a [u8], bom: &'a [u8], names: &'a [u8]) -> Result<Catalogue<'a>, Refusal> {
        let (menu, bom, names) = (View::new(menu)?, View::new(bom)?, View::new(names)?);
        if !menu.is(&schema::MENU_PRICES) || !bom.is(&schema::BOM) || !names.is(&schema::NAMES) {
            return Err(Refusal::Mismatch("schemas: want menu_prices, bom, names"));
        }
        if menu.n() != bom.n() {
            return Err(Refusal::Mismatch("menu_prices and bom have different rows"));
        }
        rows_in(&menu, MODS_PTR + 1, names.n(), "mods_col")?;
        rows_in(&bom, SUPPLY, names.n(), "supply")?;
        let key_rows = |v: &View, col: usize| (0..v.n()).filter_map(|r| Some((v.i64_at(col, r)? as u64, r))).collect::<HashMap<_, _>>();
        let (dish_rows, name_rows) = (key_rows(&menu, DISH), key_rows(&names, ID));
        Ok(Catalogue { menu, bom, names, dish_rows, name_rows })
    }

    /// The row of `product_id`: found by `K64`, CONFIRMED by its bytes in `names` (RT K-1).
    pub fn row_of(&self, product_id: &str) -> Option<usize> {
        let k = schema::k64(product_id.as_bytes());
        let row = *self.dish_rows.get(&k)?;
        let named = *self.name_rows.get(&k)?;
        (self.names.str_at(named)? == product_id).then_some(row)
    }

    /// The product's recipe from the `bom` block; `None` when the block does not have it.
    pub fn bom_of(&self, product_id: &str) -> Option<Vec<BomLine>> {
        let row = self.row_of(product_id)?;
        let (s, e) = (self.bom.u32_at(DISH_PTR, row)? as usize, self.bom.u32_at(DISH_PTR, row + 1)? as usize);
        (s..e)
            .map(|j| {
                let supply = self.names.str_at(self.bom.u32_at(SUPPLY, j)? as usize)?;
                Some(BomLine { supply: supply.to_string(), qty: self.bom.i64_at(QTY, j)? })
            })
            .collect()
    }

    /// One unit's gross price in minor units, CHECKED (B-3). `inclusive`: the
    /// venue's prices already include tax. `default_ppm` stands in for a
    /// product with no rate of its own (`NO_RATE`). The tax is the money law's
    /// (`dowiz_core::tax::tax_of`, i128 inside); the sum is `checked_add`.
    pub fn unit_gross(&self, row: usize, default_ppm: i64, inclusive: bool) -> Result<i64, Refusal> {
        let price = self.menu.i64_at(PRICE, row).ok_or(Refusal::Mismatch("no such row"))?;
        if inclusive {
            return Ok(price);
        }
        let own = self.menu.i64_at(TAX_PPM, row).ok_or(Refusal::Mismatch("no such row"))?;
        let rate = if own == NO_RATE { default_ppm } else { own };
        let rate = u32::try_from(rate).ok().filter(|r| *r <= dowiz_core::tax::RatePpm::MAX).ok_or(Refusal::BadRate { col: "tax_ppm" })?;
        let tax = dowiz_core::tax::tax_of(price, dowiz_core::tax::RatePpm(rate), false).map_err(|_| Refusal::Overflow { col: "price" })?;
        price.checked_add(tax).ok_or(Refusal::Overflow { col: "price" })
    }
}

/// Which dishes cannot be made `portions` times from `stock` (indexed by supply
/// row) -- the SCALAR ORACLE (B-4): a branch per line, stop at the first short.
/// A zero-quantity line needs nothing. Availability is asked as `C·M ≤ Q`,
/// never by dividing (B-5).
pub fn short_scalar(rp: &[u32], col: &[u32], val: &[i64], stock: &[i64], portions: i64) -> Vec<bool> {
    let mut out = Vec::with_capacity(rp.len().saturating_sub(1));
    for d in 0..rp.len().saturating_sub(1) {
        let mut short = false;
        for j in rp[d] as usize..rp[d + 1] as usize {
            let have = stock.get(col[j] as usize).copied().unwrap_or(0);
            if val[j] != 0 && val[j].saturating_mul(portions) > have {
                short = true;
                break;
            }
        }
        out.push(short);
    }
    out
}

/// The same question, branch-free over the whole CSR column: both arms always,
/// a mask selects (`bad |= (coef != 0) & (coef·q > stock)`, §B.7). A per-catalogue
/// pass, so B-8 allows it; its outputs MUST equal `short_scalar`'s (B-4). Scalar
/// code, no SIMD (operator C-1).
pub fn short_mask(rp: &[u32], col: &[u32], val: &[i64], stock: &[i64], portions: i64) -> Vec<bool> {
    let bad: Vec<u8> = (0..val.len())
        .map(|j| {
            let have = stock.get(col[j] as usize).copied().unwrap_or(0);
            u8::from(val[j] != 0) & u8::from(val[j].saturating_mul(portions) > have)
        })
        .collect();
    rp.windows(2).map(|w| bad[w[0] as usize..w[1] as usize].iter().fold(0u8, |acc, b| acc | b) != 0).collect()
}
