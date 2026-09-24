//! A dish's recipe AS THE CATALOGUE STORES IT: `{supply, qty}` per line and
//! nothing else.
//!
//! WHY SO LEAN (2026-09-24). A stored line used to repeat a snapshot of its
//! supply -- name, unit, kind, kcal, protein, fat, carbs, cost, weight -- about
//! 159 bytes a line, and the catalogue image spends a cell per byte. A venue
//! of 165 dishes with 73 recipes modelled to 1401 per mille of its ceiling and
//! could not be saved at all; the same recipes as `{supply, qty}` fit.
//!
//! Nothing needed the snapshot. The ledger reads `supply` and `qty` only
//! (`stock::bom_of`); a dish's cost AT SALE is stamped from the ledger's own
//! purchase fold (`stock::cost::Stamp`), not from the recipe; and the console
//! re-scaled every loaded line from today's supplies before drawing it. The
//! Worker derives the names and numbers from the supplies as they are now,
//! on read (`recipe::lines_of_stored`), and still reads a snapshot-form line
//! stored before this change.

use crate::minijson::esc;
use crate::stock::BomLine;

/// The stored `bom` array. `stock::bom_of` reads it back line for line.
pub fn to_json(lines: &[BomLine]) -> String {
    let parts: Vec<String> =
        lines.iter().map(|l| format!(r#"{{"supply":"{}","qty":{}}}"#, esc(&l.supply), l.qty)).collect();
    format!("[{}]", parts.join(","))
}
