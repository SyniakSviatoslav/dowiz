//! THE TIP RECORD (BLUEPRINT-POS-THE-ROOM §7 item 8, A8's second half). PURE.
//!
//! A fold over `Paid.by` × the tip each payment carried: who took how much in
//! tips over a period, served beside the till's Z report. NO DISTRIBUTION —
//! pooling and shares are a venue's policy, not this fold's.
//!
//! WHERE THE NUMBER COMES FROM. `command::pay` stamps a tip on the payment
//! that took it (`payments[i].tip`, in the ORDER's minor units) and raises the
//! round's `tip` by the same amount in the same event. So Σ over people here
//! equals Σ `tip` over the period's rounds, and the test proves it on orders
//! written by the real `pay::decide`. A tip given at an online checkout has no
//! taker and no `Paid.by`; it is not a person's and is not in this fold.
//!
//! THE PERIOD is the payment's `at`, both ends included, as the till's
//! `within` does. PER CURRENCY: an order in euros and one in lek are never
//! summed into one number.
//!
//! A REFUNDED TIP IS NOT EARNED. A refund hands back what was taken, and
//! `pay::settles` counts the tip in what was taken, so the tip goes back with
//! the rest. The rule is the takings' own (`status::took_money`): an order
//! that is REJECTED, CANCELLED or COMPENSATED_REFUND carries no tip for
//! anyone; a REFUNDING order still counts, as its money still does, until the
//! money is handed back. So the CHECK reads: Σ over people == Σ `tip` over the
//! period's rounds that took money.

use crate::services::orders::status::took_money;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// One person's tips in one currency, minor units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersonTip {
    pub by: String,
    pub currency: String,
    pub amount: i64,
}

/// Tips by who took them, per currency, over `[from_ms, to_ms]`. Sorted by
/// currency then person. An order that names no currency is the venue's.
/// LOUD on overflow rather than wrapping a sum.
pub fn tips_by_person(orders: &[Value], venue_currency: &str, from_ms: i64, to_ms: i64) -> Result<Vec<PersonTip>, String> {
    let mut sums: BTreeMap<(String, String), i64> = BTreeMap::new();
    for o in orders {
        if !took_money(o.get("status").and_then(Value::as_str).unwrap_or("")) {
            continue;
        }
        let currency = o.get("currency").and_then(Value::as_str).unwrap_or(venue_currency);
        for p in o.get("payments").and_then(Value::as_array).into_iter().flatten() {
            let tip = p.get("tip").and_then(Value::as_i64).unwrap_or(0);
            let at = p.get("at").and_then(Value::as_i64).unwrap_or(i64::MIN);
            if tip <= 0 || at < from_ms || at > to_ms {
                continue;
            }
            let by = p.get("by").and_then(Value::as_str).unwrap_or("").to_string();
            let slot = sums.entry((currency.to_string(), by)).or_insert(0);
            *slot = slot.checked_add(tip).ok_or_else(|| format!("tips in {currency} overflow"))?;
        }
    }
    Ok(sums
        .into_iter()
        .map(|((currency, by), amount)| PersonTip { by, currency, amount })
        .collect())
}

#[cfg(test)]
mod tests;
