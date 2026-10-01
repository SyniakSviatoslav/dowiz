//! WHAT A DISH IS MADE OF: its recipe read from the catalogue, the reservations an
//! order's lines imply, and how they settle into consumption or release.

use super::*;

// ── what a dish is made of ──────────────────────────────────────────────────

/// One line of a dish's bill of materials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomLine {
    pub supply: String,
    /// How much ONE portion uses, in the supply's base unit -- WHOLE units;
    /// for a leaf of a semi-finished tree, `uq` rounded to the nearest.
    pub qty: Qty,
    /// The same in MILLIONTHS of the base unit (`prep::MICRO`): exact for a
    /// leaf expanded through a semi-finished card (`prep::for_ledger` writes
    /// `{"supply","uq"}`), `qty × 10^6` for a whole line. What `draws_for`
    /// books and `cost::CostBook::dish_cost` prices.
    pub uq: i64,
}

impl BomLine {
    /// A whole-unit line, as every stored dish writes it.
    pub fn whole(supply: impl Into<String>, qty: Qty) -> BomLine {
        BomLine { supply: supply.into(), qty, uq: qty.saturating_mul(prep_micro()) }
    }
}

/// `prep::MICRO`, named here so this file's readers do not import the tree.
const fn prep_micro() -> i64 {
    crate::prep::MICRO
}

/// Read a product's recipe out of its catalogue record.
///
/// A product with no `bom` is not an error and not a problem: plenty of things
/// a venue sells -- a bottle of water, a dessert bought in -- have no recipe
/// worth tracking, and those simply never reserve anything. Stock control that
/// demands every item be modelled before any item can be sold is stock control
/// nobody switches on.
pub fn bom_of(product_json: &str) -> Vec<BomLine> {
    let mut out = Vec::new();
    // `"bom":[{"supply":"salmon","qty":40}, ...]` -- the array `"bom"` holds,
    // brackets walked outside strings (`modifiers::array_of`, W-AUDIT S7).
    let Some(body) = crate::modifiers::array_of(product_json, "bom") else { return out };
    for chunk in crate::modifiers::split_objects(body) {
        let Some(supply) = crate::minijson::str_field(&chunk, "supply") else { continue };
        if supply.is_empty() {
            continue;
        }
        // A leaf line (`uq`, millionths) or a whole line (`qty`). A line with
        // neither, or a fraction written as `qty`, is not a line.
        if let Some(uq) = crate::minijson::int_field(&chunk, "uq") {
            if uq > 0 {
                out.push(BomLine { supply, qty: (uq + prep_micro() / 2).div_euclid(prep_micro()), uq });
            }
            continue;
        }
        let Some(qty) = crate::minijson::int_field(&chunk, "qty") else { continue };
        if qty > 0 {
            out.push(BomLine::whole(supply, qty));
        }
    }
    out
}

/// `bom_of`, read from the catalogue's `bom` block when one is present (row DG7).
///
/// The block is the catalogue projection (`block::encode::project`), rebuilt
/// with every catalogue generation; a product it does not have -- one written
/// after the block was folded -- falls back to its JSON, which stays the
/// writers' format. Both readers give equal lines (`block::tests::bom_of_block_equals_json`).
pub fn bom_of_product(blocks: Option<&crate::block::view::Catalogue<'_>>, product_id: &str, product_json: &str) -> Vec<BomLine> {
    match blocks.and_then(|c| c.bom_of(product_id)) {
        Some(lines) => lines,
        None => bom_of(product_json),
    }
}

/// The stock events one order's lines imply.
///
/// `lines` is `(product_json, quantity_ordered)`. Quantities MULTIPLY: two
/// portions of a roll using forty grams of salmon reserve eighty, and getting
/// that wrong is how a kitchen runs out mid-service while the ledger says it is
/// fine.
///
/// Lines for the same supply are SUMMED rather than emitted separately, so a
/// basket with two different rolls that both use salmon is checked against the
/// total it actually needs.
pub fn reservations_for(order_id: &str, lines: &[(String, i64)]) -> Vec<StockEvent> {
    let mut totals: Vec<(String, Qty)> = Vec::new();
    for (product_json, qty_ordered) in lines {
        if *qty_ordered <= 0 {
            continue;
        }
        for line in bom_of(product_json) {
            let need = line.qty.saturating_mul(*qty_ordered);
            match totals.iter_mut().find(|(s, _)| *s == line.supply) {
                Some((_, t)) => *t = t.saturating_add(need),
                None => totals.push((line.supply.clone(), need)),
            }
        }
    }
    // Sorted, so the same basket always produces the same event sequence and
    // two hubs replaying it agree byte for byte.
    totals.sort_by(|a, b| a.0.cmp(&b.0));
    totals
        .into_iter()
        .map(|(supply, qty)| StockEvent::Reserved {
            item: supply,
            qty,
            order_id: order_id.to_string(),
        })
        .collect()
}

/// Turn an order's reservations into consumption or release.
///
/// Derived from what the LEDGER is holding for that order rather than
/// recomputed from the basket: if the menu changed between placing and
/// cooking, the recipe may have too, and releasing a different quantity from
/// the one that was reserved is how a reservation gets stranded.
pub fn settle(ledger: &StockLedger, order_id: &str, consume: bool) -> Vec<StockEvent> {
    ledger
        .stranded()
        .into_iter()
        .filter(|(o, _, _)| o == order_id)
        .map(|(_, item, qty)| {
            if consume {
                StockEvent::Consumed { item, qty, order_id: order_id.to_string() }
            } else {
                StockEvent::Released { item, qty, order_id: order_id.to_string() }
            }
        })
        .collect()
}
