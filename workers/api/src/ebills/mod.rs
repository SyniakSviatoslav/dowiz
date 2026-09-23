//! ebills.al, THE PURE HALF: the Albanian fiscal platform's sale, mapped to
//! the hub's order envelope. Bytes in, JSON out. No route, no fetch, no clock.
//! The shapes are in `wire.rs`; this file is the rules.
//!
//! WHY IT EXISTS. Dubin & Sushi rings up every dine-in course on ebills.al
//! (`docs/design/BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md`), and the owner
//! wants those orders beside the delivery orders in ONE hub with ONE stock
//! ledger. The platform has a JSON API (§1.3 there), so this is a parser and
//! a mapper, not a scraper.
//!
//! THE THREE RULES, each one a fact the blueprint measured:
//!
//! * EVERY AMOUNT ON THE WIRE IS A DOUBLE (`"totalValue": 500.0`, VAT as
//!   `83.3333333333`). Lek has no minor unit and this hub stores lek whole
//!   (`notify.rs::lek_has_no_minor_unit_and_euro_has_two`), so a value maps
//!   only if it IS whole; anything else refuses the sale and names the field.
//!   Rounding at a fiscal border is how a drawer stops reconciling.
//! * A SALE IS TWO-LEVEL. Each course at a table is a fiscalised sale of its
//!   own (`summaryInvoice:false` + `saleUnitOrder`); the bill that closes the
//!   table is a SECOND fiscalised sale whose lines repeat the courses' (§1.6:
//!   260 + 4600 = 4860, 2 + 2 = 4 lines, measured). A bill is therefore never
//!   an order here -- it is the `Paid` fact for the courses of that sitting.
//! * REFUSE, DO NOT DEFAULT. Only `CLOSED / FINISHED / draft 0 / logCis
//!   SUCCESS` was ever observed (452 of 452). A word outside that set is not
//!   "probably fine"; it is a sale the poller must show the owner unmapped.
//!
//! WHY EVERY FUNCTION IS PRIVATE. `tools/gates/unreached.py` counts `pub` and
//! `pub(crate)` fns that no shipping code calls, and nothing calls these until
//! a route does. The wiring change makes them `pub(crate)` in the same commit
//! as its route. Until then the module is compiled by its tests alone, and
//! there is no `allow(dead_code)` here: the day `mod ebills;` lands without a
//! caller, the compiler's warning is the correct report.

mod time;
mod wire;

use serde_json::{json, Value};
use crate::services::ordering::channel;
use time::epoch_ms;
use wire::*;

/// What a sale IS, decided from two flags whose four combinations were all
/// accounted for on the wire except one (§1.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// One course rung up at a table: an order.
    Course,
    /// The summary invoice that closed a table: the `Paid` fact, NOT an order.
    Bill,
    /// A sale with no table: an order, collected at the counter.
    CounterSale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MapError {
    /// A double that is not a whole number, or too large to be one exactly.
    NotWhole {
        field: &'static str,
        value: String,
    },
    Negative {
        field: &'static str,
    },
    /// Not `CLOSED`/`FINISHED`/`draft 0`: the observed set, and only it.
    NotFinished {
        status: String,
        fiscal: String,
        draft: i64,
    },
    /// No `fic`, no `logCis`, or a `logCis` row that is not `SUCCESS`.
    NotFiscalised,
    /// A `summaryInvoice` that also carries a `saleUnitOrder`: never seen,
    /// and the one combination whose meaning is unknown.
    BillWithOrder,
    /// `to_order` on a bill, or `to_paid` on anything else (§3.1).
    BillIsNotAnOrder,
    Payment(String),
    Currency(String),
    NoLines,
    /// `price × amount − discount ≠ totalValue` on one line, in whole lek.
    LineMismatch {
        line: usize,
        expected: i64,
        got: i64,
    },
    /// Σ line totals ≠ the sale's `totalValue`.
    TotalMismatch {
        expected: i64,
        got: i64,
    },
    Timestamp(String),
}

fn parse_list(body: &str) -> Result<SaleList, String> {
    serde_json::from_str(body).map_err(|e| format!("ebills sale list: {e}"))
}

fn parse_detail(body: &str) -> Result<Sale, String> {
    serde_json::from_str::<Detail>(body)
        .map(|d| d.sale)
        .map_err(|e| format!("ebills sale detail: {e}"))
}

fn parse_tables(body: &str) -> Result<Vec<TableState>, String> {
    serde_json::from_str(body).map_err(|e| format!("ebills tables: {e}"))
}

fn classify(s: &Sale) -> Result<Kind, MapError> {
    match (s.summary_invoice, s.sale_unit_order.is_some()) {
        (true, false) => Ok(Kind::Bill),
        (true, true) => Err(MapError::BillWithOrder),
        (false, true) => Ok(Kind::Course),
        (false, false) => Ok(Kind::CounterSale),
    }
}

/// Largest double that is still an exact integer: 2^53.
const EXACT_MAX: f64 = 9_007_199_254_740_992.0;

/// A wire double as a whole number, or a refusal naming the field. `-0.0`
/// and `0.0` are both zero; `NaN` and infinities are not whole.
fn whole(v: f64, field: &'static str) -> Result<i64, MapError> {
    if !v.is_finite() || v.fract() != 0.0 || v.abs() >= EXACT_MAX {
        return Err(MapError::NotWhole {
            field,
            value: format!("{v}"),
        });
    }
    Ok(v as i64)
}

/// A whole amount that may not be negative -- prices and totals.
fn lek(v: f64, field: &'static str) -> Result<i64, MapError> {
    match whole(v, field)? {
        n if n < 0 => Err(MapError::Negative { field }),
        n => Ok(n),
    }
}

/// The platform's fifteen payment words against `storefront::PAYMENT_KINDS`.
/// Three map; the rest (bank transfer, vouchers, waivers, "MULTIPLE"...) are
/// facts this hub has no word for and must not be spelled as cash.
fn payment(method: &str) -> Result<&'static str, MapError> {
    match method {
        "CASH" => Ok("cash"),
        "CARD" | "CARD_ON_POS" | "POK_CARD" => Ok("card"),
        other => Err(MapError::Payment(other.to_string())),
    }
}

/// `CLOSED`, `FINISHED`, not a draft, and every fiscalisation row `SUCCESS`
/// with a `fic` in hand -- the whole observed set, and nothing outside it.
fn finished(s: &Sale) -> Result<(), MapError> {
    if s.status != "CLOSED" || s.fiscal_status != "FINISHED" || s.draft != 0 {
        return Err(MapError::NotFinished {
            status: s.status.clone(),
            fiscal: s.fiscal_status.clone(),
            draft: s.draft,
        });
    }
    let log = s.log_cis.as_deref().unwrap_or(&[]);
    let sent = log.iter().all(|l| l.status.as_deref() == Some("SUCCESS"));
    if s.fic.is_some() && !log.is_empty() && sent {
        Ok(())
    } else {
        Err(MapError::NotFiscalised)
    }
}

/// `VAT_20` → `20`; an unknown spelling is `None`, never a guessed rate.
fn vat_pct(vat: Option<&str>) -> Option<i64> {
    vat?.strip_prefix("VAT_")?.parse().ok()
}

/// The provenance block every mapped record carries (§3.2 (1)): ids only,
/// never the operator's code and never a client's name.
fn external(s: &Sale) -> Value {
    let log = s.log_cis.as_deref().unwrap_or(&[]);
    json!({
        "source": "ebills", "sale_id": s.id, "inv_ord_num": s.inv_ord_num, "uuid": s.uuid,
        "fic": s.fic, "iic": log.first().and_then(|l| l.iic.clone()),
        "pos_id": s.point_of_sale.as_ref().map(|r| r.id),
        "sale_unit_order_id": s.sale_unit_order.as_ref().map(|o| o.id),
        "operator_id": s.extra_user.as_ref().map(|r| r.id),
    })
}

/// One line, checked: whole quantity, whole lek, and `price × amount −
/// discount == totalValue` -- the kernel's conservation habit at the border.
fn line(i: usize, r: &SaleRecord) -> Result<(Value, i64), MapError> {
    let quantity = whole(r.amount, "amount")?;
    if quantity <= 0 {
        return Err(MapError::Negative { field: "amount" });
    }
    let unit_price = lek(r.price, "price")?;
    let discount = lek(r.discount.unwrap_or(0.0), "discount")?;
    let got = lek(r.total_value, "line.totalValue")?;
    let expected = unit_price
        .checked_mul(quantity)
        .and_then(|v| v.checked_sub(discount));
    if expected != Some(got) {
        return Err(MapError::LineMismatch {
            line: i,
            expected: expected.unwrap_or(-1),
            got,
        });
    }
    let code = r.item_in_sale.as_ref().and_then(|x| x.item_code.as_deref());
    let item = json!({
        "product_id": code.map(|c| format!("ebills:{c}")), "name": r.item_name,
        "quantity": quantity, "unit_price": unit_price, "modifier_ids": [],
        "vat_rate_pct": vat_pct(r.vat.as_deref()), "discount": discount,
    });
    Ok((item, got))
}

/// A course or a counter sale as the hub's order envelope, born `PICKED_UP`:
/// the one terminal that took money and had no courier leg (§3.1). The bill
/// is refused here and belongs to `to_paid`.
fn to_order(s: &Sale, location_id: &str) -> Result<Value, MapError> {
    let kind = classify(s)?;
    if kind == Kind::Bill {
        return Err(MapError::BillIsNotAnOrder);
    }
    finished(s)?;
    let code = s
        .currency
        .as_ref()
        .map(|c| c.currency_code.as_str())
        .unwrap_or("");
    if code != "ALL" || s.currency_rate != 1.0 {
        return Err(MapError::Currency(format!("{code} @ {}", s.currency_rate)));
    }
    let pay = payment(&s.payment_method)?;
    let created_at_ms = epoch_ms(&s.timestamp)?;
    let records = s
        .sale_records
        .as_deref()
        .filter(|r| !r.is_empty())
        .ok_or(MapError::NoLines)?;
    let mut items = Vec::with_capacity(records.len());
    let mut subtotal: i64 = 0;
    for (i, r) in records.iter().enumerate() {
        let (item, value) = line(i, r)?;
        subtotal = subtotal
            .checked_add(value)
            .ok_or(MapError::Negative { field: "subtotal" })?;
        items.push(item);
    }
    let total = lek(s.total_value, "totalValue")?;
    if total != subtotal {
        return Err(MapError::TotalMismatch {
            expected: subtotal,
            got: total,
        });
    }
    let fulfilment = match (&kind, &s.sale_unit) {
        (Kind::Course, Some(u)) => json!({ "kind": "dine_in", "table": u.identifier, "fee": 0 }),
        _ => json!({ "kind": "pickup", "code": "", "fee": 0 }),
    };
    // THE SOURCE AND ITS TRUST FROM ONE PLACE (`channel.rs`, G4): the till
    // priced this sale, so the order says so -- `price_trusted` is the
    // profile's `priced_by_us`, not a second hand-typed `false`.
    let trusted = channel::profile(channel::EBILLS).is_some_and(|p| p.priced_by_us);
    Ok(json!({
        "id": format!("ebills:{}", s.uuid), "status": "PICKED_UP", "channel": channel::EBILLS,
        "customer_id": null, "items": items, "subtotal": subtotal, "total": total,
        "delivery_fee": 0, "tip": 0, "created_at_ms": created_at_ms, "price_trusted": trusted,
        "location_id": location_id, "contact": { "name": "", "phone": "" },
        "fulfilment": fulfilment, "payment": pay, "external": external(s),
    }))
}

/// A bill as the `Paid` payload for the courses of its sitting (§3.2 (4)):
/// the fiscal codes, the total the drawer took, and the table -- the join
/// key, since no GET names the courses a bill covers (§1.6).
fn to_paid(s: &Sale) -> Result<Value, MapError> {
    if classify(s)? != Kind::Bill {
        return Err(MapError::BillIsNotAnOrder);
    }
    finished(s)?;
    Ok(json!({
        "external": external(s), "total": lek(s.total_value, "totalValue")?,
        "payment": payment(&s.payment_method)?, "at_ms": epoch_ms(&s.timestamp)?,
        "table": s.sale_unit.as_ref().map(|u| u.identifier.clone()),
    }))
}

#[cfg(test)]
mod tests;
