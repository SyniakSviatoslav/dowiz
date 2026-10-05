//! THE CARD HALF OF A REFUND, as Stripe is spoken to (W-REFUND). PURE: the
//! units, the form, the idempotency key's entry, and how an answer is read.
//! The `fetch` and the order writes are in `refund_io.rs`; the webhook in
//! `refund_hook.rs`; the decision what may go back in `command/refund/card.rs`.
//!
//! ONE UNIT MISMATCH, AND IT IS A FACTOR OF A HUNDRED. dowiz counts lek in
//! whole lek (`Currency::All.minor_units() == 0`, `lib/money.js` DECIMALS).
//! Stripe treats ALL as a TWO-decimal currency: it is not on its zero-decimal
//! list (docs.stripe.com/currencies, read 2026-10-04), so `amount=1500` is
//! 15.00 lek to Stripe. `create_intent` sent the order's lek unconverted —
//! a 1500-lek order would have been charged 15 lek, under Stripe's minimum —
//! and the webhook wrote Stripe's figure back as lek. Every amount that
//! crosses to or from Stripe now goes through `to_stripe` / `from_stripe`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The outbox kind the drain sends to `POST /v1/refunds`.
pub const KIND: &str = "stripe_refund";
/// The API version the refund request pins (`Stripe-Version`). Webhook bodies
/// follow the version set on the endpoint in the Stripe dashboard instead; the
/// fields read from them (`id`, `status`, `amount`, `currency`, `metadata`,
/// `payment_intent`, `failure_reason`) are the same in every version since.
pub const STRIPE_VERSION: &str = "2024-06-20";
pub const API_REFUNDS: &str = "https://api.stripe.com/v1/refunds";

/// Stripe's zero-decimal currencies (docs.stripe.com/currencies). Every other
/// currency is two-decimal to Stripe, ALL included.
const ZERO_DECIMAL: [&str; 16] =
    ["BIF", "CLP", "DJF", "GNF", "JPY", "KMF", "KRW", "MGA", "PYG", "RWF", "UGX", "VND", "VUV", "XAF", "XOF", "XPF"];

fn decimals(code: &str) -> Option<(u32, u32)> {
    let code = code.trim().to_ascii_uppercase();
    let ours = dowiz_core::money::Currency::from_code(&code)?.minor_units();
    let theirs = if ZERO_DECIMAL.contains(&code.as_str()) { 0 } else { 2 };
    Some((ours, theirs))
}

/// dowiz's integer amount of `code` → Stripe's `amount`. `None`: a currency
/// dowiz does not price in, or an overflow — never a guess.
pub fn to_stripe(amount: i64, code: &str) -> Option<i64> {
    let (ours, theirs) = decimals(code)?;
    if theirs >= ours {
        amount.checked_mul(10_i64.checked_pow(theirs - ours)?)
    } else {
        let f = 10_i64.checked_pow(ours - theirs)?;
        (amount % f == 0).then(|| amount / f)
    }
}

/// Stripe's `amount` of `code` → dowiz's integer amount. A fraction of a lek
/// cannot be represented and is refused, not rounded.
pub fn from_stripe(amount: i64, code: &str) -> Option<i64> {
    let (ours, theirs) = decimals(code)?;
    if theirs >= ours {
        let f = 10_i64.checked_pow(theirs - ours)?;
        (amount % f == 0).then(|| amount / f)
    } else {
        amount.checked_mul(10_i64.checked_pow(ours - theirs)?)
    }
}

/// ONE CARD REFUND OWED TO STRIPE, as it waits in the venue's outbox. Every
/// field is fixed when the refund is decided; the drain re-reads nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    pub venue: String,
    pub order_id: String,
    /// The PaymentIntent the webhook recorded on the order (`payment_intent`).
    pub pi: String,
    /// In the ORDER's units (whole lek); converted only on the way out.
    pub amount: i64,
    pub currency: String,
    /// Which card refund of this order (1, 2, ...): a partial refund and its rest are two.
    pub n: u32,
    /// The Idempotency-Key: `card::idem_key(venue, order, n)`. The outbox
    /// entry's id too, so a retried turn queues ONE entry and every retry of
    /// the send carries the SAME key — Stripe answers a repeat with the refund
    /// it already made instead of making a second.
    pub key: String,
}

/// The outbox entry for `job`: id = the key, `to` = the order.
pub fn entry(job: &Job, now_ms: i64) -> crate::outbox::Entry {
    crate::outbox::Entry::new(job.key.clone(), KIND, job.order_id.clone(), serde_json::to_string(job).unwrap_or_default(), now_ms)
}

/// The form body of `POST /v1/refunds`. `Err`: a currency Stripe cannot be
/// sent an amount in.
pub fn form(job: &Job) -> Result<String, String> {
    let amount = to_stripe(job.amount, &job.currency)
        .ok_or_else(|| format!("{} {} cannot be sent to Stripe", job.amount, job.currency))?;
    if amount <= 0 {
        return Err("a refund is a positive amount".into());
    }
    let e = super::urlencode;
    Ok(format!(
        "payment_intent={}&amount={amount}&metadata[order_id]={}&metadata[venue]={}&metadata[key]={}&metadata[attempt]={}",
        e(&job.pi), e(&job.order_id), e(&job.venue), e(&job.key), job.n
    ))
}

/// What one `POST /v1/refunds` came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    /// Stripe made (or, for a repeated key, already had) this refund.
    Made { id: String, status: String, failure: Option<String> },
    /// Stripe refused it and will refuse it again: no retry, said on the order.
    Final(String),
    /// Unreachable, rate-limited, a 5xx, or the same key still in flight: retry.
    Transient(String),
}

/// Read Stripe's answer. 409 is an idempotent request still being processed
/// (or a key reused with other parameters, which a retry with the same entry
/// never is), so it is retried rather than called final.
pub fn classify(status: u16, body: &str) -> Sent {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    if (200..300).contains(&status) {
        let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        return match (s("id"), s("status")) {
            (Some(id), Some(status)) => Sent::Made { id, status, failure: s("failure_reason") },
            _ => Sent::Transient(format!("Stripe answered {status} without a refund id")),
        };
    }
    let said = v
        .pointer("/error/message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| body.chars().take(300).collect());
    if status == 409 || status == 429 || status >= 500 {
        Sent::Transient(format!("{status}: {said}"))
    } else {
        Sent::Final(format!("{status}: {said}"))
    }
}

#[cfg(test)]
#[path = "refund/tests.rs"]
mod tests;
