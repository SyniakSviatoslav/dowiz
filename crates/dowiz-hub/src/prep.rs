//! SEMI-FINISHED PRODUCTS (напівфабрикати, ПФ): a supply with its own card.
//! `docs/design/SPEC-SEMI-FINISHED-2026-09-29.md` is the design; this is it.
//!
//! THREE KINDS OF ITEM. A RAW supply is bought and counted (every kind in
//! `recipe::KINDS` today). A ПФ is a supply of kind [`KIND`] whose record
//! carries a [`Card`]: the components -- raw items OR other ПФ -- with their
//! GROSS quantities, and the batch's net YIELD. A DISH is a product whose
//! `bom` lines may now name a ПФ like any supply. Nothing about the dish
//! record, the `bom` block or the catalogue keys changes: the one new thing a
//! reader meets is a supply whose kind is `prep`.
//!
//! K = yield / Σ gross is DERIVED ([`k_pm`]) and never stored.
//!
//! THE WRITE-OFF IS EXACT DOWN THE TREE ([`expand`]): every path to a raw
//! leaf is a product of `qty / yield` fractions (`Rat`, i128, checked), the
//! paths to one leaf are summed, and the leaf is rounded ONCE to millionths
//! of its base unit (`uq`). The ledger books whole units from those with a
//! carried remainder (`stock::carry`), so a thousand sales of 0.77 g book
//! 774 g, not 1 000.
//!
//! A ПФ MAY BE KEPT READY (W-PF2 R2): a production act (`stock::act`) puts a
//! batch on the shelf, and a sale takes it first ([`stocked`]).

use serde_json::{json, Value};

use crate::import::recipes::num::Rat;

pub mod tree;
use tree::{Edge, Stop, Walk};

/// The supply kind of a semi-finished product.
pub const KIND: &str = "prep";
/// Cards on one path under a dish. Six leaves i128 headroom (SPEC §b) and no
/// real kitchen is deeper than three.
pub const DEPTH_MAX: usize = 6;
/// Lines on one card.
pub const LINES_MAX: usize = 40;
/// A line's gross and a card's yield: the recipe bound (`recipe::QTY_MAX`).
pub const QTY_MAX: i64 = crate::import::recipes::QTY_MAX;
/// Millionths of a base unit: the leaf quantity's scale.
pub const MICRO: i64 = 1_000_000;

/// One line of a card: an item and its gross quantity in the ITEM's unit,
/// in MILLIONTHS (W-QTYHUB H2): half a gram of saffron is `500_000`. Stored
/// whole as `{"item","qty"}` (byte for byte what every card was), a fraction
/// as `{"item","uq"}` -- the dish line's own form (CONTRACT-QTY §1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub item: String,
    pub uq: i64,
}

/// A ПФ's card as stored under `"card"` on its supply record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub lines: Vec<Line>,
    /// The batch's net output, in millionths of the ПФ's own base unit
    /// (stored `"yield"` whole, `"yieldUq"` for a fraction).
    pub yield_uq: i64,
}

/// Is this supply record a semi-finished product?
pub fn is_prep(supply_json: &str) -> bool {
    crate::minijson::str_field(supply_json, "kind").as_deref() == Some(KIND)
}

/// Is this raw item outside the shelf (water)? It stays a line of every card
/// and of K; it is never a leaf of the write-off.
pub fn untracked(supply_json: &str) -> bool {
    crate::minijson::bool_field(supply_json, "untracked") == Some(true)
}

/// The card of a ПФ record; `None` for a raw supply or a record without one.
pub fn card_of(supply_json: &str) -> Option<Card> {
    if !is_prep(supply_json) {
        return None;
    }
    let v: Value = serde_json::from_str(supply_json).ok()?;
    card_from_value(v.get("card")?)
}

/// Why a card cannot be saved or a tree cannot be expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A line names a supply the catalogue does not have.
    Unknown(String),
    /// A line names the card itself.
    Itself,
    /// A line's quantity or the yield is outside `1..=QTY_MAX`.
    Bound(String),
    /// Two lines name one item.
    Twice(String),
    /// No lines, or more than `LINES_MAX`.
    Lines,
    /// The walk stopped (cycle, depth, overflow).
    Tree(Stop),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Unknown(id) => write!(f, "unknown supply {id}"),
            Refusal::Itself => write!(f, "a semi-finished product cannot contain itself"),
            Refusal::Bound(what) => write!(f, "{what} is more than 0 and at most {QTY_MAX}"),
            Refusal::Twice(id) => write!(f, "{id} is on the card twice"),
            Refusal::Lines => write!(f, "a card has 1 to {LINES_MAX} lines"),
            Refusal::Tree(s) => write!(f, "{s}"),
        }
    }
}

/// One raw leaf of an expansion: millionths of its base unit per ONE of the
/// root (one portion of a dish, one unit of a ПФ).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    pub item: String,
    pub uq: i64,
}

fn edges(lines: &[(String, i64)]) -> Vec<Edge> {
    lines.iter().map(|(item, qty)| Edge { item: item.clone(), qty: Rat::new(i128::from(*qty), 1) }).collect()
}

/// Edges from `(item, uq)` lines, exact (W-QTYHUB H1): `Rat::new(uq, 10^6)`,
/// which reduces to `qty / 1` for a whole line -- the same walk as before.
pub fn edges_micro(lines: &[(String, i64)]) -> Vec<Edge> {
    lines.iter().map(|(item, uq)| Edge { item: item.clone(), qty: rat(*uq) }).collect()
}

/// A card as the walker reads it; a raw supply is a leaf (`None`).
fn node_of(json: &str) -> Option<tree::Card> {
    card_of(json).map(|c| node(&c))
}

/// Walk `lines` (a dish's `bom` as `(supply, qty)`, or `[(prep, 1)]` for one
/// unit of a ПФ) down to raw leaves, EXACT. `supply(id)` answers a stored
/// supply record. An unknown id is refused, not skipped: a write-off that
/// quietly drops a line is the defect this file exists to close.
pub fn leaves_exact(
    root: &str,
    lines: &[(String, i64)],
    supply: &dyn Fn(&str) -> Option<String>,
) -> Result<tree::Walked, Refusal> {
    leaves_of_edges(root, &edges(lines), supply)
}

/// [`leaves_exact`] for exact edges ([`edges_micro`]).
pub fn leaves_of_edges(root: &str, edges: &[Edge], supply: &dyn Fn(&str) -> Option<String>) -> Result<tree::Walked, Refusal> {
    let mut card = |id: &str| -> Result<Option<tree::Card>, String> {
        match supply(id) {
            Some(j) => Ok(node_of(&j)),
            None => Err(format!("unknown supply {id}")),
        }
    };
    Walk { depth_max: DEPTH_MAX }.run(root, edges, &mut card).map_err(|s| match s {
        Stop::Card(w) => Refusal::Unknown(w.trim_start_matches("unknown supply ").to_string()),
        other => Refusal::Tree(other),
    })
}

/// The raw leaves of `lines`, each rounded ONCE to millionths, untracked items
/// left out, sorted by id (two hubs expanding one dish agree byte for byte).
pub fn expand(lines: &[(String, i64)], supply: &dyn Fn(&str) -> Option<String>) -> Result<Vec<Leaf>, Refusal> {
    leaves_rounded(leaves_exact("dish", lines, supply)?, supply)
}

/// [`expand`] for `(item, uq)` lines in millionths (W-QTYHUB H1): a dish
/// with half a sheet of nori beside a semi-finished line expands exactly.
pub fn expand_micro(lines: &[(String, i64)], supply: &dyn Fn(&str) -> Option<String>) -> Result<Vec<Leaf>, Refusal> {
    leaves_rounded(leaves_of_edges("dish", &edges_micro(lines), supply)?, supply)
}

fn leaves_rounded(walked: tree::Walked, supply: &dyn Fn(&str) -> Option<String>) -> Result<Vec<Leaf>, Refusal> {
    let mut out = Vec::with_capacity(walked.leaves.len());
    for l in walked.leaves {
        if supply(&l.item).is_some_and(|j| untracked(&j)) {
            continue;
        }
        let uq = l.qty.to_micro().ok_or(Refusal::Tree(Stop::Overflow))?;
        if uq > 0 {
            out.push(Leaf { item: l.item, uq });
        }
    }
    out.sort_by(|a, b| a.item.cmp(&b.item));
    Ok(out)
}

/// The dish record with its `bom` replaced by its raw leaves, `[{supply,
/// uq}]`, for the ledger (`stock::bom_of` reads `uq`; `stock::draws_for` books
/// it). A dish without a `bom`, or one that does not expand, is answered AS
/// IT IS, with the refusal beside it, so the caller can say so.
pub fn for_ledger(supply: &dyn Fn(&str) -> Option<String>, product_json: &str) -> (String, Option<Refusal>) {
    let bom = crate::stock::bom_of(product_json);
    if bom.is_empty() || !bom.iter().any(|l| supply(&l.supply).is_some_and(|j| is_prep(&j))) {
        return (product_json.to_string(), None);
    }
    // EXACT LINES (W-QTYHUB H1): `uq`, so a fractional raw line beside a
    // semi-finished one is booked as typed, not rounded to the gram.
    let lines: Vec<(String, i64)> = bom.iter().map(|l| (l.supply.clone(), l.uq)).collect();
    match expand_micro(&lines, supply) {
        Ok(leaves) => {
            let Ok(mut v) = serde_json::from_str::<Value>(product_json) else { return (product_json.to_string(), None) };
            v["bom"] = Value::Array(leaves.iter().map(|l| json!({ "supply": l.item, "uq": l.uq })).collect());
            // The tree itself, so the ledger can take a batch cooked ahead
            // first (`stock::basket`); `bom` stays the all-raw answer.
            if let Some(t) = stocked::tree_json(supply, &lines) {
                v["tree"] = t;
            }
            (v.to_string(), None)
        }
        Err(why) => (product_json.to_string(), Some(why)),
    }
}

/// Everything a card must satisfy before it is stored. `supplies` is the
/// catalogue's `(id, record)` listing; the card being saved is read from
/// `card`, never from the listing's copy of `id` (a cycle back to `id` is
/// caught by the walk's stack before `id` is ever looked up).
pub fn check_card(id: &str, card: &Card, supplies: &[(String, String)]) -> Result<(), Refusal> {
    if card.lines.is_empty() || card.lines.len() > LINES_MAX {
        return Err(Refusal::Lines);
    }
    let max = QTY_MAX.saturating_mul(MICRO);
    if !(1..=max).contains(&card.yield_uq) {
        return Err(Refusal::Bound("the yield".into()));
    }
    let supply = |s: &str| supplies.iter().find(|(i, _)| i == s).map(|(_, j)| j.clone());
    for (i, l) in card.lines.iter().enumerate() {
        if l.item == id {
            return Err(Refusal::Itself);
        }
        if !(1..=max).contains(&l.uq) {
            return Err(Refusal::Bound(format!("{}: the quantity", l.item)));
        }
        if card.lines[..i].iter().any(|o| o.item == l.item) {
            return Err(Refusal::Twice(l.item.clone()));
        }
        if supply(&l.item).is_none() {
            return Err(Refusal::Unknown(l.item.clone()));
        }
    }
    let lines: Vec<(String, i64)> = card.lines.iter().map(|l| (l.item.clone(), l.uq)).collect();
    let below = leaves_of_edges(id, &edges_micro(&lines), &supply)?.depth;
    // The chain ABOVE: cards that (transitively) name this one. With this card
    // in the middle, the longest chain through it must still fit.
    if below + 1 + uses::height_above(id, supplies) > DEPTH_MAX {
        return Err(Refusal::Tree(Stop::TooDeep(vec![id.to_string()])));
    }
    Ok(())
}

/// K, per mille: the yield in grams over the sum of the lines' gross in grams
/// (g and ml are grams, a piece weighs its `weightPerUnit`). `None` when a
/// piece on the card has no weight -- shown as unknown, never refused.
pub fn k_pm(card: &Card, own_unit: &str, own_weight_per_unit: Option<f64>, supply: &dyn Fn(&str) -> Option<String>) -> Option<i64> {
    // A ratio for the screen (K per mille), not a quantity that is booked.
    let grams = |unit: &str, wpu: Option<f64>, uq: i64| -> Option<f64> {
        let qty = uq as f64 / MICRO as f64;
        match unit {
            "unit" => wpu.map(|w| w * qty),
            _ => Some(qty),
        }
    };
    let mut gross = 0.0;
    for l in &card.lines {
        let v: Value = serde_json::from_str(&supply(&l.item)?).ok()?;
        let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g");
        gross += grams(unit, v.get("weightPerUnit").and_then(Value::as_f64), l.uq)?;
    }
    if gross <= 0.0 {
        return None;
    }
    let out = grams(own_unit, own_weight_per_unit, card.yield_uq)?;
    Some((out * 1000.0 / gross).round() as i64)
}

/// A card's stored form, old and new (W-QTYHUB H2): millionths in, the old
/// bytes out for every whole line.
pub mod stored;
pub use stored::{card_from_value, card_json, micro_of, node, rat};

/// Where an item is used, and what one unit of it costs.
pub mod uses;
pub mod cost;
/// A semi-finished product kept ready: the shelf first, the card for the rest.
pub mod stocked;
pub use cost::{cost_micro, cost_micro_uq};
pub use uses::{uses_of, Uses};

#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod verify_tests;
/// W-QTYHUB H1/H2: half a sheet beside a semi-finished line, half a gram on a card.
#[cfg(test)]
mod fraction_tests;
/// W-QTYHUB RED probes (old API, fail before H1/H2).
#[cfg(test)]
mod red_probes;
