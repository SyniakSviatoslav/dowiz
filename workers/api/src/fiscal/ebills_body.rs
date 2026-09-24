//! PURE. One fiscal `Document` as the body of `POST https://www.ebills.al/api/sales`,
//! exactly as the till's own SPA builds it (docs/design/EBILLS-WRITE-PATH-2026-09-24.md
//! §1.3 the wrapper, §1.4 the sale, §1.4.1 a line, §1.6 the example built from
//! the real sale 7783 -- the golden in `ebills_body/tests.rs`).
//!
//! MONEY IS WHOLE LEK UNTIL THE LAST STEP. The wire wants doubles with the
//! SPA's `toFixed(10)` VAT fractions (`33.3333333333` for 200 lek at 20 %).
//! Every figure here is an exact rational of integers, rounded half-up at ten
//! decimals in `i128` and only then written as a JSON number (`num`); nothing
//! is stored in that form, and no float is ever computed with.
//!
//! REFUSE BY NAME, NEVER GUESS. A dish with no crosswalk to a till item, a rate
//! that is not one of the till's four, a payment the till has no single word
//! for, an order total that is not its lines: each is a `BodyRefusal` naming
//! the line, and nothing is sent. A dish is never sent as free text.

use super::document::{Document, Kind};
use crate::ebills::OURS;
use serde_json::{json, Map, Value};

/// Ten decimals, the SPA's `toFixed(10)`.
const SCALE: i128 = 10_000_000_000;
/// The legal cash limit to an individual (EBILLS-WRITE-PATH §1.5 step 5).
pub const CASH_LIMIT_LEK: i64 = 500_000;

/// What the till contributes, read by allow-listed GETs in the same firing
/// and passed through WITHOUT BEING KEPT (§6: the client block holds a name).
#[derive(Debug, Clone, PartialEq)]
pub struct Till {
    /// `/api/item-in-sales` rows: (item code, row id, the whole row as served).
    pub items: Vec<(String, i64, Value)>,
    /// The owner's crosswalk: (till item code, catalogue product id).
    pub crosswalk: Vec<(String, String)>,
    /// From a till sale of this POS: the default client, the POS, the currency.
    pub client: Value,
    pub pos: Value,
    pub currency: Value,
    /// The sale unit the owner chose (`fiscal.ebills.sale_unit`), as sent.
    pub sale_unit: Option<Value>,
    /// The till item the delivery fee is sold as (`fiscal.ebills.fee_item`).
    pub fee_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyRefusal {
    /// A dish with no till item mapped to it: the owner maps it first.
    Unmapped { line: String },
    /// Two till items mapped to one dish: which one sold is unknown.
    Ambiguous { line: String, codes: Vec<String> },
    /// Mapped to a code the till's menu no longer lists.
    NoRow { line: String, code: String },
    /// A rate that is not VAT 0/6/10/20 %.
    Rate { line: String, ppm: i64 },
    Exclusive,
    Payment(String),
    Fee(String),
    Total { lines: i64, total: i64 },
    CashLimit(i64),
    Currency(String),
    /// A corrective (negative) document is a cancel, never a create.
    Corrective,
}

impl BodyRefusal {
    /// One line for the owner, naming the dish where there is one.
    pub fn line(&self) -> String {
        match self {
            BodyRefusal::Unmapped { line } => format!("\"{line}\" has no eBills item: map it in the till link"),
            BodyRefusal::Ambiguous { line, codes } => format!("\"{line}\" is mapped to several eBills items ({})", codes.join(", ")),
            BodyRefusal::NoRow { line, code } => format!("\"{line}\" is mapped to eBills item {code}, which the till no longer lists"),
            BodyRefusal::Rate { line, ppm } => format!("\"{line}\" is taxed at {ppm} ppm; the till knows 0, 6, 10 and 20 %"),
            BodyRefusal::Exclusive => "prices are exclusive of VAT; the till's price is VAT-inclusive".into(),
            BodyRefusal::Payment(m) => format!("payment {m} has no single eBills word (CASH or CARD)"),
            BodyRefusal::Fee(why) => why.clone(),
            BodyRefusal::Total { lines, total } => format!("the lines come to {lines} and the order to {total}: a discount or tip the till cannot carry"),
            BodyRefusal::CashLimit(t) => format!("{t} lek in cash is over the legal limit of {CASH_LIMIT_LEK}"),
            BodyRefusal::Currency(c) => format!("the document is in {c}; the till invoices in ALL"),
            BodyRefusal::Corrective => "a corrective document is a cancel, not a create".into(),
        }
    }
}

/// The marker a created sale carries in `notes` (§3 (b)): how an unanswered
/// send is found again, and how the poller knows not to import it.
pub fn marker(order_id: &str) -> String {
    format!("{OURS}{order_id}")
}

/// `scaled / SCALE` as a JSON number, e.g. 333333333333 -> 33.3333333333.
/// The one place a wire double is made: text of an exact decimal, parsed.
fn num(scaled: i128) -> Value {
    let sign = if scaled < 0 { "-" } else { "" };
    let a = scaled.unsigned_abs();
    let text = format!("{sign}{}.{:010}", a / SCALE as u128, a % SCALE as u128);
    serde_json::from_str(&text).unwrap_or(Value::Null)
}

/// `n / d`, scaled and rounded half-up (n, d > 0 here: prices, rates).
fn ratio(n: i128, d: i128) -> i128 {
    (2 * n * SCALE + d) / (2 * d)
}

/// ppm -> the till's VAT word and percent.
fn vat_of(ppm: i64) -> Option<(&'static str, i128)> {
    match ppm {
        0 => Some(("VAT_0", 0)),
        60_000 => Some(("VAT_6", 6)),
        100_000 => Some(("VAT_10", 10)),
        200_000 => Some(("VAT_20", 20)),
        _ => None,
    }
}

/// One `saleRecords[]` entry (§1.4.1) and its (total, VAT) in scaled units.
fn record(no: usize, row: &Value, name: &str, qty: i64, price: i64, ppm: i64) -> Result<(Value, i128, i128), BodyRefusal> {
    let (word, pct) = vat_of(ppm).ok_or_else(|| BodyRefusal::Rate { line: name.to_string(), ppm })?;
    let (q, p) = (i128::from(qty), i128::from(price));
    let total = q * p * SCALE;
    let vat = if pct == 0 { 0 } else { ratio(q * p * pct, 100 + pct) };
    let without = ratio(p * 100, 100 + pct);
    let item_name = row.get("item").and_then(Value::as_str).unwrap_or(name);
    let rec = json!({
        "itemInSale": row, "itemName": item_name, "saleNo": no,
        "refundType": null, "notes": null, "serialNumber": null, "expireDate": null,
        "price": num(p * SCALE), "amount": num(q * SCALE), "discount": 0,
        "priceWithoutVat": num(without), "vat": word,
        "totalValue": num(total), "totalVatAmount": num(vat),
    });
    Ok((rec, total, vat))
}

/// The till row a dish is sold as, through the owner's crosswalk.
fn row_for<'a>(till: &'a Till, name: &str, product: Option<&str>) -> Result<&'a Value, BodyRefusal> {
    let unmapped = || BodyRefusal::Unmapped { line: name.to_string() };
    let pid = product.ok_or_else(unmapped)?;
    let codes: Vec<String> = till.crosswalk.iter().filter(|(_, p)| p == pid).map(|(c, _)| c.clone()).collect();
    let code = match codes.as_slice() {
        [] => return Err(unmapped()),
        [one] => one.clone(),
        _ => return Err(BodyRefusal::Ambiguous { line: name.to_string(), codes }),
    };
    till.items
        .iter()
        .find(|(c, _, _)| *c == code)
        .map(|(_, _, row)| row)
        .ok_or(BodyRefusal::NoRow { line: name.to_string(), code })
}

/// CASH or CARD: the one word every payment of the document shares.
fn method(doc: &Document) -> Result<&'static str, BodyRefusal> {
    let word = |m: &str| match m {
        "cash" => Some("CASH"),
        "card" | "apple_pay" | "google_pay" => Some("CARD"),
        _ => None,
    };
    let mut words = doc.payments.iter().map(|p| word(&p.method).ok_or_else(|| BodyRefusal::Payment(p.method.clone())));
    let first = words.next().unwrap_or(Err(BodyRefusal::Payment("none".into())))?;
    for w in words {
        if w? != first {
            return Err(BodyRefusal::Payment("a mix of cash and card".into()));
        }
    }
    if doc.kind == Kind::Cash && first != "CASH" {
        return Err(BodyRefusal::Payment("cash by its kind, card by its payments".into()));
    }
    Ok(first)
}

/// THE CREATE WRAPPER (§1.3) for one document. `notes` is the marker; `None`
/// only for the golden, which compares against 7783's own `notes: null`.
pub fn body(doc: &Document, till: &Till, notes: Option<&str>) -> Result<Value, BodyRefusal> {
    if doc.corrects.is_some() || doc.total < 0 {
        return Err(BodyRefusal::Corrective);
    }
    if doc.currency != "ALL" {
        return Err(BodyRefusal::Currency(doc.currency.clone()));
    }
    if !doc.inclusive {
        return Err(BodyRefusal::Exclusive);
    }
    let pay = method(doc)?;
    let mut records = Vec::with_capacity(doc.lines.len() + 1);
    let (mut total, mut vat, mut gross) = (0i128, 0i128, 0i64);
    for l in &doc.lines {
        let row = row_for(till, &l.name, l.product_id.as_deref())?;
        let (rec, t, v) = record(records.len() + 1, row, &l.name, l.qty, l.unit_as_priced, l.rate_ppm)?;
        records.push(rec);
        (total, vat, gross) = (total + t, vat + v, gross + l.gross);
    }
    if let Some(fee) = doc.fee.as_ref().filter(|f| f.base + f.tax != 0) {
        let code = till.fee_code.as_deref().ok_or_else(|| BodyRefusal::Fee("the delivery fee has no eBills item: choose one in the fiscal pane".into()))?;
        let row = till.items.iter().find(|(c, _, _)| c == code).map(|(_, _, r)| r);
        let row = row.ok_or_else(|| BodyRefusal::Fee(format!("the delivery fee's eBills item {code} is not on the till's menu")))?;
        let (rec, t, v) = record(records.len() + 1, row, "delivery fee", 1, fee.base + fee.tax, fee.rate_ppm)?;
        records.push(rec);
        (total, vat, gross) = (total + t, vat + v, gross + fee.base + fee.tax);
    }
    if gross != doc.total {
        return Err(BodyRefusal::Total { lines: gross, total: doc.total });
    }
    if pay == "CASH" && doc.total > CASH_LIMIT_LEK {
        return Err(BodyRefusal::CashLimit(doc.total));
    }
    let sale = json!({
        "id": null, "totalValue": num(total), "totalVatAmount": num(vat), "currencyRate": 1,
        "currency": till.currency, "paymentMethod": pay, "salePaymentMethods": [],
        "selfIssue": "SELFISSUE_0", "isReverseCharge": false, "isExport": false,
        "invOrdNum": null, "fic": null, "importedInvoiceNumber": null, "contract": null,
        "projectReference": null, "notes": notes, "document": null, "documentContentType": null,
        "filename": null, "invoiceTypeId": null, "invoiceType": null, "draft": null, "status": null,
        "fiscalSatus": null, "changedStatus": null, "modified": null, "client": till.client,
        "extraUser": null, "pointOfSale": till.pos, "delivery": null, "saleRecords": records,
        "saleEinvoiceInfos": [], "saleBanks": [], "fees": [], "vouchers": [], "clientExtraAddress": null,
    });
    let mut w = Map::new();
    w.insert("sale".into(), sale);
    for k in ["serialNumber", "serialDate", "serialNumberEic"] {
        w.insert(k.into(), Value::Null);
    }
    w.insert("correctiveBySerialNumber".into(), json!(false));
    // §1.3: with a sale unit, 7783's shape (a counter sale rung at a table and
    // closed at once); without, a POS-without-tables sale -- unknown #1.
    if let Some(unit) = &till.sale_unit {
        w.insert("saleUnit".into(), unit.clone());
    }
    w.insert("saleType".into(), json!("NORMAL"));
    w.insert("includeClosingTheTable".into(), json!(till.sale_unit.is_some()));
    Ok(Value::Object(w))
}

#[cfg(test)]
mod tests;
