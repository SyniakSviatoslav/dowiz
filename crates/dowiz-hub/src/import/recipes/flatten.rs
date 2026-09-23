//! Semi-finished products, expanded to the supplies they are made of.
//!
//! Exact fractions all the way down (`Rat`), then ONE check per leaf: it is a
//! whole number of its base unit, or the dish is refused. iiko's
//! `getPrepared` rounds each level to the gram; rounding at every level is
//! how a sauce used in forty dishes drifts forty ways.

use super::cards::{Card, Item};
use super::num::{Base, Rat};
use super::{DraftLine, QTY_MAX};

/// One leaf supply, summed over every path that reaches it.
struct Acc {
    id: String,
    base: Base,
    qty: Rat,
    net: Option<i64>,
    yld: Option<i64>,
    uses: u32,
}

/// Expand one card to leaf lines, exactly; then each must be a whole unit.
pub(super) fn flatten(c: &Card, cards: &[Card], report: &mut Vec<String>) -> Result<Vec<DraftLine>, String> {
    if let Some(e) = c.errors.first() {
        return Err(e.clone());
    }
    let mut acc: Vec<Acc> = Vec::new();
    let mut stack = vec![c.key.clone()];
    expand(c, Rat::new(1, 1), cards, &mut stack, &mut acc, report)?;
    let mut out = Vec::with_capacity(acc.len());
    for Acc { id, base, qty, net, yld, uses } in acc {
        let q = qty.whole(base, QTY_MAX).map_err(|e| format!("{id} {e}"))?;
        // Net and yield describe ONE direct line; merged ones would lie.
        let (net, yield_) = if uses == 1 { (net, yld) } else { (None, None) };
        out.push(DraftLine { supply: id, qty: q, net, yield_ });
    }
    if out.is_empty() {
        return Err("the card has no lines".into());
    }
    Ok(out)
}

fn expand(
    c: &Card,
    factor: Rat,
    cards: &[Card],
    stack: &mut Vec<String>,
    acc: &mut Vec<Acc>,
    report: &mut Vec<String>,
) -> Result<(), String> {
    for item in &c.items {
        match item {
            Item::Supply { id, base, qty, net, yld } => {
                let q = qty.mul(factor);
                match acc.iter_mut().find(|a| &a.id == id) {
                    Some(a) => {
                        a.qty = a.qty.add(q);
                        a.uses += 1;
                    }
                    None => acc.push(Acc { id: id.clone(), base: *base, qty: q, net: *net, yld: *yld, uses: 1 }),
                }
            }
            Item::Pre { key, qty } => {
                if stack.contains(key) {
                    stack.push(key.clone());
                    return Err(format!("semi-finished products name each other: {}", stack.join(" → ")));
                }
                let Some(p) = cards.iter().find(|p| &p.key == key && p.prepack) else {
                    return Err(format!("semi-finished {key:?} has no card in force today"));
                };
                if let Some(e) = p.errors.first() {
                    return Err(format!("uses {:?}, which was refused ({e})", p.dish));
                }
                let (_, batch) = p.batch.ok_or_else(|| format!("{:?} has no batch size", p.dish))?;
                stack.push(key.clone());
                expand(p, qty.mul(factor).div(batch), cards, stack, acc, report)?;
                stack.pop();
                report.push(format!("{:?} expanded into {} line(s)", p.dish, p.items.len()));
            }
        }
    }
    Ok(())
}
