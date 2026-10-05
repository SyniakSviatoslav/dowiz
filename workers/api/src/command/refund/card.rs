//! THE CARD'S SHARE OF A REFUND (W-REFUND). PURE: what may go back to the
//! card, the record of each card refund on the order, what a Stripe answer or
//! event does to that record, and when the order may end by itself.
//!
//! THE RECORD is `refund.card` on the order (carried by `hubstore/carry.rs`
//! under `refund`): `{pi, paid, currency, attempts: [{n, amount, key, status,
//! id?, failure?, by, at}], seen: [event fingerprints]}`. `paid` is what the
//! card webhook RECEIVED (`amount_received`, whole lek since `stripe::refund`
//! converts), never recomputed from items. An attempt is `queued` until the
//! drain hears from Stripe, then Stripe's own word: pending, requires_action,
//! succeeded, failed, canceled.
//!
//! WHAT MAY GO BACK: `paid` minus what succeeded minus what is still in flight.
//! In flight counts, or two taps a second apart would queue the card twice.
//!
//! THE ORDER ENDS BY ITSELF only when Stripe says `succeeded` for the whole
//! card part AND the card was all the money taken: a cash part still needs the
//! person who handed it back (`complete: true`).

use super::{decide, money_taken, step, RefundIn, Refused, Written};
use crate::hubdo::OrderView;
use crate::stripe::refund::Job;
use dowiz_hub::EventKind;
use dowiz_kernel::order_machine::OrderStatus;
use serde_json::{json, Value};

/// Statuses that still hold money on its way back.
pub const IN_FLIGHT: [&str; 3] = ["queued", "pending", "requires_action"];
/// Stripe's refund statuses, and ours before Stripe answered.
const KNOWN: [&str; 6] = ["queued", "pending", "requires_action", "succeeded", "failed", "canceled"];
/// Event fingerprints kept on the record for de-duplication.
pub const SEEN_KEPT: usize = 20;
/// The console's sentence when the venue's Worker holds no Stripe key.
pub const NO_KEY: &str = "card refund needs Stripe connected; refund the card by hand in Stripe, then mark it refunded";

/// The Idempotency-Key of card refund `n` of `order_id` at `venue`: the same
/// three always give the same key, so a retry can never refund twice.
pub fn idem_key(venue: &str, order_id: &str, n: u32) -> String {
    let h = crate::auth::sha256_hex(&format!("dowiz-refund\n{venue}\n{order_id}\n{n}"));
    format!("dowiz-rf-{}", &h[..40])
}

/// What the card paid: the intent and what it RECEIVED, or `None`.
pub fn card_paid(order: &Value) -> Option<(String, i64)> {
    let pi = order.get("payment_intent").and_then(Value::as_str).filter(|p| !p.is_empty())?;
    let paid = order.get("amount_received").and_then(Value::as_i64).filter(|a| *a > 0)?;
    Some((pi.to_string(), paid))
}

fn attempts(order: &Value) -> Vec<Value> {
    order.pointer("/refund/card/attempts").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn status_of(a: &Value) -> &str {
    a.get("status").and_then(Value::as_str).unwrap_or("")
}

/// (succeeded, in flight), in the order's units.
pub fn sums(order: &Value) -> (i64, i64) {
    attempts(order).iter().fold((0, 0), |(done, flying), a| {
        let amt = a.get("amount").and_then(Value::as_i64).unwrap_or(0).max(0);
        match status_of(a) {
            "succeeded" => (done.saturating_add(amt), flying),
            s if IN_FLIGHT.contains(&s) => (done, flying.saturating_add(amt)),
            _ => (done, flying),
        }
    })
}

/// What a planned card refund adds: the new `refund.card` record and the job.
pub enum Plan {
    /// No card money on this order: nothing for Stripe.
    None,
    /// Card money, no Stripe key: the record says so; a person refunds by hand.
    Manual(Value),
    Queue(Value, Job),
}

/// Plan one more card refund of `amount` (`None` = all that is left).
pub fn plan(order: &Value, venue: &str, venue_currency: &str, amount: Option<i64>, by: &str, now_ms: i64, stripe_on: bool) -> Result<Plan, Refused> {
    let Some((pi, paid)) = card_paid(order) else {
        return match amount {
            Some(_) => Err(Refused::Invalid("this order took no card money".into())),
            None => Ok(Plan::None),
        };
    };
    let currency = super::currency_of(order, venue_currency);
    let mut rec = order.pointer("/refund/card").cloned().filter(Value::is_object).unwrap_or_else(|| json!({}));
    rec["pi"] = json!(pi);
    rec["paid"] = json!(paid);
    rec["currency"] = json!(currency);
    if !stripe_on {
        if amount.is_some() {
            return Err(Refused::Conflict(NO_KEY.into()));
        }
        rec["manual"] = json!(true);
        return Ok(Plan::Manual(rec));
    }
    if crate::stripe::refund::to_stripe(1, &currency).is_none() {
        return Err(Refused::Invalid(format!("Stripe cannot refund in {currency:?}")));
    }
    let (done, flying) = sums(order);
    let left = paid - done - flying;
    let amt = amount.unwrap_or(left);
    if left <= 0 {
        return Err(Refused::Conflict("nothing is left on the card to refund".into()));
    }
    if amt <= 0 {
        return Err(Refused::Invalid("a card refund is a positive amount".into()));
    }
    if amt > left {
        return Err(Refused::Invalid(format!("at most {left} can go back to the card")));
    }
    let mut list = attempts(order);
    let n = list.len() as u32 + 1;
    let key = idem_key(venue, order.get("id").and_then(Value::as_str).unwrap_or(""), n);
    list.push(json!({"n": n, "amount": amt, "key": key, "status": "queued", "by": by, "at": now_ms}));
    rec["attempts"] = json!(list);
    let order_id = order.get("id").and_then(Value::as_str).unwrap_or("").to_string();
    let job = Job { venue: venue.into(), order_id, pi, amount: amt, currency, n, key };
    Ok(Plan::Queue(rec, job))
}

/// THE REFUND, CARD INCLUDED, over the images in memory. Starting a refund
/// is `refund::decide` plus the card's record in the same turn; on an order
/// already REFUNDING, `card_amount` asks for one more card refund (the rest of
/// a partial one, or a try again after `failed`). Every refusal comes before
/// the first append.
pub fn decide_card(
    hub: &mut dowiz_hub::Hub,
    stock: &mut dowiz_hub::stock::StockLog,
    current: Option<&OrderView>,
    input: &RefundIn,
    venue_currency: &str,
    stripe_on: bool,
) -> Result<(Value, Vec<Written>, Option<Job>), Refused> {
    let Some(cur) = current else { return Err(Refused::NotFound) };
    let old: Value = serde_json::from_str(&cur.order_json).map_err(|e| Refused::Append(format!("order json unreadable: {e}")))?;
    let more = !input.complete && input.card_amount.is_some() && old.get("status").and_then(Value::as_str) == Some("REFUNDING");
    // THE COURIER AT THE DOOR never sends money back to a card: the venue
    // does, from the order sheet, once the refusal is on the order.
    if input.complete || input.at_door {
        return decide(hub, stock, current, input, venue_currency).map(|(o, w)| (o, w, None));
    }
    if old.get("location_id").and_then(Value::as_str) != Some(input.location_id.as_str()) {
        return Err(Refused::NotFound);
    }
    if input.by.trim().is_empty() {
        return Err(Refused::Invalid("a refund names who is refunding".into()));
    }
    // Planned on the order as it was, so its refusals come before any append.
    let planned = plan(&old, &input.location_id, venue_currency, input.card_amount, &input.by, input.now_ms, stripe_on)?;
    let (mut order, mut written, seq) = if more {
        (old.clone(), Vec::new(), cur.seq)
    } else {
        let (o, w) = decide(hub, stock, current, input, venue_currency)?;
        let s = w.last().map_or(cur.seq, |x| x.2);
        (o, w, s)
    };
    let (rec, job) = match planned {
        Plan::None if more => return Err(Refused::Invalid("this order took no card money".into())),
        Plan::None => return Ok((order, written, None)),
        Plan::Manual(r) => (r, None),
        Plan::Queue(r, j) => (r, Some(j)),
    };
    // NOTHING TAKEN, ALREADY ENDED: `decide` closed it in this turn, so there
    // is nothing on the card to send back.
    if order.get("status").and_then(Value::as_str) != Some("REFUNDING") {
        return Ok((order, written, None));
    }
    let before = order.clone();
    order["refund"]["card"] = rec;
    let body = crate::fold::delta(&before, &order).to_string();
    let seq = super::next_seq(seq, input.now_ms);
    hub.append(EventKind::Noted, &input.order_id, &body, seq, [0u8; 32])
        .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    written.push((EventKind::Noted, body, seq));
    Ok((order, written, job))
}

/// One thing Stripe said about one refund: the drain's answer or a webhook.
#[derive(Debug, Clone, Default)]
pub struct Update {
    /// `metadata.venue` on a refund this platform made; `None` on one made by hand.
    pub venue: Option<String>,
    pub pi: Option<String>,
    pub refund_id: Option<String>,
    /// `metadata.key`: which of our attempts this is.
    pub key: Option<String>,
    pub status: String,
    /// In the order's units (already `from_stripe`).
    pub amount: Option<i64>,
    pub failure: Option<String>,
    pub fingerprint: Option<String>,
    pub at: i64,
}

fn rank(s: &str) -> u8 {
    match s {
        "queued" => 0,
        "pending" | "requires_action" => 1,
        _ => 2,
    }
}

/// Apply `up` to `order` at `venue`. `Ok(None)`: nothing new (a duplicate
/// event, or an older word than the record holds). A refund of ANOTHER
/// venue or another payment is refused, never written here.
pub fn apply(order: &Value, venue: &str, up: &Update) -> Result<Option<Value>, Refused> {
    if order.get("location_id").and_then(Value::as_str) != Some(venue) {
        return Err(Refused::NotFound);
    }
    if up.venue.as_deref().is_some_and(|v| v != venue) {
        return Err(Refused::Invalid("this refund belongs to another venue".into()));
    }
    let Some((pi, paid)) = card_paid(order) else {
        return Err(Refused::Invalid("this order took no card money".into()));
    };
    if up.pi.as_deref().is_some_and(|p| p != pi) {
        return Err(Refused::Invalid("this refund is for another payment".into()));
    }
    if !KNOWN.contains(&up.status.as_str()) {
        return Err(Refused::Invalid(format!("{:?} is not a refund status", up.status)));
    }
    let mut rec = order.pointer("/refund/card").cloned().filter(Value::is_object).unwrap_or_else(|| json!({"pi": pi, "paid": paid}));
    let mut seen: Vec<Value> = rec.get("seen").and_then(Value::as_array).cloned().unwrap_or_default();
    if let Some(fp) = &up.fingerprint {
        if seen.iter().any(|s| s.as_str() == Some(fp)) {
            return Ok(None);
        }
    }
    let mut list = attempts(order);
    let at = list.iter().position(|a| {
        up.key.as_deref().is_some_and(|k| a.get("key").and_then(Value::as_str) == Some(k))
            || up.refund_id.as_deref().is_some_and(|i| a.get("id").and_then(Value::as_str) == Some(i))
    });
    let i = match at {
        Some(i) => i,
        // A REFUND MADE BY HAND IN STRIPE: recorded, so the sums stay true.
        None => {
            list.push(json!({"n": list.len() + 1, "amount": up.amount.unwrap_or(0), "by": "stripe", "at": up.at, "status": "pending"}));
            list.len() - 1
        }
    };
    let a = &mut list[i];
    let was = status_of(a).to_string();
    let same = was == up.status && up.refund_id.as_deref().is_none_or(|r| a.get("id").and_then(Value::as_str) == Some(r));
    if rank(&up.status) < rank(&was) || (same && up.failure.is_none()) {
        return Ok(None);
    }
    a["status"] = json!(up.status);
    if let Some(id) = &up.refund_id {
        a["id"] = json!(id);
    }
    if let Some(f) = &up.failure {
        a["failure"] = json!(f);
    }
    a["told_at"] = json!(up.at);
    if let Some(fp) = &up.fingerprint {
        seen.push(json!(fp));
        while seen.len() > SEEN_KEPT {
            seen.remove(0);
        }
        rec["seen"] = json!(seen);
    }
    rec["attempts"] = json!(list);
    let mut out = order.clone();
    out["refund"]["card"] = rec;
    Ok(Some(out))
}

/// The card part was all the money taken, and Stripe says it is back.
pub fn covered(order: &Value) -> bool {
    let Some((_, paid)) = card_paid(order) else { return false };
    let owed = money_taken(order).map(|(o, _)| o).unwrap_or(i64::MAX);
    order.get("status").and_then(Value::as_str) == Some("REFUNDING") && sums(order).0 >= paid && owed <= paid
}

/// REFUNDING → COMPENSATED_REFUND by Stripe's word, as one `Advanced` delta,
/// or `None` when the order is not covered.
pub fn close(order: &Value, now_ms: i64) -> Result<Option<String>, Refused> {
    if !covered(order) {
        return Ok(None);
    }
    let before = order.clone();
    let mut o = order.clone();
    o["refund"]["returned"] = json!({"by": "stripe", "at": now_ms});
    step(&mut o, OrderStatus::CompensatedRefund, now_ms)?;
    Ok(Some(crate::fold::delta(&before, &o).to_string()))
}

#[cfg(test)]
mod tests;
