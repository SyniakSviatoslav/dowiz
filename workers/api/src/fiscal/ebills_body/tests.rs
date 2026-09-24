//! THE GOLDEN (card L70 CHECK): the body built for a synthetic document equal
//! to the real sale 7783's content equals EBILLS-WRITE-PATH §1.6's redacted
//! example FIELD BY FIELD. The example is copied below verbatim; the till's
//! pass-through blocks (client, POS, currency, the item row, the sale unit)
//! are taken from it as the till would serve them, and every computed field
//! -- totals, VAT, price without VAT, the wrapper's flags -- is this code's.

use super::*;
use crate::fiscal::document::{Group, Kind, Line, Payment};

const EXAMPLE_7783: &str = r#"
{
  "sale": {
    "id": null,
    "totalValue": 200.0,
    "totalVatAmount": 33.3333333333,
    "currencyRate": 1,
    "currency": {"id": 1, "currency": "Albania Lek", "currencyCode": "ALL", "isActive": true,
                 "isBaseCurrency": false, "sellRate": 1.0, "buyRate": 1.0},
    "paymentMethod": "CASH",
    "salePaymentMethods": [],
    "selfIssue": "SELFISSUE_0",
    "isReverseCharge": false,
    "isExport": false,
    "invOrdNum": null, "fic": null,
    "importedInvoiceNumber": null, "contract": null, "projectReference": null, "notes": null,
    "document": null, "documentContentType": null, "filename": null,
    "invoiceTypeId": null, "invoiceType": null,
    "draft": null, "status": null, "fiscalSatus": null, "changedStatus": null, "modified": null,
    "client": {"id": 1, "name": "<str:14>", "contact": null, "email": null, "notes": null,
               "defaultClient": true, "status": "ACTIVE", "nipt": "", "identType": "ID_NUMBER",
               "address": "", "city": "", "country": "ALB", "spendingLimit": null, "barcode": null},
    "extraUser": null,
    "pointOfSale": {"id": 1, "license": "<str:23>", "posName": "<str:4>", "printing": "THERMAL",
                    "printerSize": "SIZE_80", "address": "<str:70>", "city": "<str:6>", "tcrOrderNo": 1,
                    "bussinesUnit": "<str:10>", "onlyCashInv": false, "isActive": true,
                    "printerBrand": "NORMAL", "printingType": "DIRECT", "connectorPort": 9090, "useSmallDim": true},
    "delivery": null,
    "saleRecords": [
      {"itemInSale": {"id": 23, "itemCode": "16", "item": "<str:15>", "price": 200.0, "promoPrice": null,
                      "vat": "VAT_20", "isMixProduct": false, "noTVSHType": null, "barcode": null,
                      "statusi": "ACTIVE", "isService": true, "linkedItemInWarehouseId": null,
                      "itemCategory": {"id": 2, "category": "freskuese", "status": "ACTIVE"},
                      "unit": {"id": 1839, "unit": "Copë", "unitCode": "XPP", "isActive": true},
                      "itemRemboursement": {"id": 23, "partialRemboursement": 0.0, "fullRemboursement": 0.0}},
       "itemName": "<str:15>", "saleNo": 1,
       "refundType": null, "notes": null, "serialNumber": null, "expireDate": null,
       "price": 200.0, "amount": 1.0, "discount": 0,
       "priceWithoutVat": 166.6666666667, "vat": "VAT_20",
       "totalValue": 200.0, "totalVatAmount": 33.3333333333}
    ],
    "saleEinvoiceInfos": [], "saleBanks": [], "fees": [], "vouchers": [],
    "clientExtraAddress": null
  },
  "serialNumber": null, "serialDate": null, "serialNumberEic": null, "correctiveBySerialNumber": false,
  "saleUnit": {"id": 7, "identifier": "7", "type": "TABLE", "status": "ACTIVE", "posX": 2.0, "posY": 2.0},
  "saleType": "NORMAL",
  "includeClosingTheTable": true
}
"#;

fn example() -> Value {
    serde_json::from_str(EXAMPLE_7783).expect("the §1.6 example is JSON")
}

/// The till as it would answer for 7783: the blocks the example carries.
fn till() -> Till {
    let ex = example();
    let sale = &ex["sale"];
    let row = sale["saleRecords"][0]["itemInSale"].clone();
    Till {
        items: vec![("16".into(), 23, row)],
        crosswalk: vec![("16".into(), "p-cola".into())],
        client: sale["client"].clone(),
        pos: sale["pointOfSale"].clone(),
        currency: sale["currency"].clone(),
        sale_unit: Some(ex["saleUnit"].clone()),
        fee_code: None,
    }
}

/// 7783's content as a dowiz document: one line, 1 x 200 lek at 20 %, cash.
fn doc_7783() -> Document {
    Document {
        uuid: [7; 16],
        order_id: "o7783".into(),
        issued_at_ms: 1_788_000_000_000,
        kind: Kind::Cash,
        currency: "ALL".into(),
        inclusive: true,
        lines: vec![Line { name: "Cola".into(), product_id: Some("p-cola".into()), qty: 1, unit_as_priced: 200, gross: 200, rate_ppm: 200_000 }],
        groups: vec![Group { rate_ppm: 200_000, lines: 1, base: 167, tax: 33 }],
        fee: None,
        total_base: 167,
        total_tax: 33,
        total: 200,
        payments: vec![Payment { method: "cash".into(), amount: 200, currency: "ALL".into(), rate_ppm: None }],
        table: None,
        channel: "storefront".into(),
        corrects: None,
    }
}

/// Every path where two values differ, so a failure names the field.
fn diff(path: &str, got: &Value, want: &Value, out: &mut Vec<String>) {
    match (got, want) {
        (Value::Object(g), Value::Object(w)) => {
            for k in g.keys().chain(w.keys().filter(|k| !g.contains_key(*k))) {
                diff(&format!("{path}.{k}"), g.get(k).unwrap_or(&Value::Null), w.get(k).unwrap_or(&Value::Null), out);
                if g.contains_key(k) != w.contains_key(k) {
                    out.push(format!("{path}.{k}: present in only one side"));
                }
            }
        }
        (Value::Array(g), Value::Array(w)) if g.len() == w.len() => {
            for (i, (a, b)) in g.iter().zip(w).enumerate() {
                diff(&format!("{path}[{i}]"), a, b, out);
            }
        }
        _ if got != want => out.push(format!("{path}: got {got}, want {want}")),
        _ => {}
    }
}

#[test]
fn the_body_for_7783_equals_the_redacted_example_field_by_field() {
    let got = body(&doc_7783(), &till(), None).expect("7783 builds");
    let mut out = Vec::new();
    diff("", &got, &example(), &mut out);
    assert!(out.is_empty(), "{} field(s) differ:\n{}", out.len(), out.join("\n"));
    assert_eq!(got, example(), "and as one value");
}

/// The sender's body is the golden plus the marker in `notes`, nothing else.
#[test]
fn the_sent_body_carries_the_order_marker_in_notes_and_nothing_else_differs() {
    let mut got = body(&doc_7783(), &till(), Some(&marker("o7783"))).unwrap();
    assert_eq!(got["sale"]["notes"], json!("dowiz:o7783"));
    got["sale"]["notes"] = Value::Null;
    assert_eq!(got, example());
}

/// VAT per class: the four divisors of §1.4.1, at ten decimals.
#[test]
fn each_vat_class_uses_its_divisor_at_ten_decimals() {
    let at = |ppm: i64, price: i64, qty: i64| {
        let (rec, _, _) = record(1, &json!({"item": "x"}), "x", qty, price, ppm).unwrap();
        (rec["priceWithoutVat"].to_string(), rec["totalVatAmount"].to_string(), rec["vat"].clone())
    };
    assert_eq!(at(200_000, 200, 1), ("166.6666666667".into(), "33.3333333333".into(), json!("VAT_20")));
    assert_eq!(at(100_000, 110, 3), ("100.0".into(), "30.0".into(), json!("VAT_10")));
    assert_eq!(at(60_000, 530, 2), ("500.0".into(), "60.0".into(), json!("VAT_6")));
    assert_eq!(at(0, 90, 2), ("90.0".into(), "0.0".into(), json!("VAT_0")));
    assert_eq!(at(200_000, 250, 2).1, "83.3333333333", "the measured course 8602: 2 x 250 at 20 %");
}

/// A DISH WITH NO CROSSWALK IS REFUSED BY NAME, never sent as free text.
#[test]
fn an_unmapped_dish_is_refused_by_name() {
    let mut t = till();
    t.crosswalk.clear();
    assert_eq!(body(&doc_7783(), &t, None), Err(BodyRefusal::Unmapped { line: "Cola".into() }));
    assert!(BodyRefusal::Unmapped { line: "Cola".into() }.line().contains("\"Cola\""));
    let mut d = doc_7783();
    d.lines[0].product_id = None;
    assert_eq!(body(&d, &till(), None), Err(BodyRefusal::Unmapped { line: "Cola".into() }), "a document from before product ids");
}

/// Its twin, and the other ways a mapping can be wrong.
#[test]
fn a_mapped_dish_builds_and_an_ambiguous_or_vanished_one_is_refused() {
    assert!(body(&doc_7783(), &till(), None).is_ok(), "the twin");
    let mut two = till();
    two.crosswalk.push(("17".into(), "p-cola".into()));
    assert!(matches!(body(&doc_7783(), &two, None), Err(BodyRefusal::Ambiguous { .. })));
    let mut gone = till();
    gone.items.clear();
    assert_eq!(body(&doc_7783(), &gone, None), Err(BodyRefusal::NoRow { line: "Cola".into(), code: "16".into() }));
}

#[test]
fn what_the_till_cannot_carry_is_refused_and_the_rest_builds() {
    let refuse = |f: &dyn Fn(&mut Document)| {
        let mut d = doc_7783();
        f(&mut d);
        body(&d, &till(), None).unwrap_err()
    };
    assert!(matches!(refuse(&|d| d.lines[0].rate_ppm = 88_750), BodyRefusal::Rate { ppm: 88_750, .. }));
    assert_eq!(refuse(&|d| d.inclusive = false), BodyRefusal::Exclusive);
    assert!(matches!(refuse(&|d| d.payments[0].method = "crypto".into()), BodyRefusal::Payment(_)));
    assert_eq!(refuse(&|d| d.total = 180), BodyRefusal::Total { lines: 200, total: 180 });
    assert_eq!(refuse(&|d| d.corrects = Some([1; 16])), BodyRefusal::Corrective);
    assert!(matches!(refuse(&|d| d.currency = "EUR".into()), BodyRefusal::Currency(_)));
    assert!(matches!(refuse(&|d| d.fee = Some(Group { rate_ppm: 200_000, lines: 0, base: 250, tax: 50 })), BodyRefusal::Fee(_)));
    let big = refuse(&|d| {
        d.lines[0].qty = 2501;
        d.lines[0].gross = 500_200;
        d.total = 500_200;
    });
    assert_eq!(big, BodyRefusal::CashLimit(500_200));
    // THE TWINS: card pays over the cash limit; a fee with its item is a line.
    let mut card = doc_7783();
    (card.kind, card.payments[0].method) = (Kind::NonCash, "card".into());
    assert_eq!(body(&card, &till(), None).unwrap()["sale"]["paymentMethod"], json!("CARD"));
    let mut fee = doc_7783();
    fee.fee = Some(Group { rate_ppm: 200_000, lines: 0, base: 250, tax: 50 });
    fee.total = 500;
    let mut t = till();
    t.fee_code = Some("16".into());
    let b = body(&fee, &t, None).expect("a fee with its till item builds");
    assert_eq!(b["sale"]["saleRecords"].as_array().unwrap().len(), 2);
    assert_eq!(b["sale"]["totalValue"], json!(500.0));
}

/// Without a sale unit: the POS-without-tables shape (§1.3, unknown #1).
#[test]
fn without_a_sale_unit_the_wrapper_is_a_plain_counter_sale() {
    let mut t = till();
    t.sale_unit = None;
    let b = body(&doc_7783(), &t, None).unwrap();
    assert!(b.get("saleUnit").is_none());
    assert_eq!((b["saleType"].clone(), b["includeClosingTheTable"].clone()), (json!("NORMAL"), json!(false)));
}
