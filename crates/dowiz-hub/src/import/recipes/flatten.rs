//! Semi-finished products, expanded to the supplies they are made of.
//!
//! Exact fractions all the way down (`Rat`), then ONE check per leaf: it is a
//! whole number of its base unit, or the dish is refused. iiko's
//! `getPrepared` rounds each level to the gram; rounding at every level is
//! how a sauce used in forty dishes drifts forty ways.
//!
//! THE RECURSION IS THE CATALOGUE'S (`crate::prep::tree`, 2026-09-29): the
//! same walker that expands a stored semi-finished card at a sale, so a cycle,
//! a depth and an overflow are found by one piece of code in both places.

use super::cards::{Card, Item};
use super::{DraftLine, QTY_MAX};
use crate::prep::tree::{self, Edge, Walk};

/// Expand one card to leaf lines, exactly; then each must be a whole unit.
pub(super) fn flatten(c: &Card, cards: &[Card], report: &mut Vec<String>) -> Result<Vec<DraftLine>, String> {
    if let Some(e) = c.errors.first() {
        return Err(e.clone());
    }
    let root: Vec<Edge> = c.items.iter().map(edge).collect();
    // Net and yield describe ONE direct line of THIS card; a leaf reached
    // through a semi-finished product, or twice, keeps neither.
    let side = |id: &str| {
        c.items.iter().find_map(|i| match i {
            Item::Supply { id: s, net, yld, .. } if s == id => Some((*net, *yld)),
            _ => None,
        })
    };
    let mut node = |key: &str| -> Result<Option<tree::Card>, String> {
        let Some(p) = cards.iter().find(|p| p.key == key && p.prepack) else {
            // A supply id, or a name no card in force answers: a leaf. The
            // line reader already refused a name that is neither.
            return Ok(None);
        };
        if let Some(e) = p.errors.first() {
            return Err(format!("uses {:?}, which was refused ({e})", p.dish));
        }
        let (_, batch) = p.batch.ok_or_else(|| format!("{:?} has no batch size", p.dish))?;
        Ok(Some(tree::Card { lines: p.items.iter().map(edge).collect(), batch }))
    };
    let walked = Walk { depth_max: crate::prep::DEPTH_MAX }.run(&c.key, &root, &mut node).map_err(|s| s.to_string())?;
    for (key, n) in &walked.through {
        let dish = cards.iter().find(|p| &p.key == key).map(|p| p.dish.as_str()).unwrap_or(key);
        report.push(format!("{dish:?} expanded into {n} line(s)"));
    }
    let mut out = Vec::with_capacity(walked.leaves.len());
    for leaf in walked.leaves {
        let base = c
            .items
            .iter()
            .chain(cards.iter().filter(|p| p.prepack).flat_map(|p| p.items.iter()))
            .find_map(|i| match i {
                Item::Supply { id, base, .. } if *id == leaf.item => Some(*base),
                _ => None,
            })
            .ok_or_else(|| format!("{} is not a supply", leaf.item))?;
        let q = leaf.qty.whole(base, QTY_MAX).map_err(|e| format!("{} {e}", leaf.item))?;
        let (net, yield_) = match (leaf.uses, leaf.direct) {
            (1, true) => side(&leaf.item).unwrap_or((None, None)),
            _ => (None, None),
        };
        out.push(DraftLine { supply: leaf.item, qty: q, net, yield_ });
    }
    if out.is_empty() {
        return Err("the card has no lines".into());
    }
    Ok(out)
}

/// A card's item as the walker's edge: a supply by its id, a semi-finished
/// product by its card key.
fn edge(i: &Item) -> Edge {
    match i {
        Item::Supply { id, qty, .. } => Edge { item: id.clone(), qty: *qty },
        Item::Pre { key, qty } => Edge { item: key.clone(), qty: *qty },
    }
}
