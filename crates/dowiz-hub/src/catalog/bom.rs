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

/// The stored `bom` array with a line's WEIGHED net and out (grams, research
/// 2026-09-26 R6) written only where the owner typed them: a line that
/// follows its supply's defaults stays `{supply, qty}`, so a recipe nobody
/// weighed costs the catalogue image not one byte more. `stock::bom_of` reads
/// `supply` and `qty` out of either form and nothing else.
pub fn to_json_weighed(lines: &[(BomLine, Option<i64>, Option<i64>)]) -> String {
    let parts: Vec<String> = lines
        .iter()
        .map(|(l, net, out)| {
            let mut s = format!(r#"{{"supply":"{}","qty":{}"#, esc(&l.supply), l.qty);
            if let Some(n) = net {
                s.push_str(&format!(r#","net":{n}"#));
            }
            if let Some(o) = out {
                s.push_str(&format!(r#","out":{o}"#));
            }
            s.push('}');
            s
        })
        .collect();
    format!("[{}]", parts.join(","))
}
