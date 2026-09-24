//! PURE. The stamp card as a fold, its three settings, and the one place a
//! full card is spent (see `mod.rs` for what it is and what it refuses to be).
//!
//! THE COUNT IS NEVER STORED. `count` walks the orders every time it is asked,
//! for the reason `promo.rs:17-21` gives for the used-count: a counter kept
//! beside the orders is a second number that can disagree with them. What the
//! log DOES record is a spent card -- `"loyalty": {"n", "discount"}` on the
//! order that took the reward -- because that is a fact about an order (its
//! price), not a copy of a fold.
//!
//! WHAT A STAMP IS:
//! * an order of theirs that is DELIVERED or PICKED_UP -- a cancelled, rejected
//!   or refunded one is not, because the venue sold nothing;
//! * a SITTING of theirs (a round they placed at the table, A9) that is closed
//!   and PAID: every round terminal and Σ paid ≥ the bill (`sitting::open`),
//!   with a bill above zero. ONE stamp per sitting, however many rounds.
//!
//! A SPENT CARD is an order of theirs carrying `loyalty.n`, while it still
//! took money: it takes `n` stamps off. An order the venue refused gives the
//! card back, because the reward was never received.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::command::sitting::{self, Round};
use crate::hubdo::OrderView;
use crate::services::orders::status::took_money;

/// The three settings (`dowiz_hub::settings::KNOWN`).
pub const ENABLED: &str = "loyalty.stamps.enabled";
pub const STAMPS_N: &str = "loyalty.stamps.n";
pub const REWARD: &str = "loyalty.stamps.reward_minor";
/// A card of one stamp is a discount on every order; a card of more than
/// twenty is one nobody completes.
pub const N_RANGE: RangeInclusive<i64> = 2..=20;
/// The envelope key a spent card is recorded under.
pub const RECORD: &str = "loyalty";

/// A venue's card, when it has one that can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    pub n: i64,
    pub reward_minor: i64,
}

/// Refuse a malformed value WHILE THE OWNER CAN FIX IT (the `tax_cfg::validate`
/// shape). An empty value clears the setting and is always accepted; any other
/// key is not this function's business.
pub fn validate(key: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Ok(());
    }
    match key {
        ENABLED => match value {
            "0" | "1" => Ok(()),
            _ => Err(format!("{key}: 0 or 1")),
        },
        STAMPS_N => match value.parse::<i64>() {
            Ok(n) if N_RANGE.contains(&n) => Ok(()),
            _ => Err(format!("{key}: a whole number from {} to {}", N_RANGE.start(), N_RANGE.end())),
        },
        REWARD => match value.parse::<i64>() {
            Ok(m) if m > 0 => Ok(()),
            _ => Err(format!("{key}: a whole amount in minor units, more than 0")),
        },
        _ => Ok(()),
    }
}

/// The venue's card from a settings reader (`Settings::known`). `None` when it
/// is off, and when a value that `validate` should have refused is stored: a
/// card that cannot say what it gives gives nothing.
pub fn config(known: impl Fn(&str) -> String) -> Option<Card> {
    if !matches!(known(ENABLED).trim(), "1" | "true" | "yes" | "on") {
        return None;
    }
    let n = known(STAMPS_N).trim().parse::<i64>().ok().filter(|n| N_RANGE.contains(n))?;
    let reward_minor = known(REWARD).trim().parse::<i64>().ok().filter(|m| *m > 0)?;
    Some(Card { n, reward_minor })
}

/// What the Worker hands the object at placement: the card, and which orders
/// are this person's. `phones` are the raw spellings, as the envelopes carry
/// them, of every key in the person's alias circle -- the object holds no
/// signing secret, so the Worker resolves keys and the object matches strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StampIn {
    pub n: i64,
    pub reward_minor: i64,
    pub location_id: String,
    pub phones: Vec<String>,
}

/// The phone an order was placed under, as typed.
pub fn phone_of(o: &Value) -> Option<&str> {
    o.pointer("/contact/phone").and_then(Value::as_str).filter(|p| !p.trim().is_empty())
}

fn at_venue(o: &Value, location_id: &str) -> bool {
    o.get("location_id").and_then(Value::as_str) == Some(location_id)
}

/// `mine` for the object: this venue, one of these spellings.
pub fn by_phones<'a>(location_id: &'a str, phones: &'a [String]) -> impl Fn(&Value) -> bool + 'a {
    move |o| at_venue(o, location_id) && phone_of(o).is_some_and(|p| phones.iter().any(|x| x == p))
}

/// Every distinct spelling at this venue that `in_circle` says is this person,
/// plus the one being placed. The Worker's half of `StampIn`.
pub fn spellings(listed: &[OrderView], location_id: &str, in_circle: impl Fn(&str) -> bool, placing: &str) -> Vec<String> {
    let mut out = vec![placing.to_string()];
    for v in listed {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        if let Some(p) = phone_of(&o).filter(|_| at_venue(&o, location_id)) {
            if !out.iter().any(|x| x == p) && in_circle(p) {
                out.push(p.to_string());
            }
        }
    }
    out
}

fn status(o: &Value) -> &str {
    o.get("status").and_then(Value::as_str).unwrap_or("")
}

fn at_ms(o: &Value) -> i64 {
    o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0)
}

fn sitting_of(o: &Value) -> Option<&str> {
    o.get("sitting_id").and_then(Value::as_str)
}

/// DELIVERED or PICKED_UP: the kernel's names for "they had it".
fn had_it(o: &Value) -> bool {
    use dowiz_kernel::OrderStatus as S;
    matches!(S::from_str(status(o)), Some(S::Delivered | S::PickedUp))
}

/// The stamps a spent card took, while the order that spent it took money.
fn spent(o: &Value) -> Option<i64> {
    took_money(status(o)).then(|| o.pointer("/loyalty/n").and_then(Value::as_i64)).flatten().filter(|n| *n > 0)
}

/// What the card took off this order, in minor units (0: it did not).
pub fn used(o: &Value) -> i64 {
    o.pointer("/loyalty/discount").and_then(Value::as_i64).unwrap_or(0)
}

/// Closed, paid, and not free: the sitting's one stamp.
fn sitting_paid(rounds: &[Round<'_>]) -> bool {
    !rounds.is_empty() && !sitting::open(rounds) && sitting::bill(rounds) > 0
}

/// THE FOLD: stamps since the last spent card, before `mod N`. Events are
/// ordered by time, a spend before a stamp at the same instant, and the count
/// never goes below zero (a card spent under a larger `n` than today's).
pub fn count(listed: &[OrderView], mine: impl Fn(&Value) -> bool) -> i64 {
    let parsed: Vec<(&OrderView, Value)> =
        listed.iter().filter_map(|v| Some((v, serde_json::from_str::<Value>(&v.order_json).ok()?))).collect();
    let mut events: Vec<(i64, u8, i64)> = Vec::new();
    let mut sittings: BTreeMap<String, (bool, Vec<Round<'_>>)> = BTreeMap::new();
    for (v, o) in &parsed {
        let me = mine(o);
        if me {
            if let Some(k) = spent(o) {
                events.push((at_ms(o), 0, -k));
            }
        }
        match sitting_of(o) {
            Some(s) => {
                let e = sittings.entry(s.to_string()).or_insert((false, Vec::new()));
                e.0 |= me;
                e.1.push(Round { view: *v, order: o.clone() });
            }
            None if me && had_it(o) => events.push((at_ms(o), 1, 1)),
            None => {}
        }
    }
    for (me, rounds) in sittings.values() {
        if *me && sitting_paid(rounds) {
            let last = rounds.iter().map(|r| at_ms(&r.order)).max().unwrap_or(0);
            events.push((last, 1, 1));
        }
    }
    events.sort();
    events.into_iter().fold(0, |c, (_, _, d)| (c + d).max(0))
}

/// `stamps(orders, key, n) -> (have, redeemable)` of §3.5: the stamps on the
/// card now, and how many full cards are waiting to be spent.
pub fn stamps(listed: &[OrderView], mine: impl Fn(&Value) -> bool, n: i64) -> (i64, i64) {
    let n = n.max(1);
    let c = count(listed, mine);
    (c % n, c / n)
}

/// THE ONE PLACE A FULL CARD IS SPENT: inside the object's turn, over the log
/// it is about to append to, so of two placements racing for one card the
/// second sees the first's `loyalty` record and is priced without it. The
/// reward is a `Fixed` promo through the hub's engine, on what is left of the
/// subtotal after any code, and never more than that.
pub fn redeem(listed: &[OrderView], s: &StampIn, left: i64, now_ms: i64) -> Option<i64> {
    let (_, owed) = stamps(listed, by_phones(&s.location_id, &s.phones), s.n);
    if owed < 1 {
        return None;
    }
    let reward = dowiz_hub::promo::Promo {
        code: "STAMPCARD".into(),
        kind: dowiz_hub::promo::Kind::Fixed,
        value: s.reward_minor,
        min_order: 0,
        from_ms: None,
        until_ms: None,
        max_uses: None,
        active: true,
    };
    reward.redeem(left, now_ms, 0).ok().filter(|cut| *cut > 0)
}

/// Patch a spent card into the envelope `command::place::decide` is building:
/// the record, the discount beside any code's, and the total.
pub fn apply(envelope: &mut Value, listed: &[OrderView], s: &StampIn, subtotal: i64, fee: i64, tip: i64, now_ms: i64) {
    let code_cut = envelope.get("discount").and_then(Value::as_i64).unwrap_or(0);
    if let Some(cut) = redeem(listed, s, subtotal - code_cut, now_ms) {
        envelope[RECORD] = json!({ "n": s.n, "discount": cut });
        envelope["discount"] = json!(code_cut + cut);
        envelope["total"] = json!(subtotal - code_cut - cut + fee + tip);
    }
}

/// What the customer's own order page shows: `have` of `n`, counting THIS
/// order while it is live and not yet stamped -- the card starts at 1 with
/// the order just placed, which is true, not a gift. `1..=n`, a full card
/// reads `n / n` until it is spent.
pub fn shown(listed: &[OrderView], mine: impl Fn(&Value) -> bool, n: i64, this: &Value) -> i64 {
    let n = n.max(1);
    let c = count(listed, &mine);
    let stamped = match sitting_of(this) {
        Some(s) => sitting_paid(&sitting::rounds(listed, s)),
        None => had_it(this),
    };
    let x = c + i64::from(took_money(status(this)) && !stamped);
    if x == 0 { 0 } else { (x - 1) % n + 1 }
}

#[cfg(test)]
mod tests;
