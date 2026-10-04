//! PURE. THE BAG INSERT'S WELCOME OFFER (W-QR, operator 2026-10-04 "qr-коди так").
//!
//! WHAT IT IS. One public offer the owner sets for guests who arrive from the
//! QR card in a delivery bag (`?src=bag`): the SAME offer for everyone who
//! scans, never personalised. It is applied once per phone -- the phone the
//! guest types at checkout, through the same alias circle as the stamp card
//! (`loyalty::handlers`) -- inside the object's turn, over the log it is about
//! to append to, so of two placements racing on one phone the second is
//! placed WITHOUT it and says why (`welcome_refused`).
//!
//! WHAT IT IS NOT. Never a price: the menu price of every dish stays what it
//! is (a marketplace contract may demand price parity, and an extra is not a
//! price). The bonus is a discount recorded on the order that took it, beside
//! a code's and a full stamp card's, measured on the subtotal the ONE pricer
//! (`ordering::pricing`) computed. No counter is stored: "used" is a fold over
//! the orders, like the stamp card's count.
//!
//! THE THREE KINDS:
//! * `fixed`  -- N off a basket of at least `min` (through `dowiz_hub::promo`);
//! * `gift`   -- one named dish is on the house when it is in the basket;
//! * `stamps` -- the order counts as two stamps on the venue's stamp card.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::hubdo::OrderView;
use crate::services::orders::status::took_money;

/// The offer, one JSON value in the venue's settings image.
pub const OFFER: &str = "bag.welcome";
/// The marketplace commission the OWNER typed, whole percent; empty = unknown.
pub const COMMISSION: &str = "bag.commission_pct";
/// The envelope key a granted offer is recorded under.
pub const RECORD: &str = "welcome";
/// The envelope key that says, politely, why it was not granted.
pub const REFUSED: &str = "welcome_refused";
/// A fixed bonus above this is a typo, not a welcome (100 000 lek).
pub const MAX_VALUE: i64 = 100_000;
/// A commission the owner may type: 1..=60 %.
pub const PCT: std::ops::RangeInclusive<i64> = 1..=60;

/// The offer as stored and as the storefront reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Offer {
    Fixed { value: i64, min: i64 },
    Gift { product: String },
    Stamps,
}

/// What the owner's form sends. AN UNKNOWN FIELD IS A REFUSAL: a form that
/// gives money away must not quietly drop the field that limits it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfferIn {
    pub kind: String,
    #[serde(default)]
    pub value: Option<i64>,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub product: Option<String>,
}

impl Offer {
    pub fn kind(&self) -> &'static str {
        match self {
            Offer::Fixed { .. } => "fixed",
            Offer::Gift { .. } => "gift",
            Offer::Stamps => "stamps",
        }
    }

    pub fn to_json(&self) -> Value {
        match self {
            Offer::Fixed { value, min } => json!({ "kind": "fixed", "value": value, "min": min }),
            Offer::Gift { product } => json!({ "kind": "gift", "product": product }),
            Offer::Stamps => json!({ "kind": "stamps" }),
        }
    }

    /// The stored value back. Anything `check` would refuse reads as NO offer:
    /// an offer that cannot say what it gives gives nothing.
    pub fn parse(raw: &str) -> Option<Offer> {
        let v: Value = serde_json::from_str(raw).ok()?;
        let i: OfferIn = serde_json::from_value(v).ok()?;
        check(&i, true).ok().flatten()
    }
}

/// The owner's form, checked, in the owner's words. `Ok(None)` clears the
/// offer (kind `""` or `"off"`). `stamps_on`: the venue's stamp card runs --
/// a double stamp on a card that does not exist would be a promise of nothing.
pub fn check(i: &OfferIn, stamps_on: bool) -> Result<Option<Offer>, &'static str> {
    match i.kind.trim() {
        "" | "off" => Ok(None),
        "fixed" => {
            let value = i.value.unwrap_or(0);
            if value <= 0 || value > MAX_VALUE {
                return Err("bag_value: a whole amount above 0");
            }
            let min = i.min.unwrap_or(0);
            if min < 0 {
                return Err("bag_min: a minimum basket cannot be below 0");
            }
            Ok(Some(Offer::Fixed { value, min }))
        }
        "gift" => match i.product.as_deref().map(str::trim) {
            Some(p) if !p.is_empty() && p.len() <= 64 => Ok(Some(Offer::Gift { product: p.to_string() })),
            _ => Err("bag_gift: choose the dish that is on the house"),
        },
        "stamps" if stamps_on => Ok(Some(Offer::Stamps)),
        "stamps" => Err("bag_stamps: switch the stamp card on first"),
        _ => Err("bag_kind: fixed, gift or stamps"),
    }
}

/// The commission the owner typed: `Ok(None)` when empty (the card then shows
/// orders only and invents no percent).
pub fn check_pct(raw: &str) -> Result<Option<i64>, &'static str> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    match raw.parse::<i64>() {
        Ok(p) if PCT.contains(&p) => Ok(Some(p)),
        _ => Err("bag_pct: a whole percent from 1 to 60, or empty"),
    }
}

/// What the Worker hands the object: the offer, the campaign the guest came
/// from, and every spelling of their phone at this venue (the stamp card's
/// resolver; the object holds no signing secret).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WelcomeIn {
    pub offer: Offer,
    pub location_id: String,
    pub phones: Vec<String>,
    #[serde(default)]
    pub campaign: Option<String>,
}

/// Has this person had the offer? An order of theirs that carries the record
/// while it still took money. A refused order gives it back, like a card.
pub fn used(listed: &[OrderView], mine: impl Fn(&Value) -> bool) -> bool {
    listed.iter().filter_map(|v| serde_json::from_str::<Value>(&v.order_json).ok()).any(|o| {
        mine(&o)
            && o.get(RECORD).is_some()
            && took_money(o.get("status").and_then(Value::as_str).unwrap_or(""))
    })
}

/// Why the offer was not granted, as a word the storefront translates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    Used,
    BelowMinimum,
    GiftMissing,
    NothingLeft,
}

impl Refusal {
    pub fn as_str(self) -> &'static str {
        match self {
            Refusal::Used => "used",
            Refusal::BelowMinimum => "below_minimum",
            Refusal::GiftMissing => "gift_missing",
            Refusal::NothingLeft => "nothing_left",
        }
    }
}

/// What the offer takes off a basket, given what codes and cards left of it.
/// `Ok(0)` for `stamps`, which takes nothing off and counts double instead.
pub fn cut(offer: &Offer, envelope: &Value, subtotal: i64, left: i64, now_ms: i64) -> Result<i64, Refusal> {
    let raw = match offer {
        Offer::Stamps => return Ok(0),
        Offer::Fixed { value, min } => {
            let p = dowiz_hub::promo::Promo {
                code: "BAGWELCOME".into(),
                kind: dowiz_hub::promo::Kind::Fixed,
                value: *value,
                min_order: *min,
                from_ms: None,
                until_ms: None,
                max_uses: None,
                active: true,
            };
            p.redeem(subtotal, now_ms, 0).map_err(|_| Refusal::BelowMinimum)?
        }
        Offer::Gift { product } => envelope
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|l| l.get("product_id").and_then(Value::as_str) == Some(product.as_str()))
            .filter_map(|l| l.get("unit_price").and_then(Value::as_i64))
            .min()
            .ok_or(Refusal::GiftMissing)?,
    };
    match raw.min(left) {
        c if c > 0 => Ok(c),
        _ => Err(Refusal::NothingLeft),
    }
}

/// Patch the offer into the envelope `command::place::decide` is building,
/// AFTER a code and a stamp card: the record, the discount beside theirs, the
/// total. Or the refusal word, and the price untouched.
pub fn apply(envelope: &mut Value, listed: &[OrderView], w: &WelcomeIn, subtotal: i64, fee: i64, tip: i64, now_ms: i64) {
    let mine = crate::services::loyalty::stamps::by_phones(&w.location_id, &w.phones);
    if used(listed, mine) {
        envelope[REFUSED] = json!(Refusal::Used.as_str());
        return;
    }
    let before = envelope.get("discount").and_then(Value::as_i64).unwrap_or(0);
    match cut(&w.offer, envelope, subtotal, subtotal - before, now_ms) {
        Err(r) => envelope[REFUSED] = json!(r.as_str()),
        Ok(c) => {
            envelope[RECORD] = json!({ "kind": w.offer.kind(), "discount": c, "c": w.campaign });
            if c > 0 {
                envelope["discount"] = json!(before + c);
                envelope["total"] = json!(subtotal - before - c + fee + tip);
            }
        }
    }
}

/// How many stamps an order is worth on the stamp card: two when it took the
/// `stamps` welcome, one otherwise.
pub fn stamp_weight(o: &Value) -> i64 {
    match o.pointer("/welcome/kind").and_then(Value::as_str) {
        Some("stamps") => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests;
