//! PURE. What a proposal token carries, and what its confirmation gives back.
//!
//! THE CONFIRMATION CARRIES ITS OWN INSTRUCTION. The scope `voice:<verb>:<arg>`
//! is signed, so the round, the version, the dish, the count, the amount and
//! the method a person agreed to are exactly the ones the surface is handed
//! back -- nothing is classified, matched or looked up a second time, and a
//! round that moved in between is refused by the route through `base_seq`.
//!
//! The arg is fields joined by `|` (items by `,`, product and count by `*`).
//! A field holding one of those is refused at encode time rather than escaped:
//! the ids here are the hub's own, and a table name with a pipe in it is not
//! one a voice command should be reaching.

use serde_json::{json, Value};

const SEPARATORS: [char; 3] = ['|', ',', '*'];
/// A field longer than this is not one of ours.
const FIELD_MAX: usize = 64;

/// Is this safe to put in a scope field?
pub fn clean(s: &str) -> bool {
    !s.is_empty() && s.chars().count() <= FIELD_MAX && !s.contains(SEPARATORS)
}

/// The arg for a verb, or `None` when a field is not clean.
pub fn arg(fields: &[&str]) -> Option<String> {
    fields.iter().all(|f| clean(f)).then(|| fields.join("|"))
}

/// `pid*qty,pid*qty`, or `None` when an id is not clean.
pub fn items(lines: &[(String, u32)]) -> Option<String> {
    lines
        .iter()
        .map(|(p, q)| clean(p).then(|| format!("{p}*{q}")))
        .collect::<Option<Vec<_>>>()
        .map(|v| v.join(","))
}

/// The `place` arg: the table, then the lines.
pub fn place(table: &str, lines: &[(String, u32)]) -> Option<String> {
    if !clean(table) || lines.is_empty() {
        return None;
    }
    items(lines).map(|l| format!("{table}|{l}"))
}

fn num(s: &str) -> Option<i64> {
    s.parse().ok()
}

/// The confirmation's instruction for one of the verbs this layer added, or
/// `None` for the hub's own (their arg is an order id, sent as `orderId`).
pub fn decode(verb: &str, rest: &str) -> Option<Value> {
    let f: Vec<&str> = rest.split('|').collect();
    match (verb, f.as_slice()) {
        ("add", [order, seq, product, qty]) => Some(json!({
            "orderId": order, "baseSeq": num(seq)?, "productId": product, "quantity": num(qty)?
        })),
        ("place", [table, list]) => {
            let items = list
                .split(',')
                .map(|l| {
                    let (p, q) = l.split_once('*')?;
                    Some(json!({ "product_id": p, "quantity": num(q)? }))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(json!({ "table": table, "items": items }))
        }
        ("pay", [order, seq, amount, method]) => Some(json!({
            "orderId": order, "baseSeq": num(seq)?, "amount": num(amount)?, "method": method
        })),
        ("dish_off" | "dish_on", [product]) => Some(json!({ "productId": product })),
        ("venue", [state]) => Some(json!({ "state": state })),
        ("receive", [item, qty]) => Some(json!({ "itemId": item, "qty": num(qty)? })),
        ("waste", [item, qty, reason]) => Some(json!({ "itemId": item, "qty": num(qty)?, "reason": reason })),
        _ => None,
    }
}

/// The verbs `decode` owns. A confirmation for one of them that does not
/// decode is refused, never passed through as an order id.
pub fn is_ours(verb: &str) -> bool {
    matches!(verb, "add" | "place" | "pay" | "dish_off" | "dish_on" | "venue" | "receive" | "waste")
}

#[cfg(test)]
mod tests;
