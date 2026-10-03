//! PURE. The offline sale's checks, its re-pricing, and its envelope.
//!
//! A REFUSAL HERE IS A MALFORMED BODY, never a fact about the venue: a sale
//! that happened is recorded whatever changed since (see the module header).

use super::{Conflict, LineIn, SaleIn, PREFIX};
use crate::fiscal::queue::DEADLINE_MS;
use crate::services::ordering::pricing::{price_basket, Refusal, Want};
use serde_json::{json, Value};

/// A basket a counter rings up, not a catering order.
pub const MAX_LINES: usize = 50;
/// The pricer's own bound (`pricing::price_basket`).
pub const MAX_QTY: i64 = 99;
/// A tablet clock this far AHEAD of the server's is believed (clock drift).
pub const AHEAD_MS: i64 = 5 * 60 * 1000;
/// HOW LONG A TABLET MAY HAVE BEEN OFFLINE, as far as its clock is believed.
/// A HYPOTHESIS about an outage, written down as one: a week is longer than
/// any connection loss a venue keeps trading through, and a sale older than
/// that is far past its 48 h anyway. Outside it the server's clock is used
/// and the tablet's claim is kept beside it (`Conflict{kind:"clock"}`).
pub const MAX_OFFLINE_MS: i64 = 7 * 24 * 3600 * 1000;

/// `[A-Za-z0-9_-]{8,64}`: `newKey()` mints a UUID (or `k-<base36>-<base36>`).
pub fn key_ok(k: &str) -> bool {
    (8..=64).contains(&k.len()) && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The order this sale becomes. Deterministic in the key: the replay's id.
pub fn order_id(key: &str) -> String {
    format!("{PREFIX}{key}")
}

/// The fiscal deadline, 48 h from the sale (Law 87/2019 art. 29).
pub fn deadline(sold_at_ms: i64) -> i64 {
    sold_at_ms.saturating_add(DEADLINE_MS)
}

/// Is the body a sale at all? `Err` is the 400's text.
pub fn check(s: &SaleIn) -> Result<(), String> {
    if !key_ok(&s.sale_key) {
        return Err("sale_key must be 8-64 of [A-Za-z0-9_-]".into());
    }
    if s.method != "cash" {
        return Err("an offline sale is cash: a card needs the network".into());
    }
    if s.currency.len() != 3 || !s.currency.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err("currency must be a three-letter code".into());
    }
    if s.lines.is_empty() || s.lines.len() > MAX_LINES {
        return Err(format!("a sale has 1 to {MAX_LINES} lines"));
    }
    let mut sum: i64 = 0;
    for (i, l) in s.lines.iter().enumerate() {
        if l.product_id.trim().is_empty() || l.product_id.len() > 64 {
            return Err(format!("line {i}: no product"));
        }
        if !(1..=MAX_QTY).contains(&l.quantity) {
            return Err(format!("line {i}: quantity must be 1 to {MAX_QTY}"));
        }
        if l.name.chars().count() > 120 {
            return Err(format!("line {i}: a dish name is at most 120 characters"));
        }
        if l.unit_price < 0 {
            return Err(format!("line {i}: a price is not negative"));
        }
        let gross = l.quantity.checked_mul(l.unit_price).ok_or_else(|| format!("line {i} overflows"))?;
        sum = sum.checked_add(gross).ok_or_else(|| "the total overflows".to_string())?;
    }
    if s.total != sum {
        return Err(format!("the total {} is not the lines' {sum}", s.total));
    }
    if s.total <= 0 {
        return Err("a sale takes money".into());
    }
    Ok(())
}

/// The sale's instant as the order records it, and the conflict when the
/// tablet's clock is not believed (ahead of the server, or older than
/// `MAX_OFFLINE_MS`): the server's own clock then, the claim kept beside it.
pub fn when(claimed: i64, now_ms: i64) -> (i64, Option<Conflict>) {
    if claimed > now_ms + AHEAD_MS || claimed < now_ms - MAX_OFFLINE_MS {
        let c = Conflict { said: format!("the tablet said {claimed}; the server's clock {now_ms} was used"), ..Conflict::said("clock", "") };
        return (now_ms, Some(c));
    }
    // A clock a little ahead is the same instant: never after the sync.
    (claimed.min(now_ms), None)
}

/// THE PRICER'S REFUSAL AS ONE WORD, shared with the tablet: `room/offline-sale.js`
/// answers the same words for the same dishes (`parity.json`), and a synced
/// line that the pricer refuses now carries this word as its conflict.
pub fn refusal_kind(r: &Refusal) -> &'static str {
    match r {
        Refusal::Quantity => "quantity",
        Refusal::Unknown(_) => "unknown",
        Refusal::Unreadable { .. } => "unreadable",
        Refusal::Unavailable(_) => "off_sale",
        Refusal::NoPrice(_) => "no_price",
        Refusal::Options { .. } => "options",
    }
}

/// One line re-priced by THE pricer, as the line was sold (no options).
/// `Ok(name, vat_ppm, station-stamped line)`; a conflict is pushed when the
/// dish moved. The line ALWAYS keeps the price the guest paid.
fn reprice(l: &LineIn, lookup: &dyn Fn(&str) -> Option<String>, conflicts: &mut Vec<Conflict>) -> Value {
    let mut line = json!({
        "product_id": l.product_id, "modifier_ids": [], "quantity": l.quantity,
        "unit_price": l.unit_price, "name": if l.name.trim().is_empty() { l.product_id.clone() } else { l.name.trim().to_string() },
    });
    let want = [Want { product_id: &l.product_id, modifier_ids: &[], quantity: l.quantity }];
    match price_basket(lookup, want) {
        Ok(b) => {
            let p = &b.lines[0];
            if p.unit_price != l.unit_price {
                conflicts.push(Conflict::line("price_changed", &l.product_id, l.unit_price, Some(p.unit_price)));
            }
            line["name"] = json!(p.name);
            if let Some(r) = p.vat_ppm {
                line["vat_ppm"] = json!(r.0);
            }
            crate::bell_route::stamp_line(&mut line, p.station);
        }
        Err(r) => {
            conflicts.push(Conflict { said: r.text(), ..Conflict::line(refusal_kind(&r), &l.product_id, l.unit_price, None) });
        }
    }
    line
}

/// The order envelope, in the shape `ebills::map::to_order` gives a counter
/// sale: born `PICKED_UP` (the one terminal that took money and had no
/// courier leg), paid in cash at the sale's instant -- the payment the till's
/// drawer reads (`command::till::fold::cash_payments`) -- with the paid prices,
/// and `offline` naming the key, the clocks and every conflict.
///
/// LAW 3 BY CONSTRUCTION: `total = subtotal = Σ quantity × unit_price`, no
/// discount, fee or tip.
#[allow(clippy::too_many_arguments)]
pub fn envelope(s: &SaleIn, by: &str, sold_at: i64, now_ms: i64, lookup: &dyn Fn(&str) -> Option<String>, mut conflicts: Vec<Conflict>) -> Value {
    let items: Vec<Value> = s.lines.iter().map(|l| reprice(l, lookup, &mut conflicts)).collect();
    let mut offline = json!({
        "key": s.sale_key, "sold_at_ms": sold_at, "synced_at_ms": now_ms,
        "fiscal_deadline_ms": deadline(sold_at), "conflicts": conflicts,
    });
    if sold_at != s.sold_at_ms {
        offline["claimed_at_ms"] = json!(s.sold_at_ms);
    }
    if let Some(v) = s.menu_version {
        offline["menu_version"] = json!(v);
    }
    json!({
        "id": order_id(&s.sale_key), "status": "PICKED_UP",
        "channel": crate::services::ordering::channel::CONSOLE,
        "customer_id": null, "items": items, "subtotal": s.total, "discount": 0,
        "total": s.total, "delivery_fee": 0, "tip": 0, "created_at_ms": sold_at,
        "price_trusted": true, "location_id": s.location_id,
        "contact": { "name": "", "phone": "" }, "currency": s.currency,
        "fulfilment": { "kind": "pickup", "code": "", "fee": 0 },
        "payment": "cash", "payment_status": "paid", "placed_by": by,
        "payments": [{ "by": by, "amount": s.total, "method": "cash", "currency": s.currency, "at": sold_at }],
        "offline": offline,
    })
}

/// The shelf's input: each line's ledger record (`basket.ledger`) and quantity.
/// A dish the catalogue no longer has draws nothing -- a guess would take the
/// wrong thing off the shelf; its line carries the conflict that says so.
pub fn bom_lines(s: &SaleIn, ledger: &dyn Fn(&str) -> Option<String>) -> Vec<(String, i64)> {
    s.lines.iter().filter_map(|l| Some((ledger(&l.product_id)?, l.quantity))).collect()
}
