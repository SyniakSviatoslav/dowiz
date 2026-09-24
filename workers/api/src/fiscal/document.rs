//! PURE. One fiscal document for one order (TAX §3.7), built from the order
//! as the log serves it. No fetch, no clock (`now_ms` is passed in), no
//! provider name: the adapter that turns this into a platform's wire format is
//! another lane's, and it is not opened (HARD LIMIT, see `sender.rs`).
//!
//! THE TAX IS READ, NEVER RECOMPUTED. The `tax` block was stamped once at
//! placement (`services/ordering/tax_block.rs`) and law 9 audits it; a second
//! computation here would be a second pricer, the thing `a18886d4` closed.
//! A line carries the rate it was charged at (`items[].vat_ppm`) and its gross;
//! the per-group base and tax are the authority (§4 item 3: per-line rounding
//! is recommended against), so a line has no tax figure of its own.
//!
//! A FOREIGN TENDER IS A PAYMENT MEANS, NOT A SECOND INVOICE. Law 87/2019
//! chapter IV: a EUR payment on a Durrës bill is a line in `payments` with its
//! currency and `rate_ppm`, on an invoice in the venue's currency.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::services::orders::status::took_money;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Line {
    pub name: String,
    /// The catalogue product the line sold: the key of the eBills crosswalk
    /// (`ebills_body.rs`). `None` on a document queued before it was kept,
    /// and such a line is refused by name, never sent as free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    pub qty: i64,
    pub unit_as_priced: i64,
    pub gross: i64,
    pub rate_ppm: i64,
}

/// One rate group of the stamped block (`SameTaxes` by another name).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Group {
    pub rate_ppm: i64,
    pub lines: i64,
    pub base: i64,
    pub tax: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Payment {
    pub method: String,
    pub amount: i64,
    pub currency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_ppm: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Kind {
    Cash,
    NonCash,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Document {
    pub uuid: [u8; 16],
    pub order_id: String,
    pub issued_at_ms: i64,
    pub kind: Kind,
    pub currency: String,
    pub inclusive: bool,
    pub lines: Vec<Line>,
    pub groups: Vec<Group>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee: Option<Group>,
    pub total_base: i64,
    pub total_tax: i64,
    pub total: i64,
    pub payments: Vec<Payment>,
    pub table: Option<String>,
    pub channel: String,
    /// The document this one corrects (art. 32: a registered invoice is never
    /// deleted, it is corrected by one that names it).
    pub corrects: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    NoTax,
    AlreadyFiscalised { by: String },
    Untrusted,
    NotTaken,
    Currency(String),
    Lines(String),
}

fn int(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(Value::as_i64)
}
fn text<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

/// The document for one order, or why there is none. Rules in TAX §3.7's order.
pub fn document(order: &Value, venue_currency: &str, now_ms: i64) -> Result<Document, Refusal> {
    // 1. No tax block, no fiscal document.
    let Some(tax) = order.get("tax").filter(|t| t.is_object()) else { return Err(Refusal::NoTax) };
    // 2. It came FROM the fiscal platform: sending it back is a double invoice.
    if let Some(fic) = order.get("external").and_then(|e| e.get("fic")).filter(|f| !f.is_null()) {
        return Err(Refusal::AlreadyFiscalised { by: fic.as_str().unwrap_or("external.fic").to_string() });
    }
    // 2b. An order IMPORTED from the till (channel `ebills`, `external.source`
    // ebills) was fiscalised by the till, with or without a `fic` in hand.
    let from_till = text(order, "channel") == Some(crate::services::ordering::channel::EBILLS)
        || order.get("external").and_then(|e| text(e, "source")) == Some("ebills");
    if from_till {
        return Err(Refusal::AlreadyFiscalised { by: "ebills".into() });
    }
    // 3. Never charged, never invoiced.
    if order.get("price_trusted").and_then(Value::as_bool) == Some(false) {
        return Err(Refusal::Untrusted);
    }
    // 4. The kernel's own question, not a list of statuses copied here.
    match text(order, "status") {
        Some(s) if took_money(s) => {}
        _ => return Err(Refusal::NotTaken),
    }
    let currency = text(order, "currency").unwrap_or(venue_currency);
    if currency != venue_currency {
        return Err(Refusal::Currency(format!("the order is in {currency}, the venue invoices in {venue_currency}")));
    }
    let order_id = text(order, "id").or_else(|| text(order, "order_id")).unwrap_or("");
    if order_id.is_empty() {
        return Err(Refusal::Lines("the order has no id".into()));
    }

    let items = order.get("items").and_then(Value::as_array).filter(|a| !a.is_empty());
    let Some(items) = items else { return Err(Refusal::Lines("no lines".into())) };
    let mut lines = Vec::with_capacity(items.len());
    for (i, it) in items.iter().enumerate() {
        let qty = int(it, "quantity").filter(|q| *q >= 1);
        let unit = int(it, "unit_price").filter(|u| *u >= 0);
        let rate = int(it, "vat_ppm").filter(|r| *r >= 0);
        let (Some(qty), Some(unit), Some(rate)) = (qty, unit, rate) else {
            return Err(Refusal::Lines(format!("line {i} lacks quantity, unit_price or its stamped vat_ppm")));
        };
        let gross = qty.checked_mul(unit).ok_or_else(|| Refusal::Lines(format!("line {i} overflows")))?;
        let name = text(it, "name").or_else(|| text(it, "product_id")).unwrap_or("?").to_string();
        let product_id = text(it, "product_id").map(str::to_string);
        lines.push(Line { name, product_id, qty, unit_as_priced: unit, gross, rate_ppm: rate });
    }

    let group = |g: &Value| Group {
        rate_ppm: int(g, "rate_ppm").unwrap_or(0),
        lines: int(g, "lines").unwrap_or(0),
        base: int(g, "base").unwrap_or(0),
        tax: int(g, "tax").unwrap_or(0),
    };
    let groups: Vec<Group> = tax.get("groups").and_then(Value::as_array).into_iter().flatten().map(group).collect();
    let counted: i64 = groups.iter().map(|g| g.lines).sum();
    if groups.is_empty() || counted != lines.len() as i64 {
        return Err(Refusal::Lines(format!("the tax groups count {counted} lines, the order has {}", lines.len())));
    }
    let fee = tax.get("fee").filter(|f| f.is_object()).map(group);
    let total_base = groups.iter().map(|g| g.base).sum::<i64>() + fee.as_ref().map_or(0, |f| f.base);
    let total_tax = int(tax, "total").unwrap_or(0);
    let total = int(order, "total").unwrap_or(0);

    let payments = payments_of(order, currency, total);
    let kind = if payments.iter().all(|p| p.method == "cash") { Kind::Cash } else { Kind::NonCash };
    let venue = text(order, "location_id").unwrap_or("");
    Ok(Document {
        uuid: uuid_of(&format!("fiscal:{venue}:{order_id}")),
        order_id: order_id.to_string(),
        issued_at_ms: now_ms,
        kind,
        currency: currency.to_string(),
        inclusive: tax.get("inclusive").and_then(Value::as_bool).unwrap_or(false),
        lines,
        groups,
        fee,
        total_base,
        total_tax,
        total,
        payments,
        table: order.get("fulfilment").and_then(|f| text(f, "table")).map(str::to_string),
        channel: text(order, "channel").unwrap_or("storefront").to_string(),
        corrects: None,
    })
}

/// Every recorded payment, each in its own currency with its rate. An order
/// paid at the door (`payment: "cash"`, no `payments`) is one payment of the
/// total in the order's currency.
fn payments_of(order: &Value, currency: &str, total: i64) -> Vec<Payment> {
    let recorded: Vec<Payment> = order
        .get("payments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|p| Payment {
            method: text(p, "method").unwrap_or("other").to_string(),
            amount: int(p, "amount").unwrap_or(0),
            currency: text(p, "currency").unwrap_or(currency).to_string(),
            rate_ppm: int(p, "rate_ppm"),
        })
        .collect();
    if !recorded.is_empty() {
        return recorded;
    }
    let method = text(order, "payment").unwrap_or("other").to_string();
    vec![Payment { method, amount: total, currency: currency.to_string(), rate_ppm: None }]
}

/// The corrective document for `orig` (art. 32, TAX §3.7 rule 4): every amount
/// negated, the same lines and groups, and `corrects` naming the original.
/// Its uuid is derived from the original's, so re-deriving it is idempotent.
pub(super) fn corrective(orig: &Document, now_ms: i64) -> Document {
    let neg_group = |g: &Group| Group { base: -g.base, tax: -g.tax, ..g.clone() };
    Document {
        uuid: uuid_of(&format!("corrects:{}", uuid_text(&orig.uuid))),
        issued_at_ms: now_ms,
        lines: orig.lines.iter().map(|l| Line { gross: -l.gross, ..l.clone() }).collect(),
        groups: orig.groups.iter().map(neg_group).collect(),
        fee: orig.fee.as_ref().map(neg_group),
        total_base: -orig.total_base,
        total_tax: -orig.total_tax,
        total: -orig.total,
        payments: orig.payments.iter().map(|p| Payment { amount: -p.amount, ..p.clone() }).collect(),
        corrects: Some(orig.uuid),
        ..orig.clone()
    }
}

/// A deterministic RFC 9562 version-8 uuid from a key: two FNV-1a passes with
/// different offsets, then the version and variant bits. Deterministic is the
/// point — a retry recomputes the same key and the platform sees one invoice.
fn uuid_of(key: &str) -> [u8; 16] {
    let fnv = |seed: u64| {
        key.bytes().fold(seed, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3))
    };
    let mut u = [0u8; 16];
    u[..8].copy_from_slice(&fnv(0xcbf2_9ce4_8422_2325).to_be_bytes());
    u[8..].copy_from_slice(&fnv(0x6c62_272e_07bb_0142).to_be_bytes());
    u[6] = (u[6] & 0x0f) | 0x80;
    u[8] = (u[8] & 0x3f) | 0x80;
    u
}

/// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, the form the platform takes.
pub(super) fn uuid_text(u: &[u8; 16]) -> String {
    let h: String = u.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

#[cfg(test)]
mod tests;
