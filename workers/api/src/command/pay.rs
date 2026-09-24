//! TAKE A PAYMENT: one `Paid` event with an amount, a method, a currency, a
//! signer, and — for cash — the till it went into.
//!
//! A check (sitting) can receive multiple payments until its total is covered.
//! Each payment is one `Paid` event on its order; the check closes when Σ
//! payments == the bill. The split is the room calling `pay` N times with N
//! amounts, and the object refusing if the sum would exceed the bill (G5).
//!
//! CURRENCY (BLUEPRINT-OPERATIONAL-BLIND-SPOTS P3-2): the invoice stays in the
//! ORDER's currency; a payment may be in another, with its integer rate stated
//! (the unit is defined in `fx`). Σ ≤ total and "paid" are in the order's
//! currency (`amount_in_order_currency`); the drawer counts the note in the
//! currency it was handed over in (`command::till`).
//!
//! CASH NEEDS AN OPEN TILL (BLUEPRINT-POS-THE-ROOM §2.5): "a cash `Paid`
//! outside any open till is a breach" — refused here, and law 10 names any
//! that got in before this refusal existed. The object stamps the open till's
//! id on the payment; the client does not get to choose a drawer.
//!
//! THE CONFIRMED FRAME (§4.5 (2)). An order whose bill is settled carries
//! `payment_status: "paid"` and cannot be amended — taking money off it is a
//! refund (not this command).

use super::amend::next_seq;
use super::Refused;
use crate::hubdo::OrderView;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub mod fx;
pub mod legs;
pub mod tender;
pub mod wallet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayIn {
    pub order_id: String,
    pub location_id: String,
    /// Minor units OF THE CURRENCY PAID IN. At least 1.
    pub amount: i64,
    /// The payment method: a closed set (`validate_method`).
    pub method: String,
    /// The signer. Mandatory, and never waived.
    pub by: String,
    /// The till the client believes is open. Checked, never trusted: if it
    /// names another till than the open one the payment is refused.
    #[serde(default)]
    pub till_id: Option<String>,
    /// Advisory: which orders this payment covers, for the UI. Not checked.
    #[serde(default)]
    pub covers: Option<Vec<String>>,
    /// `dowiz_core::money::Currency` code handed over. Absent → the order's.
    #[serde(default)]
    pub currency: Option<String>,
    /// Required when `currency` is not the order's; refused when it is. The
    /// unit is in `fx`'s header: order minor units per payment minor unit × 1e6.
    #[serde(default)]
    pub rate_ppm: Option<i64>,
    /// A TIP taken with this payment, in the ORDER's minor units (§2.3,
    /// OPERATIONAL-BLIND-SPOTS P1-3). It raises the round's `tip` and `total`
    /// by itself in the same `Paid`, so law 3 (`total = lines + fee + tip -
    /// discount`) still holds and the sum rule is checked against the NEW
    /// total: "keep the change" is `amount = owed, tip = the change`. The tip
    /// is not the venue's money and not the drawer's: the payment's `amount`
    /// is the bill's share only, so the till's expected cash never includes it.
    #[serde(default)]
    pub tip: Option<i64>,
    /// `method: "wallet"` only: the wallet paying, as the wallet routes name
    /// it (`wallet.rs`). Its debit leg is written in the same turn
    /// (`pay::wallet`), and a balance short of the amount is a refusal.
    #[serde(default)]
    pub wallet: Option<String>,
    pub now_ms: i64,
}

/// What the object knows that the payment needs and the request cannot say.
#[derive(Debug, Clone, Copy)]
pub struct Room<'a> {
    /// The till open right now, if any (`command::till`'s fold).
    pub open_till: Option<&'a str>,
    /// The venue's currency: an order that does not name its own is in this.
    pub venue_currency: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayOut {
    /// The order as it now is.
    pub merged: String,
    pub seq: u64,
    pub generation: i64,
}

/// A round in one of these was never owed, or its money is already going back:
/// taking a payment on it would record money the venue must then return.
/// Every other status takes one — a dine-in bill is settled after the food.
fn refuses_payment(status: &str) -> bool {
    matches!(status, "CANCELLED" | "REJECTED" | "REFUNDING" | "COMPENSATED_REFUND")
}

/// The closed set of payment methods. EXACT, not trimmed: " cash" passing
/// here and failing the `== "cash"` till rule below would be cash with no
/// drawer.
pub fn validate_method(method: &str) -> bool {
    matches!(method, "cash" | "card" | "cheque" | "transfer" | "gift_card" | "wallet" | "other")
}

/// What one recorded payment took off the bill, in the order's currency. A
/// payment recorded before payments carried a currency is in the order's.
///
/// A TIP SETTLES TOO: it raised the total by itself, so it is paid by the
/// payment that carried it (`PayIn::tip`).
pub fn settles(p: &Value) -> i64 {
    let bill = p.get("amount_in_order_currency").or_else(|| p.get("amount")).and_then(Value::as_i64).unwrap_or(0);
    bill.saturating_add(p.get("tip").and_then(Value::as_i64).unwrap_or(0))
}

/// Σ of the payments already on the order plus this one, in the order's currency.
fn paid_with(order: &Value, this: i64) -> Result<i64, Refused> {
    let mut sum = this;
    for p in order.get("payments").and_then(Value::as_array).into_iter().flatten() {
        sum = sum.checked_add(settles(p)).ok_or_else(|| Refused::Invalid("payment sum overflows".into()))?;
    }
    Ok(sum)
}

/// THE WHOLE PAYMENT, over the order image already in memory. Nothing is
/// written unless every rule passed; the one write is the last statement.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    current: Option<&OrderView>,
    input: &PayIn,
    room: &Room,
) -> Result<(Value, String, u64), Refused> {
    let Some(current) = current else { return Err(Refused::NotFound) };
    if input.by.trim().is_empty() {
        return Err(Refused::Invalid("a payment names who took it".into()));
    }
    if input.amount < 1 {
        return Err(Refused::Invalid("a payment is at least 1 minor unit".into()));
    }
    if !validate_method(&input.method) {
        return Err(Refused::Invalid(format!("method {}: not recognized", input.method)));
    }
    let before: Value = serde_json::from_str(&current.order_json)
        .map_err(|e| Refused::Append(format!("order json unreadable: {e}")))?;
    if before.get("location_id").and_then(Value::as_str) != Some(input.location_id.as_str()) {
        return Err(Refused::NotFound);
    }
    let status = before.get("status").and_then(Value::as_str).unwrap_or("");
    if refuses_payment(status) {
        return Err(Refused::Conflict(format!("this round is {status}; it takes no payment")));
    }
    let till = if input.method == "cash" {
        let Some(open) = room.open_till else {
            return Err(Refused::Conflict("open the till first: cash goes into a drawer".into()));
        };
        if let Some(asked) = input.till_id.as_deref().filter(|t| *t != open) {
            return Err(Refused::Conflict(format!("till {asked} is not open; {open} is")));
        }
        Some(open)
    } else {
        None
    };

    let tip = input.tip.unwrap_or(0);
    if tip < 0 {
        return Err(Refused::Invalid("a tip is not negative".into()));
    }
    if (input.method == "wallet") != input.wallet.as_deref().is_some_and(|w| !w.trim().is_empty()) {
        return Err(Refused::Invalid("a wallet payment names its wallet, and only a wallet payment does".into()));
    }
    // A WALLET PAYS THE BILL'S SHARE ONLY. Its leg debits `amount`; a tip on
    // it would settle money no leg took from anyone (law 12 compares the leg
    // to `amount`, not to `settles`). The guest tips in cash or on the card.
    if input.method == "wallet" && tip > 0 {
        return Err(Refused::Invalid("a wallet pays the bill only: take the tip in cash or on the card".into()));
    }
    let order_currency = before.get("currency").and_then(Value::as_str).unwrap_or(room.venue_currency);
    let settled = fx::settle(order_currency, input.currency.as_deref(), input.rate_ppm, input.amount)?;
    let old_total = before.get("total").and_then(Value::as_i64).unwrap_or(0);
    let total = old_total.checked_add(tip).ok_or_else(|| Refused::Invalid("the tip overflows the total".into()))?;
    let paid = paid_with(&before, settled.in_order_currency.saturating_add(tip))?;
    if paid > total {
        return Err(Refused::Conflict(format!("payment sum {paid} exceeds the total {total}")));
    }

    // LAW 3: a payment never changes subtotal or discount; a TIP raises `tip`
    // and `total` by the same amount, the one term law 3 lets it move.
    let mut payment = json!({
        "by": input.by,
        "amount": input.amount,
        "method": input.method,
        "currency": settled.currency.code(),
        "at": input.now_ms,
    });
    if let Some(t) = till {
        payment["till_id"] = json!(t);
    }
    if let Some(rate) = settled.rate_ppm {
        payment["rate_ppm"] = json!(rate);
        payment["amount_in_order_currency"] = json!(settled.in_order_currency);
    }
    if let Some(w) = input.wallet.as_deref().filter(|_| input.method == "wallet") {
        payment["wallet"] = json!(w.trim());
    }
    let mut order = before.clone();
    if tip > 0 {
        payment["tip"] = json!(tip);
        let old_tip = before.get("tip").and_then(Value::as_i64).unwrap_or(0);
        order["tip"] = json!(old_tip.saturating_add(tip));
        order["total"] = json!(total);
    }
    let mut payments = before.get("payments").and_then(Value::as_array).cloned().unwrap_or_default();
    payments.push(payment);
    order["payments"] = Value::Array(payments);
    if paid == total {
        order["payment_status"] = json!("paid");
    }

    let body = crate::fold::delta(&before, &order).to_string();
    let seq = next_seq(current.seq, input.now_ms);
    hub.append(dowiz_hub::EventKind::Paid, &input.order_id, &body, seq, [0u8; 32])
        .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    Ok((order, body, seq))
}

#[cfg(test)]
mod tests;
