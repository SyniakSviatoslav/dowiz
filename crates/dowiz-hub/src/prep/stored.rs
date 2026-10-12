//! A CARD'S STORED FORM (W-QTYHUB H2, CONTRACT-QTY §1): a line or a yield
//! in millionths of the base unit, read from `uq`/`yieldUq` or a whole
//! `qty`/`yield`, and written back so that every whole card keeps the exact
//! bytes it was stored with.

use serde_json::{json, Value};

use super::{tree, Card, Line, MICRO};
use crate::import::recipes::num::Rat;
use tree::Edge;

impl Line {
    /// `qty` whole base units.
    pub fn whole(item: impl Into<String>, qty: i64) -> Line {
        Line { item: item.into(), uq: qty.saturating_mul(MICRO) }
    }
    /// Whole base units, half up: what a whole-unit reader shows.
    pub fn qty(&self) -> i64 {
        half_micro(self.uq)
    }
}

impl Card {
    /// The yield in whole base units, half up.
    pub fn yield_qty(&self) -> i64 {
        half_micro(self.yield_uq)
    }
}

fn half_micro(uq: i64) -> i64 {
    (uq.saturating_add(MICRO / 2)).div_euclid(MICRO)
}

/// An exact quantity in millionths as the tree's rational.
pub fn rat(uq: i64) -> Rat {
    Rat::new(i128::from(uq), i128::from(MICRO))
}

/// A stored quantity in millionths: `uq_key` when the object carries it,
/// else whole `qty_key` x 10^6 -- every record written before fractions.
pub fn micro_of(v: &Value, uq_key: &str, qty_key: &str) -> Option<i64> {
    match v.get(uq_key).and_then(Value::as_i64) {
        Some(u) => Some(u),
        None => v.get(qty_key)?.as_i64().map(|q| q.saturating_mul(MICRO)),
    }
}

/// A card as its JSON stores it (`card_json`'s form, old or new).
pub fn card_from_value(c: &Value) -> Option<Card> {
    let yield_uq = micro_of(c, "yieldUq", "yield")?;
    let lines = c
        .get("lines")
        .and_then(Value::as_array)
        .map(|ls| ls.iter().filter_map(|l| Some(Line { item: l.get("item")?.as_str()?.to_string(), uq: micro_of(l, "uq", "qty")? })).collect())
        .unwrap_or_default();
    Some(Card { lines, yield_uq })
}

/// The card as the walker reads it: exact edges, exact batch.
pub fn node(c: &Card) -> tree::Card {
    tree::Card { lines: c.lines.iter().map(|l| Edge { item: l.item.clone(), qty: rat(l.uq) }).collect(), batch: rat(c.yield_uq) }
}

/// The card as the record stores it: lean, keys in one order. A whole line
/// and a whole yield write exactly the bytes they always did.
pub fn card_json(card: &Card) -> Value {
    let lines: Vec<Value> = card
        .lines
        .iter()
        .map(|l| match l.uq % MICRO {
            0 => json!({ "item": l.item, "qty": l.uq / MICRO }),
            _ => json!({ "item": l.item, "uq": l.uq }),
        })
        .collect();
    match card.yield_uq % MICRO {
        0 => json!({ "lines": lines, "yield": card.yield_uq / MICRO }),
        _ => json!({ "lines": lines, "yieldUq": card.yield_uq }),
    }
}

