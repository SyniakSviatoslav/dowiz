//! A SEMI-FINISHED PRODUCT KEPT READY (lane W-PF2 R2, SPEC-SEMI-FINISHED §k):
//! the kitchen cooks a batch ahead (`stock::act`), and a sale -- or the next
//! batch of something made from it -- takes the READY product off the shelf
//! first. What the shelf cannot give is expanded through the card to raw, as
//! if nothing had been cooked ahead.
//!
//! WHY EXPAND THE REST AND NOT REFUSE: a batch running out mid-service does
//! not stop the kitchen -- the cook makes the next one from raw, which is
//! exactly what the expansion books. Refusing would 86 a dish whose raw items
//! are on the shelf; the operator's standing rule is that only a MEASURED
//! shortage of a raw item refuses a sale (`stock.rs` I1, uncounted never).
//!
//! THE WALK, TOP-DOWN IN TOPOLOGICAL ORDER: every semi-finished node's whole
//! need is summed from all its users before it is split into "from the shelf"
//! and "from its card", so a basket with two rolls on one rice takes the rice
//! batch once, for both. Exact (`Rat`) all the way; a leaf is rounded ONCE to
//! millionths (`uq`). With nothing stocked the answer is `super::expand`'s,
//! leaf for leaf (proved in `stocked/tests.rs`).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use super::tree::{Card, Stop};
use super::{card_of, is_prep, untracked, Leaf, Refusal, DEPTH_MAX, MICRO};
use crate::import::recipes::num::Rat;

/// What the walk asks of the world.
pub struct World<'a> {
    /// The card of a semi-finished id, `None` for a raw one, `Err` for an id
    /// nobody knows.
    pub card: &'a dyn Fn(&str) -> Result<Option<Card>, Refusal>,
    /// A raw item nobody counts (water): no leaf.
    pub untracked: &'a dyn Fn(&str) -> bool,
    /// Millionths of a READY semi-finished product the shelf can give now
    /// (0: not kept, or nothing left).
    pub ready: &'a dyn Fn(&str) -> i64,
}

fn lt(a: Rat, b: Rat) -> Result<bool, Refusal> {
    let over = Refusal::Tree(Stop::Overflow);
    Ok(a.num.checked_mul(b.den).ok_or(over.clone())? < b.num.checked_mul(a.den).ok_or(over)?)
}

fn add(m: &mut BTreeMap<String, Rat>, id: &str, q: Rat) -> Result<(), Refusal> {
    let cur = m.get(id).copied().unwrap_or(Rat::new(0, 1));
    m.insert(id.to_string(), cur.checked_add(q).ok_or(Refusal::Tree(Stop::Overflow))?);
    Ok(())
}

/// Every card under `roots`, with how many card lines name each (its users),
/// cycles and depth refused with their path.
fn cards_under(roots: &[(String, Rat)], w: &World) -> Result<(BTreeMap<String, Card>, BTreeMap<String, usize>), Refusal> {
    // Each card walked once; its height (cards under it, itself included)
    // remembered, so a shared sauce is not re-walked per path and the depth
    // of a path through it is still known.
    fn visit(id: &str, w: &World, stack: &mut Vec<String>, cards: &mut BTreeMap<String, (Card, usize)>) -> Result<usize, Refusal> {
        if stack.iter().any(|s| s == id) {
            let mut path = stack.clone();
            path.push(id.to_string());
            return Err(Refusal::Tree(Stop::Cycle(path)));
        }
        if let Some((_, h)) = cards.get(id) {
            if stack.len() - 1 + h > DEPTH_MAX {
                let mut path = stack.clone();
                path.push(id.to_string());
                return Err(Refusal::Tree(Stop::TooDeep(path)));
            }
            return Ok(*h);
        }
        let Some(c) = (w.card)(id)? else { return Ok(0) };
        stack.push(id.to_string());
        if stack.len() - 1 > DEPTH_MAX {
            return Err(Refusal::Tree(Stop::TooDeep(stack.clone())));
        }
        let mut below = 0;
        for l in &c.lines {
            below = below.max(visit(&l.item, w, stack, cards)?);
        }
        stack.pop();
        cards.insert(id.to_string(), (c, below + 1));
        Ok(below + 1)
    }
    let mut walked = BTreeMap::new();
    for (id, _) in roots {
        visit(id, w, &mut vec!["root".to_string()], &mut walked)?;
    }
    let cards: BTreeMap<String, Card> = walked.into_iter().map(|(k, (c, _))| (k, c)).collect();
    let mut users: BTreeMap<String, usize> = cards.keys().map(|k| (k.clone(), 0)).collect();
    for c in cards.values() {
        for l in &c.lines {
            if let Some(n) = users.get_mut(&l.item) {
                *n += 1;
            }
        }
    }
    Ok((cards, users))
}

/// How much of each semi-finished product's WHOLE need the shelf gave
/// (`take / need`), for the products it gave anything (W-PF3 T1): the same
/// share of every dish that reaches the product, so a stamp per dish adds up
/// to what the basket booked.
pub type Shelf = BTreeMap<String, Rat>;

/// The walk itself: top-down, `take(id, need)` says how much of a node's
/// whole need comes off the shelf; the rest goes through its card. Exact.
fn walk(roots: &[(String, Rat)], w: &World, take_of: &dyn Fn(&str, Rat) -> Result<Rat, Refusal>) -> Result<(BTreeMap<String, Rat>, Shelf), Refusal> {
    let over = || Refusal::Tree(Stop::Overflow);
    let (cards, mut users) = cards_under(roots, w)?;
    let mut need: BTreeMap<String, Rat> = BTreeMap::new();
    for (id, q) in roots {
        add(&mut need, id, *q)?;
    }
    let mut out: BTreeMap<String, Rat> = BTreeMap::new();
    let mut shelf: Shelf = BTreeMap::new();
    let mut ready: BTreeSet<String> = users.iter().filter(|(_, n)| **n == 0).map(|(k, _)| k.clone()).collect();
    while let Some(id) = ready.pop_first() {
        let c = &cards[&id];
        let n = need.remove(&id).unwrap_or(Rat::new(0, 1));
        let take = take_of(&id, n)?;
        if take.num > 0 {
            add(&mut out, &id, take)?;
            shelf.insert(id.clone(), take.checked_div(n).ok_or_else(over)?);
        }
        let rest = n.checked_add(Rat::new(-take.num, take.den)).ok_or_else(over)?;
        let per = rest.checked_div(c.batch).ok_or_else(over)?;
        for l in &c.lines {
            if rest.num > 0 {
                add(&mut need, &l.item, l.qty.checked_mul(per).ok_or_else(over)?)?;
            }
            if let Some(u) = users.get_mut(&l.item) {
                *u -= 1;
                if *u == 0 {
                    ready.insert(l.item.clone());
                }
            }
        }
    }
    for (id, q) in need {
        if !(w.untracked)(&id) {
            add(&mut out, &id, q)?;
        }
    }
    Ok((out, shelf))
}

/// `roots` (item, exact quantity) down to what leaves the shelf: raw leaves,
/// and ready semi-finished products as far as the shelf has them. Sorted by
/// id, untracked items out, every leaf rounded once.
pub fn plan(roots: &[(String, Rat)], w: &World) -> Result<Vec<Leaf>, Refusal> {
    plan_split(roots, w).map(|(l, _)| l)
}

/// [`plan`], and the share of each product the shelf gave ([`Shelf`]).
pub fn plan_split(roots: &[(String, Rat)], w: &World) -> Result<(Vec<Leaf>, Shelf), Refusal> {
    let take = |id: &str, n: Rat| -> Result<Rat, Refusal> {
        let shelf = Rat::new(i128::from((w.ready)(id).max(0)), i128::from(MICRO));
        Ok(if lt(n, shelf)? { n } else { shelf })
    };
    let (out, shelf) = walk(roots, w, &take)?;
    let mut leaves = Vec::with_capacity(out.len());
    for (item, q) in out {
        let uq = q.to_micro().ok_or(Refusal::Tree(Stop::Overflow))?;
        if uq > 0 {
            leaves.push(Leaf { item, uq });
        }
    }
    Ok((leaves, shelf))
}

/// What `roots` take when every product gives the SAME share from the shelf
/// as the basket's walk gave it (`shelf`), exact, unrounded: one dish's part
/// of a basket (`w.ready` is not asked).
pub fn portion(roots: &[(String, Rat)], w: &World, shelf: &Shelf) -> Result<BTreeMap<String, Rat>, Refusal> {
    let take = |id: &str, n: Rat| -> Result<Rat, Refusal> {
        Ok(match shelf.get(id) {
            Some(f) => n.checked_mul(*f).ok_or(Refusal::Tree(Stop::Overflow))?,
            None => Rat::new(0, 1),
        })
    };
    walk(roots, w, &take).map(|(out, _)| out)
}

/// The world a catalogue answers: cards from its supplies, unknown ids refused.
pub fn card_from(supply: &dyn Fn(&str) -> Option<String>, id: &str) -> Result<Option<Card>, Refusal> {
    let j = supply(id).ok_or_else(|| Refusal::Unknown(id.to_string()))?;
    Ok(card_of(&j).map(|c| Card {
        lines: c.lines.iter().map(|l| super::tree::Edge { item: l.item.clone(), qty: Rat::new(i128::from(l.qty), 1) }).collect(),
        batch: Rat::new(i128::from(c.yield_qty), 1),
    }))
}

/// What a sale must carry for the shelf to be asked (`stock::basket`): the
/// dish's own lines, every card under them, and the untracked leaves. `None`
/// for a dish whose tree reaches no semi-finished product.
pub fn tree_json(supply: &dyn Fn(&str) -> Option<String>, lines: &[(String, i64)]) -> Option<Value> {
    let mut cards = serde_json::Map::new();
    let mut quiet: BTreeSet<String> = BTreeSet::new();
    let mut todo: Vec<String> = lines.iter().map(|(s, _)| s.clone()).collect();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    while let Some(id) = todo.pop() {
        if !seen.insert(id.clone()) || seen.len() > 4096 {
            continue;
        }
        let Some(j) = supply(&id) else { continue };
        if untracked(&j) {
            quiet.insert(id.clone());
        }
        if is_prep(&j) {
            if let Some(c) = card_of(&j) {
                todo.extend(c.lines.iter().map(|l| l.item.clone()));
                cards.insert(id.clone(), super::card_json(&c));
            }
        }
    }
    if cards.is_empty() {
        return None;
    }
    let lines: Vec<Value> = lines.iter().map(|(s, q)| json!({ "supply": s, "qty": q })).collect();
    Some(json!({ "lines": lines, "cards": cards, "untracked": quiet }))
}

#[cfg(test)]
mod tests;
