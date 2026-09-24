//! A SALE AS THE HUB'S RECORDS: a course or counter sale as the order
//! envelope, a bill as the `Paid` payload. Pure; the rules are the parent's
//! (`whole`, `lek`, `payment`, `finished`), the shapes are here.

use super::time::epoch_ms;
use super::wire::{Sale, SaleRecord};
use super::{classify, finished, lek, payment, vat_pct, whole, Kind, MapError};
use crate::services::ordering::channel;
use serde_json::{json, Value};

/// The provenance block every mapped record carries (§3.2 (1)): ids only,
/// never the operator's code and never a client's name.
///
/// `log_cis_len` IS THE AMENDMENT WITNESS (§6.1): a sale re-fiscalised in
/// ebills grows its `logCis`, and the import compares this number (and the
/// total) with what it holds to tell an amended sale from a repeat.
pub(crate) fn external(s: &Sale) -> Value {
    let log = s.log_cis.as_deref().unwrap_or(&[]);
    json!({
        "source": "ebills", "sale_id": s.id, "inv_ord_num": s.inv_ord_num, "uuid": s.uuid,
        "fic": s.fic.clone().or_else(|| log.first().and_then(|l| l.fic.clone())),
        "iic": log.first().and_then(|l| l.iic.clone()),
        "sale_unit_kind": s.sale_unit.as_ref().map(|u| u.kind.clone()),
        "pos_id": s.point_of_sale.as_ref().map(|r| r.id),
        "sale_unit_order_id": s.sale_unit_order.as_ref().map(|o| o.id),
        "operator_id": s.extra_user.as_ref().map(|r| r.id),
        "log_cis_len": log.len(),
    })
}

/// One line, checked: whole quantity, whole lek, `price × amount ==
/// totalValue`. Answers the line and its value.
///
/// MEASURED 2026-09-23, AND IT OVERTURNED THE FIRST MAPPER: `price` is the
/// unit price AFTER the line's discount, and `discount` is a PERCENT --
/// every discounted line on the day was `price 0.0, discount 100.0,
/// totalValue 0.0` (a dish given free). Subtracting it as lek refused them
/// all. So the percent is kept as a fact on the line and never subtracted; a
/// line where `price × amount` is not the total is still refused -- a partial
/// discount whose `price` is the list price would land there, loudly.
///
/// A VOID's lines are negative (`amount -1`, measured on sale 8718); only a
/// void may carry them.
fn line(i: usize, r: &SaleRecord, void: bool) -> Result<(Value, i64), MapError> {
    let quantity = whole(r.amount, "amount")?;
    if quantity == 0 || (quantity < 0) != void {
        return Err(MapError::Negative { field: "amount" });
    }
    let unit_price = lek(r.price, "price")?;
    let pct = whole(r.discount.unwrap_or(0.0), "discount")?;
    if !(0..=100).contains(&pct) {
        return Err(MapError::NotWhole { field: "discount", value: format!("{pct}%") });
    }
    let got = whole(r.total_value, "line.totalValue")?;
    let expected = unit_price.checked_mul(quantity);
    if expected != Some(got) {
        return Err(MapError::LineMismatch { line: i, expected: expected.unwrap_or(-1), got });
    }
    let code = r.item_in_sale.as_ref().and_then(|x| x.item_code.as_deref());
    let item = json!({
        "product_id": code.map(|c| format!("ebills:{c}")), "name": r.item_name,
        "quantity": quantity, "unit_price": unit_price, "modifier_ids": [],
        "vat_rate_pct": vat_pct(r.vat.as_deref()), "discount_pct": pct,
    });
    Ok((item, got))
}

/// A course or a counter sale as the hub's order envelope, born `PICKED_UP`:
/// the one terminal that took money and had no courier leg (§3.1). The bill
/// is refused here and belongs to `to_paid`.
///
/// LAW 3 HOLDS BY CONSTRUCTION: `total = Σ unit_price × quantity`, the
/// envelope's `discount` 0 -- the till's discounts are already inside its
/// unit prices (`line`), so nothing is subtracted twice.
pub(crate) fn to_order(s: &Sale, location_id: &str) -> Result<Value, MapError> {
    let kind = classify(s)?;
    if kind == Kind::Bill {
        return Err(MapError::BillIsNotAnOrder);
    }
    finished(s)?;
    let code = s.currency.as_ref().map(|c| c.currency_code.as_str()).unwrap_or("");
    if code != "ALL" || s.currency_rate != 1.0 {
        return Err(MapError::Currency(format!("{code} @ {}", s.currency_rate)));
    }
    let pay = payment(&s.payment_method)?;
    let created_at_ms = epoch_ms(&s.timestamp)?;
    let records = s.sale_records.as_deref().filter(|r| !r.is_empty()).ok_or(MapError::NoLines)?;
    // A VOID: `changedStatus: CANCELLED`, every line negative, and the sale it
    // reverses named in `modified` (§6.6's "not observed" -- observed now).
    let void = s.changed_status.as_deref() == Some("CANCELLED");
    let void_of = match (void, &s.modified) {
        (true, Some(m)) => Some((format!("ebills:{}", m.uuid), m.id)),
        (true, None) => return Err(MapError::NotFinished { status: "CANCELLED without the sale it voids".into(), fiscal: s.fiscal_status.clone(), draft: s.draft }),
        (false, _) => None,
    };
    let mut items = Vec::with_capacity(records.len());
    let mut subtotal: i64 = 0;
    for (i, r) in records.iter().enumerate() {
        let (item, value) = line(i, r, void)?;
        subtotal = subtotal.checked_add(value).ok_or(MapError::Negative { field: "subtotal" })?;
        items.push(item);
    }
    let total = whole(s.total_value, "totalValue")?;
    if (total < 0) != void && total != 0 {
        return Err(MapError::Negative { field: "totalValue" });
    }
    if total != subtotal {
        return Err(MapError::TotalMismatch { expected: subtotal, got: total });
    }
    let fulfilment = match (&kind, &s.sale_unit) {
        (Kind::Course, Some(u)) => json!({ "kind": "dine_in", "table": u.identifier, "fee": 0 }),
        _ => json!({ "kind": "pickup", "code": "", "fee": 0 }),
    };
    // THE SOURCE AND ITS TRUST FROM ONE PLACE (`channel.rs`, G4): the till
    // priced this sale, so the order says so -- `price_trusted` is the
    // profile's `priced_by_us`, not a second hand-typed `false`.
    let trusted = channel::profile(channel::EBILLS).is_some_and(|p| p.priced_by_us);
    let mut o = json!({
        "id": format!("ebills:{}", s.uuid), "status": "PICKED_UP", "channel": channel::EBILLS,
        "customer_id": null, "items": items, "subtotal": subtotal, "discount": 0,
        "total": total, "delivery_fee": 0, "tip": 0, "created_at_ms": created_at_ms,
        "price_trusted": trusted, "location_id": location_id,
        "contact": { "name": "", "phone": "" }, "currency": "ALL",
        "fulfilment": fulfilment, "payment": pay, "external": external(s),
    });
    if let Some((order, sale_id)) = void_of {
        o["void_of"] = json!(order);
        o["external"]["void_of_sale_id"] = json!(sale_id);
    }
    // A COUNTER SALE IS PAID WHEN IT IS RUNG UP; a course is paid by the bill
    // that closes its table, which arrives later as its own `Paid`.
    if kind == Kind::CounterSale {
        o["payment_status"] = json!("paid");
    }
    Ok(o)
}

/// A bill as the `Paid` payload for the courses of its sitting (§3.2 (4)):
/// the fiscal codes, the total the drawer took, and the table -- the join
/// key, since no GET names the courses a bill covers (§1.6).
pub(crate) fn to_paid(s: &Sale) -> Result<Value, MapError> {
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

/// One table of the live floor as the hub keeps it: the number painted on
/// it, whether someone is sitting there, and the unpaid total in whole lek.
/// No waiter's name, no `activeUser` (§6.8): the wire type has no field.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct FloorRow {
    pub(crate) table: String,
    pub(crate) status: String,
    pub(crate) occupied: bool,
    pub(crate) unpaid: Option<i64>,
}

/// The floor, whole-lek or refused like every other amount at this border.
pub(crate) fn floor(tables: &[super::wire::TableState]) -> Result<Vec<FloorRow>, MapError> {
    let mut rows = Vec::with_capacity(tables.len());
    for t in tables {
        let unpaid = t.order_total.map(|v| lek(v, "orderTotal")).transpose()?;
        rows.push(FloorRow { table: t.identifier.clone(), status: t.status.clone(), occupied: t.occupied(), unpaid });
    }
    rows.sort_by(|a, b| a.table.len().cmp(&b.table.len()).then(a.table.cmp(&b.table)));
    Ok(rows)
}
