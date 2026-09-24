//! AN AGGREGATOR ORDER, ENTERED BY HAND (BLUEPRINT-OPERATIONAL-BLIND-SPOTS
//! §2.9, P2-3) — the part of §2.9 that needs no partner credentials.
//!
//! §2.9's inbound is an adapter per platform producing a `place` command with
//! the channel, `price_trusted: false` and the platform's id in `external{}`,
//! born CONFIRMED because the platform took the acceptance in-app. There is no
//! partner access (no Wolt Order API key, no Glovo Partners API), so the
//! adapter's OUTPUT is built here from what a member of staff reads off the
//! platform's tablet, and everything after it is the same path as any order:
//! `command::place::decide` reserves on the same ledger, stamps the tax and
//! appends `Placed`.
//!
//! WHAT IS DECIDED HERE:
//! - the order id is `<channel>-<external id>` (URL-safe as it stands), so the platform's id IS the
//!   idempotency key: entering it twice returns the first order, never a
//!   second one (`existing`, checked in the object's turn);
//! - the money is the platform's: line prices as charged, the platform's
//!   discount, and a `total` that must equal lines − discount. Anything else
//!   is refused, so conservation law 3 holds by construction. The commission
//!   is NOT in the total and is not recorded (the agreement is unread);
//! - the source's profile (`channel.rs`) decides trust and delivery:
//!   `price_trusted: false`, a masked contact, payment `platform` (the
//!   customer paid the platform: no cash for a drawer or a courier), and a
//!   PICKUP, because the platform's courier collects it at the counter.

use super::Refused;
use crate::hubdo::OrderView;
use crate::services::ordering::channel;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// A platform order id: what its tablet shows. Bounded, and plain so it can be
/// half of an order id.
pub const EXTERNAL_MAX: usize = 64;
/// A line's quantity, as the storefront bounds it.
pub const QTY_MAX: i64 = 99;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Line {
    pub product_id: String,
    pub quantity: i64,
    /// What the platform charged for one, in the venue's minor units.
    pub unit_price: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub channel: String,
    pub external_id: String,
    pub lines: Vec<Line>,
    /// The platform's own discount on this order, if its tablet shows one.
    #[serde(default)]
    pub discount: i64,
    /// What the customer was charged, as the platform shows it.
    pub total: i64,
}

/// What the object answers: the order as stored, and whether it already was.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorOut {
    pub stored: String,
    pub existing: bool,
}

/// The order id for a platform's order, refusing a non-marketplace channel and
/// an id that is not one.
pub fn order_id(ch: &str, external_id: &str) -> Result<String, Refused> {
    let Some(ch) = channel::marketplace_word(ch) else {
        return Err(Refused::Invalid(format!("{ch:?} is not a marketplace")));
    };
    let x = external_id.trim();
    let plain = x.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if x.is_empty() || x.len() > EXTERNAL_MAX || !plain {
        return Err(Refused::Invalid(format!(
            "the platform's order id is 1-{EXTERNAL_MAX} letters, digits, '-' or '_'"
        )));
    }
    // A DASH, NOT A COLON: the id travels in every order URL the console and
    // the room build (`/owner/orders/<id>/action`, `/staff/orders/<id>/refund`),
    // and `encodeURIComponent` turns ':' into `%3A`, which the router hands to
    // the handler undecoded -- every action on a Wolt order answered 404
    // (2026-09-24). A channel word has no dash, so `<channel>-` stays a prefix.
    Ok(format!("{ch}-{x}"))
}

/// Already entered? The first order is the answer to the second entry.
pub fn existing<'a>(listed: &'a [OrderView], id: &str) -> Option<&'a OrderView> {
    listed.iter().find(|o| o.order_id == id)
}

/// THE SAME PLATFORM NUMBER, ENTERED AGAIN. Same lines (as a multiset: the
/// order they were typed in is not content), same discount, same total: the
/// same entry, and the first order is the answer. Anything else is a named
/// CONFLICT -- one platform order cannot be two baskets, and silently handing
/// back the first would hide the typo from the person who made it.
pub fn same_entry(stored: &str, entered: &str) -> Result<(), Refused> {
    fn content(s: &str) -> Option<(Vec<(String, i64, i64)>, i64, i64)> {
        let v: Value = serde_json::from_str(s).ok()?;
        let mut lines = v
            .get("items")?
            .as_array()?
            .iter()
            .map(|i| {
                Some((
                    i.get("product_id")?.as_str()?.to_string(),
                    i.get("quantity")?.as_i64()?,
                    i.get("unit_price")?.as_i64()?,
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        lines.sort();
        Some((lines, v.get("discount").and_then(Value::as_i64).unwrap_or(0), v.get("total")?.as_i64()?))
    }
    let (Some(was), Some(now)) = (content(stored), content(entered)) else {
        return Err(Refused::Conflict("this platform order is already entered and cannot be compared".into()));
    };
    if was == now {
        return Ok(());
    }
    Err(Refused::Conflict(format!(
        "this platform order was already entered with a total of {} and {} line(s); \
         this entry says {} and {} line(s). Correct the first order, do not enter it twice",
        was.2,
        was.0.len(),
        now.2,
        now.0.len()
    )))
}

/// The envelope `place` will store, and its subtotal. Pure: the clock and the
/// venue come in as arguments, the dish names from the catalogue the caller
/// holds.
pub fn envelope(
    e: &Entry,
    location_id: &str,
    currency: &str,
    names: &[(String, String)],
    now_ms: i64,
) -> Result<(String, Value, i64), Refused> {
    let id = order_id(&e.channel, &e.external_id)?;
    let profile = channel::profile(&e.channel).ok_or_else(|| Refused::Invalid("no profile".into()))?;
    if e.lines.is_empty() {
        return Err(Refused::Invalid("an order has at least one line".into()));
    }
    let mut subtotal: i64 = 0;
    for l in &e.lines {
        if !(1..=QTY_MAX).contains(&l.quantity) || l.unit_price < 0 || l.product_id.trim().is_empty() {
            return Err(Refused::Invalid(format!("line {:?} is not a line", l.product_id)));
        }
        let v = l.unit_price.checked_mul(l.quantity).ok_or_else(|| Refused::Invalid("a line overflows".into()))?;
        subtotal = subtotal.checked_add(v).ok_or_else(|| Refused::Invalid("the lines overflow".into()))?;
    }
    if e.discount < 0 || e.discount > subtotal {
        return Err(Refused::Invalid("the discount is not within the lines".into()));
    }
    // LAW 3, BY CONSTRUCTION: the platform's total is its lines less its
    // discount. A commission netted into it would be refused here.
    if e.total != subtotal - e.discount {
        return Err(Refused::Invalid(format!(
            "the platform's total {} is not its lines {subtotal} less its discount {}",
            e.total, e.discount
        )));
    }
    let lines: Vec<Value> = e
        .lines
        .iter()
        .map(|l| json!({ "product_id": l.product_id, "quantity": l.quantity, "unit_price": l.unit_price }))
        .collect();
    let kernel = dowiz_kernel::json_api::place_order_at(
        id.clone(),
        None,
        &Value::Array(lines).to_string(),
        now_ms,
        Some(e.channel.clone()),
    )
    .map_err(Refused::Invalid)?;
    let mut o: Value = serde_json::from_str(&kernel).map_err(|e| Refused::Append(format!("kernel json: {e}")))?;
    if let Some(items) = o.get_mut("items").and_then(Value::as_array_mut) {
        for line in items.iter_mut() {
            let pid = line.get("product_id").and_then(Value::as_str).unwrap_or("").to_string();
            if let Some((_, n)) = names.iter().find(|(p, _)| *p == pid) {
                line["name"] = json!(n);
            }
        }
    }
    // Accepted in the platform's app before anyone here saw it: CONFIRMED.
    o["status"] = json!("CONFIRMED");
    crate::live_eta::stamp(&mut o, "CONFIRMED", now_ms);
    o["subtotal"] = json!(subtotal);
    o["discount"] = json!(e.discount);
    o["total"] = json!(e.total);
    o["delivery_fee"] = json!(0);
    o["tip"] = json!(0);
    o["price_trusted"] = json!(profile.priced_by_us);
    o["location_id"] = json!(location_id);
    o["currency"] = json!(currency);
    o["payment"] = json!("platform");
    o["payment_status"] = json!("paid");
    o["contact"] = json!({ "name": "", "phone": "" });
    o["fulfilment"] = if profile.delivered_by_us {
        return Err(Refused::Invalid("a marketplace the venue delivers for is not modelled".into()));
    } else {
        json!({ "kind": "pickup", "code": "", "fee": 0 })
    };
    o["external"] = json!({ "source": e.channel, "order_id": e.external_id.trim() });
    Ok((id, o, subtotal))
}

/// THE KITCHEN'S BELL for an entered order, rendered the way a placed order's
/// is (`notify::order_text`) so Telegram and the print rail see it through the
/// same outbox. Its first line names the platform and ITS order number: that
/// is what the platform's courier says at the counter, not dowiz's id.
pub fn bell(env: &Value, e: &Entry, names: &[(String, String)], currency: &str, venue: &str) -> String {
    let lines: Vec<crate::notify::LineOut> = e
        .lines
        .iter()
        .map(|l| crate::notify::LineOut {
            name: names
                .iter()
                .find(|(p, _)| *p == l.product_id)
                .map(|(_, n)| n.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| l.product_id.clone()),
            quantity: l.quantity,
            unit_price: l.unit_price,
        })
        .collect();
    format!(
        "{} {}\n{}",
        e.channel.to_uppercase(),
        e.external_id.trim(),
        crate::notify::order_text(env, &lines, currency, venue)
    )
}

#[cfg(test)]
mod tests;
