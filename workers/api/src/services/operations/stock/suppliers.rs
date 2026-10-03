//! SUPPLIERS AS CARDS, AND THE ORDERS SENT TO THEM (W-STOCK P5), pure.
//!
//! Two kinds through the stock door that already exists, `POST
//! /api/owner/stock/:kind` (no new route), body `{"card": {...}}`:
//!
//!   supplier  {name, phone?, telegram?, days?: [1..7], leadDays?, cutoff?: "HH:MM", lang?, gone?, id?}
//!             -- a card, written again to edit it; `gone: true` takes it off the list.
//!   ordered   {supplier: <card id>, lines: [{item, qty}]}
//!             -- the order list as it was SENT: "on order" until deliveries close it.
//!
//! Both are NOTES on the stock log (`dowiz_hub::stock::notes`): in the chain,
//! never a movement, so the shelf, the cost and every checkpoint are as they
//! were. The newest card of an id is the card. An order is closed by the
//! deliveries of its items AFTER it, oldest order first, and forgotten after
//! [`ORDER_OPEN_DAYS`] -- a delivery nobody recorded must not hide a shortage
//! for ever.

use std::collections::BTreeMap;

use dowiz_hub::stock::journal::Journal;
use dowiz_hub::stock::notes::at_of;
use dowiz_hub::stock::{StockEvent, StockLog};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::turn::{StockTurnIn, Told};

/// The movement's word for a card, and the note's kind.
pub const CARD: &str = "supplier";
/// The movement's word for an order sent, and the note's kind.
pub const ORDERED: &str = "ordered";
/// An order nobody delivered against stops counting as "on order" after this.
pub const ORDER_OPEN_DAYS: i64 = 14;
pub const NAME_MAX: usize = 80;
pub const LEAD_MAX: i64 = 30;
pub const LINES_MAX: usize = 100;
pub const QTY_MAX: i64 = 1_000_000;
const DAY_MS: i64 = 86_400_000;

/// A supplier's card.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Card {
    /// Minted from the name when empty; the same id edits the same card.
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub telegram: String,
    /// Delivery weekdays, 1 = Monday .. 7 = Sunday.
    #[serde(default)]
    pub days: Vec<u8>,
    #[serde(default)]
    pub lead_days: i64,
    /// The last time of day an order reaches the next delivery, "HH:MM"; "" = none said.
    #[serde(default)]
    pub cutoff: String,
    /// The language the order text is written in; "" = the owner's.
    #[serde(default)]
    pub lang: String,
    #[serde(default)]
    pub gone: bool,
}

/// One line of an order sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub item: String,
    pub qty: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ordered {
    pub supplier: String,
    pub lines: Vec<Line>,
}

type Bad = (u16, String);

/// The card as it is stored, or why it cannot be. PURE.
pub fn check(mut c: Card) -> Result<Card, Bad> {
    c.name = c.name.trim().to_string();
    if c.name.is_empty() || c.name.chars().count() > NAME_MAX {
        return Err((400, format!("a supplier has a name of 1 to {NAME_MAX} characters")));
    }
    c.id = match c.id.trim() {
        "" => dowiz_hub::import::slug(&c.name),
        id => id.to_string(),
    };
    if c.id.is_empty() || c.id.len() > 64 {
        return Err((400, "a supplier needs a short id".into()));
    }
    c.phone = c.phone.trim().to_string();
    if c.phone.chars().count() > 32 || !c.phone.chars().all(|ch| ch.is_ascii_digit() || " +-()".contains(ch)) {
        return Err((400, "a phone is digits, spaces, + - ( ), up to 32".into()));
    }
    c.telegram = c.telegram.trim().to_string();
    if c.telegram.chars().count() > 40 {
        return Err((400, "a Telegram name is up to 40 characters".into()));
    }
    if c.days.iter().any(|d| !(1..=7).contains(d)) {
        return Err((400, "delivery days are 1 (Monday) to 7 (Sunday)".into()));
    }
    c.days.sort_unstable();
    c.days.dedup();
    if !(0..=LEAD_MAX).contains(&c.lead_days) {
        return Err((400, format!("lead time is 0 to {LEAD_MAX} days")));
    }
    c.cutoff = c.cutoff.trim().to_string();
    if !c.cutoff.is_empty() && !is_hhmm(&c.cutoff) {
        return Err((400, "the order cutoff is HH:MM".into()));
    }
    c.lang = c.lang.trim().to_string();
    if !c.lang.is_empty() && !dowiz_hub::lang::LANGS.contains(&c.lang.as_str()) {
        return Err((400, format!("the language is one of {}", dowiz_hub::lang::LANGS.join(", "))));
    }
    Ok(c)
}

fn is_hhmm(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5 && b[2] == b':' && s[..2].parse::<u8>().is_ok_and(|h| h < 24) && s[3..].parse::<u8>().is_ok_and(|m| m < 60)
}

/// Every card the log holds, the newest of each id, in the order the ids
/// first appeared; a card marked `gone` is off the list.
pub fn cards(log: &StockLog) -> Vec<Card> {
    let mut out: Vec<Card> = Vec::new();
    for rec in log.notes(CARD) {
        let Some(card) = serde_json::from_str::<Value>(&rec).ok().and_then(|v| serde_json::from_value::<Card>(v["card"].clone()).ok()) else {
            continue;
        };
        match out.iter_mut().find(|c| c.id == card.id) {
            Some(c) => *c = card,
            None => out.push(card),
        }
    }
    out.retain(|c| !c.gone);
    out
}

/// What is on its way, per supply: the quantity, and since when (the oldest open order).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Open {
    pub qty: i64,
    pub since: Option<i64>,
}
pub type OnOrder = BTreeMap<String, Open>;

/// Orders sent and not yet delivered, per supply. PURE over the notes and the
/// journal: each delivery after an order closes the oldest open quantity of
/// its item; an order older than [`ORDER_OPEN_DAYS`] at `now` is forgotten.
pub fn on_order(orders: &[String], j: &Journal, now: i64) -> OnOrder {
    // (at, item, qty) of every line sent, oldest first.
    let mut sent: Vec<(i64, String, i64)> = Vec::new();
    for rec in orders {
        let at = at_of(rec).unwrap_or(0);
        let Some(o) = serde_json::from_str::<Value>(rec).ok().and_then(|v| serde_json::from_value::<Ordered>(v["order"].clone()).ok()) else {
            continue;
        };
        sent.extend(o.lines.into_iter().map(|l| (at, l.item, l.qty)));
    }
    let mut out = OnOrder::new();
    let items: Vec<String> = {
        let mut v: Vec<String> = sent.iter().map(|s| s.1.clone()).collect();
        v.sort();
        v.dedup();
        v
    };
    for item in items {
        let mut open: Vec<(i64, i64)> = sent.iter().filter(|s| s.1 == item).map(|s| (s.0, s.2)).collect();
        for e in &j.entries {
            let (StockEvent::Received { item: i, qty }, Some(at)) = (&e.ev, e.meta.at) else { continue };
            if *i != item {
                continue;
            }
            let mut left = *qty;
            for o in open.iter_mut().filter(|o| o.0 <= at) {
                let take = left.min(o.1);
                o.1 -= take;
                left -= take;
                if left == 0 {
                    break;
                }
            }
        }
        let live: Vec<&(i64, i64)> = open.iter().filter(|o| o.1 > 0 && now - o.0 <= ORDER_OPEN_DAYS * DAY_MS).collect();
        if !live.is_empty() {
            out.insert(item, Open { qty: live.iter().map(|o| o.1).sum(), since: live.iter().map(|o| o.0).min() });
        }
    }
    out
}

/// A card or an order sent, as the venue object's turn: checked, appended as
/// a note, answered. Nothing is told to the groups. A refusal writes nothing.
pub fn run(log: &mut StockLog, input: &StockTurnIn) -> Result<(Value, Told), Bad> {
    let raw = input.body.get("card").cloned().ok_or((400, "the body is {\"card\": {...}}".to_string()))?;
    log.set_clock(input.now_ms);
    let refused = |e: dowiz_hub::stock::StockError| (500, e.to_string());
    if input.kind == CARD {
        let card = check(serde_json::from_value::<Card>(raw).map_err(|e| (400, format!("bad supplier card: {e}")))?)?;
        log.append_note(CARD, &json!({ "card": card, "by_": input.by }).to_string()).map_err(refused)?;
        return Ok((json!({ "ok": true, "kind": CARD, "card": card }), Vec::new()));
    }
    let o = serde_json::from_value::<Ordered>(raw).map_err(|e| (400, format!("bad order: {e}")))?;
    let known = cards(log);
    if !known.iter().any(|c| c.id == o.supplier) {
        return Err((404, format!("no supplier card {}", o.supplier)));
    }
    if o.lines.is_empty() || o.lines.len() > LINES_MAX {
        return Err((400, format!("an order has 1 to {LINES_MAX} lines")));
    }
    for l in &o.lines {
        if !input.supplies.contains_key(&l.item) {
            return Err((404, format!("not found: {}", l.item)));
        }
        if !(1..=QTY_MAX).contains(&l.qty) {
            return Err((400, format!("{}: 1 to {QTY_MAX}", l.item)));
        }
    }
    let po = format!("po_{}", input.now_ms);
    log.append_note(ORDERED, &json!({ "order": o, "po": po, "by_": input.by }).to_string()).map_err(refused)?;
    Ok((json!({ "ok": true, "kind": ORDERED, "po": po, "lines": o.lines.len() }), Vec::new()))
}

#[cfg(test)]
#[path = "suppliers/tests.rs"]
mod tests;
