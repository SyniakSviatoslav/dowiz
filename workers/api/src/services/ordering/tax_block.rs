//! PURE. The `tax` block an order carries (BLUEPRINT-TAX-PRICE-CHANNEL §3.4).
//!
//! COMPUTED ONCE, WHERE THE DISCOUNT IS KNOWN. `command::place::decide` calls
//! this after it has redeemed the promo, so the taxable base is the discounted
//! amount (§2.3) and the stored tax is the tax of the basket the customer paid
//! for. The lines are not re-priced: the Worker priced them, and a second
//! pricer is what `a18886d4` closed. This is only the tail — `summarise` over
//! the given lines after the given cut.
//!
//! EVERY LINE IS STAMPED WITH ITS RATE (`items[].vat_ppm`): its own, else the
//! venue default in force. A receipt a year from now reads the rate off the
//! line, never out of today's settings (§2.6: stamping, not looking up).
//!
//! THE ENVELOPE'S `total` IS CHECKED, NOT TRUSTED. Inclusive prices already
//! contain their tax, so the total the Worker wrote must equal equation 6;
//! if it does not, the order is refused by name rather than stored with two
//! totals that disagree. Exclusive prices ADD the tax, so `total` is rewritten.

use serde_json::{json, Value};

use super::tax_cfg::VenueTax;

/// Stamp `items[].vat_ppm` and write `envelope["tax"]`. `Err` names why this
/// order cannot be taxed; the caller refuses it (`Refused::Untaxed`).
pub fn stamp(
    envelope: &mut Value,
    venue: &VenueTax,
    fee: i64,
    tip: i64,
    discount: i64,
) -> Result<(), String> {
    use dowiz_core::tax::{summarise, TaxInput, TaxLine};
    let Some(items) = envelope.get("items").and_then(Value::as_array) else {
        return Err("tax: the order has no lines to tax".into());
    };
    let mut lines = Vec::with_capacity(items.len());
    for (i, it) in items.iter().enumerate() {
        let qty = it.get("quantity").and_then(Value::as_i64).filter(|q| *q >= 1);
        let unit = it.get("unit_price").and_then(Value::as_i64).filter(|u| *u >= 0);
        let (Some(qty), Some(unit)) = (qty, unit) else {
            return Err(format!("tax: line {i} has no readable quantity and unit_price"));
        };
        let amount = qty.checked_mul(unit).ok_or_else(|| format!("tax: line {i} overflows"))?;
        let own = super::pricing::own_rate(it).map_err(|e| format!("tax: line {i}: {e}"))?;
        let rate = super::tax_cfg::rate_for(own, Some(venue)).unwrap_or(venue.default);
        lines.push(TaxLine { amount, rate });
    }
    let fee_line = (fee > 0).then_some(TaxLine { amount: fee, rate: venue.fee });
    let s = summarise(&TaxInput { lines: &lines, discount, inclusive: venue.inclusive, fee: fee_line, tip })?;

    let stated = envelope.get("total").and_then(Value::as_i64);
    if venue.inclusive && stated != Some(s.order_total) {
        return Err(format!(
            "tax: the order says total {} and its lines, cut, fee and tip make {}",
            stated.map_or("none".into(), |t| t.to_string()),
            s.order_total
        ));
    }
    let group = |g: &dowiz_core::tax::TaxGroup| {
        json!({ "rate_ppm": g.rate_ppm, "base": g.base, "tax": g.tax, "lines": g.lines })
    };
    let block = json!({
        "inclusive": s.inclusive,
        "groups": s.groups.iter().map(group).collect::<Vec<_>>(),
        "fee": s.fee.map(|f| json!({ "rate_ppm": f.rate_ppm, "base": f.base, "tax": f.tax })),
        "total": s.tax_total,
        "discount_allocated": s.discount_allocated,
    });
    // Nothing is written until everything above has succeeded.
    if let Some(items) = envelope.get_mut("items").and_then(Value::as_array_mut) {
        for (it, l) in items.iter_mut().zip(&lines) {
            it["vat_ppm"] = json!(l.rate.0);
        }
    }
    envelope["tax"] = block;
    envelope["total"] = json!(s.order_total);
    Ok(())
}

#[cfg(test)]
mod tests;
