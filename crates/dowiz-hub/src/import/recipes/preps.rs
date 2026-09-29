//! SEMI-FINISHED CARDS IMPORTED AS REAL SEMI-FINISHED PRODUCTS (lane W-PF2 R1,
//! SPEC-SEMI-FINISHED §g's seam): a prepack card of the file becomes a
//! [`DraftPrep`] -- a supply of kind `prep` with its card -- and a dish that
//! uses it NAMES it on its recipe, so the kitchen edits the sauce once and
//! every dish follows (`crate::prep`). Only when `Opts::preps` is set: the
//! Worker writes the supplies before the dishes (`bulk/preps.rs`).
//!
//! A CARD THAT CANNOT BE A REAL ONE IS NOT GUESSED: a line or a batch that is
//! not a whole number of its unit, more than `prep::LINES_MAX` lines, a
//! refused row, a cycle, too many levels -- the card is not created, a
//! warning says why, and each dish that uses it is FLATTENED through it
//! exactly as before (`flatten.rs`), or refused by the same rule.
//!
//! A FILE WITH NO PREPACK reads byte for byte as it did: no dish takes this
//! path (`dish_lines` answers `None` for a card with no semi-finished line)
//! and `as_json` writes no `preps` key (`tests.rs` pins the old bytes).

use std::collections::BTreeMap;

use super::cards::{Card, Item};
use super::num::{Base, Rat};
use super::{DraftLine, RecipeDraft, QTY_MAX};
use crate::minijson::esc;

/// A semi-finished product the import creates or updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftPrep {
    /// The card's key (`slug` of its name): the supply id.
    pub id: String,
    pub name: String,
    /// `g`, `ml` or `unit`: what the yield is counted in.
    pub unit: &'static str,
    /// `(item, gross)`, an item being a supply or another semi-finished product.
    pub lines: Vec<(String, i64)>,
    /// The batch's net output, in `unit`.
    pub yield_qty: i64,
}

/// The base unit a line of `c` is counted in: its supply's, or the batch
/// unit of the semi-finished product it names.
fn base_of(i: &Item, cards: &[Card]) -> Option<Base> {
    match i {
        Item::Supply { base, .. } => Some(*base),
        Item::Pre { key, .. } => cards.iter().find(|p| p.prepack && &p.key == key).and_then(|p| p.batch.map(|b| b.0)),
    }
}

fn id_of(i: &Item) -> &str {
    match i {
        Item::Supply { id, .. } => id,
        Item::Pre { key, .. } => key,
    }
}

/// A card's lines, one per item (a file may list an item twice: summed,
/// exactly), each a whole number of its unit or refused with the reason.
fn merged(c: &Card, cards: &[Card]) -> Result<Vec<(String, i64, usize)>, String> {
    let mut acc: Vec<(String, Rat, Base, usize)> = Vec::new();
    for i in &c.items {
        let (id, q) = match i {
            Item::Supply { id, qty, .. } | Item::Pre { key: id, qty } => (id.clone(), *qty),
        };
        let base = base_of(i, cards).ok_or_else(|| format!("{id} has no readable unit"))?;
        match acc.iter_mut().find(|a| a.0 == id) {
            Some(a) => {
                a.1 = a.1.checked_add(q).ok_or("the fractions grew past what can be computed exactly")?;
                a.3 += 1;
            }
            None => acc.push((id, q, base, 1)),
        }
    }
    acc.into_iter().map(|(id, q, base, n)| q.whole(base, QTY_MAX).map(|w| (id.clone(), w, n)).map_err(|e| format!("{id} {e}"))).collect()
}

/// One prepack as a draft on its own, and the semi-finished products it names.
fn own(c: &Card, cards: &[Card]) -> Result<(DraftPrep, Vec<String>), String> {
    if let Some(e) = c.errors.first() {
        return Err(e.clone());
    }
    let (base, batch) = c.batch.ok_or("it has no batch size")?;
    let yield_qty = batch.whole(base, QTY_MAX).map_err(|e| format!("its batch {e}"))?;
    let lines = merged(c, cards)?;
    if lines.is_empty() || lines.len() > crate::prep::LINES_MAX {
        return Err(format!("a card has 1 to {} lines", crate::prep::LINES_MAX));
    }
    let deps = c.items.iter().filter(|i| matches!(i, Item::Pre { .. })).map(|i| id_of(i).to_string()).collect();
    let lines = lines.into_iter().map(|(id, q, _)| (id, q)).collect();
    Ok((DraftPrep { id: c.key.clone(), name: c.dish.clone(), unit: base.as_str(), lines, yield_qty }, deps))
}

/// Every prepack of the file that can be a real semi-finished product, into
/// `draft.preps` CHILDREN FIRST (a card is saved against the ones it names);
/// the keys made, for `dish_lines`. The rest are warnings.
pub(super) fn build(cards: &[Card], draft: &mut RecipeDraft) -> Vec<String> {
    let pre: Vec<&Card> = cards.iter().filter(|c| c.prepack).collect();
    let mut failed: BTreeMap<String, String> = BTreeMap::new();
    let mut depth: BTreeMap<String, usize> = BTreeMap::new();
    let mut made: Vec<String> = Vec::new();
    loop {
        let mut moved = false;
        for c in &pre {
            if depth.contains_key(&c.key) || failed.contains_key(&c.key) {
                continue;
            }
            let (p, deps) = match own(c, cards) {
                Ok(v) => v,
                Err(e) => {
                    failed.insert(c.key.clone(), e);
                    moved = true;
                    continue;
                }
            };
            if let Some(d) = deps.iter().find(|d| failed.contains_key(*d)) {
                failed.insert(c.key.clone(), format!("it uses {d}, which cannot be created"));
                moved = true;
                continue;
            }
            if !deps.iter().all(|d| depth.contains_key(d)) {
                continue; // waits for what it names
            }
            let level = 1 + deps.iter().map(|d| depth[d]).max().unwrap_or(0);
            if level > crate::prep::DEPTH_MAX {
                failed.insert(c.key.clone(), format!("more than {} levels of semi-finished products", crate::prep::DEPTH_MAX));
            } else {
                depth.insert(c.key.clone(), level);
                made.push(p.id.clone());
                draft.preps.push(p);
            }
            moved = true;
        }
        if !moved {
            break;
        }
    }
    for c in &pre {
        let why = match failed.get(&c.key) {
            Some(w) => w.clone(),
            None if !depth.contains_key(&c.key) => "semi-finished products name each other".to_string(),
            None => continue,
        };
        draft.warnings.push(format!("semi-finished {:?} is not created: {why}; a dish that uses it takes its ingredients instead", c.dish));
    }
    made
}

/// A dish's recipe NAMING the semi-finished products it uses: `None` when the
/// card has no semi-finished line, or one that was not made, or a refused row
/// -- the caller flattens then, as it always did.
pub(super) fn dish_lines(c: &Card, cards: &[Card], made: &[String]) -> Option<Result<Vec<DraftLine>, String>> {
    let pre: Vec<&str> = c.items.iter().filter(|i| matches!(i, Item::Pre { .. })).map(id_of).collect();
    if pre.is_empty() || !c.errors.is_empty() || !pre.iter().all(|k| made.iter().any(|m| m == k)) {
        return None;
    }
    let lines = match merged(c, cards) {
        Ok(l) => l,
        Err(e) => return Some(Err(e)),
    };
    // Net and yield describe ONE direct line of a raw supply; summed or a
    // semi-finished product, a line keeps neither (as `flatten` rules).
    let side = |id: &str| {
        c.items.iter().find_map(|i| match i {
            Item::Supply { id: s, net, yld, .. } if s == id => Some((*net, *yld)),
            _ => None,
        })
    };
    Some(Ok(lines
        .into_iter()
        .map(|(id, qty, n)| {
            let (net, yield_) = if n == 1 { side(&id).unwrap_or((None, None)) } else { (None, None) };
            DraftLine { supply: id, qty, net, yield_ }
        })
        .collect()))
}

/// `,"preps":[...]` for the preview, or nothing at all: a file with no
/// semi-finished product answers the bytes it always did.
pub(super) fn json_tail(preps: &[DraftPrep]) -> String {
    if preps.is_empty() {
        return String::new();
    }
    let rows: Vec<String> = preps
        .iter()
        .map(|p| {
            let lines: Vec<String> = p.lines.iter().map(|(i, q)| format!(r#"{{"item":"{}","qty":{q}}}"#, esc(i))).collect();
            format!(r#"{{"id":"{}","name":"{}","unit":"{}","yield":{},"lines":[{}]}}"#, esc(&p.id), esc(&p.name), p.unit, p.yield_qty, lines.join(","))
        })
        .collect();
    format!(r#","preps":[{}]"#, rows.join(","))
}

#[cfg(test)]
mod tests;
