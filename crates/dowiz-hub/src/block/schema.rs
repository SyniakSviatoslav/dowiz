//! THE ONE TABLE OF BLOCK SCHEMAS (§B.4, B-2).
//!
//! A schema string is ASCII, `name:v1(col:type:unit,...)`; the block header's
//! `schema` field (§B.2 offset 16) is its `K64`. Every column's §B.2 type code
//! and unit are DERIVED from the string by `parse`, so there is no second hand
//! copy of the layout to drift from it. Mirrors: `bebop-lang/selfhost/std/block.bp`
//! and `crates/bebop-wasm/oracle.py` (row DG9) must carry these same strings.
//!
//! TYPE TOKENS -> §B.2 codes: `i64` 1, `i32` 2, `u8` 3, `u32off` 4, `u32rp` 5,
//! `u32` 6 (CSR col). An `i64` that directly follows a `u32` (CSR col) is the
//! CSR `val` column, code 7 -- §B.3's "CSR (`row_ptr`, `col`, `val`)" triple.

use std::sync::OnceLock;

/// One schema of the table.
#[derive(Debug, PartialEq, Eq)]
pub struct Schema {
    pub name: &'static str,
    pub string: &'static str,
}

/// §B.4 row 1: the catalogue's prices, tax rates and modifier deltas.
pub const MENU_PRICES: Schema = Schema {
    name: "menu_prices",
    string: "menu_prices:v1(dish:i64:0,price:i64:1,tax_ppm:i64:5,mods_ptr:u32rp:0,mods_col:u32:0,mods_val:i64:1)",
};

/// §B.4 row 2: the bills of materials. `qty`'s unit is 0 (none): the column
/// spans supplies of different base units, and §B.4's `<unit of the supply>`
/// cannot be one byte for all of them -- each supply record carries its unit.
pub const BOM: Schema = Schema { name: "bom", string: "bom:v1(dish_ptr:u32rp:0,supply:u32:0,qty:i64:0)" };

/// §B.4 row 3: stock levels (for `unavailable(D)`, DC A.6). `qty` unit 0, as `bom`.
pub const STOCK_LEVELS: Schema =
    Schema { name: "stock_levels", string: "stock_levels:v1(supply:i64:0,qty:i64:0,gen:i64:7)" };

/// §B.4 row 4: the intern table -- every id a block names by row.
pub const NAMES: Schema = Schema { name: "names", string: "names:v1(id:i64:0,bytes:u8:0,off:u32off:0)" };

/// Every shipped schema. A reader refuses a block whose schema is not here.
pub const TABLE: [&Schema; 4] = [&MENU_PRICES, &BOM, &STOCK_LEVELS, &NAMES];

/// One column as the schema string declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColSpec {
    pub name: &'static str,
    pub typ: u8,
    pub unit: u8,
}

/// A table entry with its derived key and columns.
#[derive(Debug)]
pub struct Known {
    pub schema: &'static Schema,
    pub k64: u64,
    pub cols: Vec<ColSpec>,
    /// Does the schema carry a CSR column (then `nnz` may be non-zero)?
    pub csr: bool,
}

/// RT §2.2: `K64 = (crc32x(bytes) << 32) | (len & 0xffffffff)`. An INDEX, never
/// a proof (RT K-1): blocks confirm ids by their bytes (`view::Catalogue::row_of`).
pub fn k64(bytes: &[u8]) -> u64 {
    (u64::from(super::crc32(bytes)) << 32) | (bytes.len() as u64 & 0xffff_ffff)
}

/// Parse a schema string into its columns; `Err` names the first bad token.
pub fn parse(s: &'static str) -> Result<Vec<ColSpec>, String> {
    let open = s.find("(").ok_or("no `(`")?;
    let head = &s[..open];
    if !head.contains(":v") || !s.ends_with(')') {
        return Err(format!("`{s}` is not name:vN(...)"));
    }
    let mut out: Vec<ColSpec> = Vec::new();
    for part in s[open + 1..s.len() - 1].split(',') {
        let mut f = part.split(':');
        let (Some(name), Some(tok), Some(unit), None) = (f.next(), f.next(), f.next(), f.next()) else {
            return Err(format!("column `{part}` is not col:type:unit"));
        };
        let after_col = out.last().is_some_and(|c| c.typ == super::ty::CSR_COL);
        let typ = match tok {
            "i64" if after_col => super::ty::CSR_VAL,
            "i64" => super::ty::I64,
            "i32" => super::ty::I32,
            "u8" => super::ty::BYTES,
            "u32off" => super::ty::OFFSETS,
            "u32rp" => super::ty::ROW_PTR,
            "u32" => super::ty::CSR_COL,
            other => return Err(format!("column `{name}`: unknown type `{other}`")),
        };
        let unit: u8 = unit.parse().map_err(|_| format!("column `{name}`: unit `{unit}` is not a number"))?;
        if unit > super::unit::GENERATION {
            return Err(format!("column `{name}`: unit {unit} is not in §B.2"));
        }
        out.push(ColSpec { name, typ, unit });
    }
    Ok(out)
}

/// The table, parsed once. A schema string that does not parse is a build
/// defect, caught by `schema_table_agrees`, and here it panics on first use.
pub fn known() -> &'static [Known] {
    static KNOWN: OnceLock<Vec<Known>> = OnceLock::new();
    KNOWN.get_or_init(|| {
        TABLE
            .iter()
            .map(|s| {
                let cols = parse(s.string).unwrap_or_else(|e| panic!("schema {}: {e}", s.name));
                let csr = cols.iter().any(|c| c.typ == super::ty::ROW_PTR);
                Known { schema: *s, k64: k64(s.string.as_bytes()), cols, csr }
            })
            .collect()
    })
}

/// The table entry whose `K64` is `key`.
pub fn by_k64(key: u64) -> Option<&'static Known> {
    known().iter().find(|k| k.k64 == key)
}

/// The table entry for `schema`; `None` for a schema that is not in `TABLE`.
pub fn of(schema: &Schema) -> Option<&'static Known> {
    known().iter().find(|k| k.schema == schema)
}
