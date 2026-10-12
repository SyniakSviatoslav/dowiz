//! D34 / R-GAPS C12: a tip, a comp and a promo cut are LINES of the eBills
//! body, and the lines sum to the order's total EXACTLY. Every figure is read
//! back from the wire JSON as an exact decimal (`scaled`), never as a float.
//! What the lines cannot reach is still refused, by name.

use super::super::{body, BodyRefusal, Till};
use crate::fiscal::document::{document, Document, Group, Kind, Line, Payment};
use crate::services::ordering::tax_block::stamp;
use crate::services::ordering::tax_cfg::VenueTax;
use dowiz_core::tax::{summarise, RatePpm, TaxInput, TaxLine};
use serde_json::{json, Value};

const V20: VenueTax = VenueTax { default: RatePpm(200_000), inclusive: true, fee: RatePpm(200_000) };

/// A wire number as an exact integer of 10^-10 units ("33.3333333333" -> 333333333333).
fn scaled(v: &Value) -> i128 {
    let t = v.to_string();
    let (neg, t) = t.strip_prefix('-').map_or((false, t.as_str()), |r| (true, r));
    let (w, f) = t.split_once('.').unwrap_or((t, ""));
    assert!(f.len() <= 10 && !t.contains('e'), "{v} is not a ten-decimal number");
    let n = w.parse::<i128>().unwrap() * 10_000_000_000 + format!("{f:0<10}").parse::<i128>().unwrap();
    if neg { -n } else { n }
}

fn lek(n: i64) -> i128 {
    i128::from(n) * 10_000_000_000
}

fn row(code: &str, id: i64, vat: &str) -> (String, i64, Value) {
    (code.into(), id, json!({"id": id, "itemCode": code, "item": format!("item {code}"), "vat": vat, "isService": true}))
}

fn till() -> Till {
    Till {
        items: vec![row("16", 23, "VAT_20"), row("17", 24, "VAT_6"), row("F", 30, "VAT_20"), row("T", 31, "VAT_0")],
        crosswalk: vec![("16".into(), "cola".into()), ("17".into(), "bread".into())],
        client: json!({"id": 1}),
        pos: json!({"id": 1}),
        currency: json!({"currencyCode": "ALL"}),
        sale_unit: None,
        fee_code: Some("F".into()),
        tip_code: Some("T".into()),
    }
}

fn line(name: &str, pid: &str, qty: i64, unit: i64, ppm: i64) -> Line {
    Line { name: name.into(), product_id: Some(pid.into()), qty, unit_as_priced: unit, gross: qty * unit, rate_ppm: ppm, comped: false }
}

/// A cash document; `total` = Σ gross - discount + tip (no fee).
fn doc(lines: Vec<Line>, discount: i64, tip: i64) -> Document {
    let total = lines.iter().map(|l| l.gross).sum::<i64>() - discount + tip;
    Document {
        uuid: [1; 16], order_id: "o1".into(), issued_at_ms: 1, kind: Kind::Cash, currency: "ALL".into(), inclusive: true,
        lines, groups: Vec::<Group>::new(), fee: None, total_base: 0, total_tax: 0, total, discount, tip,
        payments: vec![Payment { method: "cash".into(), amount: total, currency: "ALL".into(), rate_ppm: None }],
        table: None, channel: "room".into(), corrects: None,
    }
}

/// The records, and the sum of their `totalValue`s, which must be the sale's.
fn records(b: &Value) -> (Vec<Value>, i128) {
    let recs = b["sale"]["saleRecords"].as_array().expect("records").clone();
    let sum: i128 = recs.iter().map(|r| scaled(&r["totalValue"])).sum();
    assert_eq!(sum, scaled(&b["sale"]["totalValue"]), "the sale's totalValue is its records'");
    let vat: i128 = recs.iter().map(|r| scaled(&r["totalVatAmount"])).sum();
    assert_eq!(vat, scaled(&b["sale"]["totalVatAmount"]), "the sale's VAT is its records'");
    for (i, r) in recs.iter().enumerate() {
        assert_eq!(r["saleNo"], json!(i + 1), "saleNo runs 1..n");
        assert_eq!(scaled(&r["totalValue"]), scaled(&r["price"]) * scaled(&r["amount"]) / 10_000_000_000, "price x amount, record {i}");
    }
    (recs, sum)
}

#[test]
fn a_tipped_order_carries_the_tip_as_a_vat_0_line_and_sums_to_its_total() {
    let d = doc(vec![line("Cola", "cola", 1, 200, 200_000)], 0, 50);
    let b = body(&d, &till(), None).expect("a tip with its till item builds");
    let (recs, sum) = records(&b);
    assert_eq!(sum, lek(250), "200 + the 50 tip, exactly");
    let tip = recs.last().unwrap();
    assert_eq!((tip["itemInSale"]["itemCode"].clone(), tip["vat"].clone()), (json!("T"), json!("VAT_0")));
    assert_eq!((scaled(&tip["totalValue"]), scaled(&tip["totalVatAmount"])), (lek(50), 0), "a gratuity is not taxed");
}

#[test]
fn a_comped_line_is_the_tills_own_free_dish_and_the_lines_sum_to_the_total() {
    let mut free = line("Cola", "cola", 2, 200, 200_000);
    free.comped = true;
    let d = doc(vec![line("Bread", "bread", 1, 106, 60_000), free], 400, 0);
    assert_eq!(d.total, 106);
    let b = body(&d, &till(), None).expect("a comp builds");
    let (recs, sum) = records(&b);
    assert_eq!(sum, lek(106));
    let c = &recs[1];
    // BLUEPRINT-EBILLS §8.2 (3), measured on the till: `price 0.0, discount 100.0, totalValue 0.0`.
    assert_eq!((scaled(&c["price"]), scaled(&c["discount"]), scaled(&c["totalValue"])), (0, lek(100), 0));
    assert_eq!(scaled(&c["amount"]), lek(2), "the dish was served: its quantity stays");
    assert_eq!(recs[0]["discount"], json!(0), "the paid line is untouched");
}

#[test]
fn a_promo_cut_lowers_the_prices_per_rate_exactly_as_the_tax_law_allocates_it() {
    // 2 x 100 at 20 % and 1 x 100 at 6 %, a 7-lek cut that divides nothing.
    let d = doc(vec![line("Cola", "cola", 2, 100, 200_000), line("Bread", "bread", 1, 100, 60_000)], 7, 0);
    let b = body(&d, &till(), None).expect("a promo cut builds");
    let (recs, sum) = records(&b);
    assert_eq!(sum, lek(293));
    // The law's allocation, called, not re-derived: Σ records per rate == the group's discounted base.
    let law = summarise(&TaxInput {
        lines: &[TaxLine { amount: 200, rate: RatePpm(200_000) }, TaxLine { amount: 100, rate: RatePpm(60_000) }],
        discount: 7, inclusive: true, fee: None, tip: 0,
    })
    .unwrap();
    for g in &law.groups {
        let word = if g.rate_ppm == 200_000 { "VAT_20" } else { "VAT_6" };
        let got: i128 = recs.iter().filter(|r| r["vat"] == json!(word)).map(|r| scaled(&r["totalValue"])).sum();
        assert_eq!(got, lek(g.base), "{word}");
    }
    for r in &recs {
        assert_eq!(scaled(&r["price"]) % 10_000_000_000, 0, "every price is whole lek: {r}");
        assert!(scaled(&r["discount"]) >= 0);
    }
}

#[test]
fn a_cut_that_does_not_divide_the_quantity_splits_the_line_into_two_whole_lek_prices() {
    let d = doc(vec![line("Cola", "cola", 3, 250, 200_000)], 1, 0);
    let b = body(&d, &till(), None).unwrap();
    let (recs, sum) = records(&b);
    assert_eq!(sum, lek(749));
    let got: Vec<(i128, i128)> = recs.iter().map(|r| (scaled(&r["amount"]), scaled(&r["price"]))).collect();
    assert_eq!(got, vec![(lek(2), lek(250)), (lek(1), lek(249))]);
    assert_eq!(recs[0]["discount"], json!(0));
    assert_eq!(recs[1]["discount"].to_string(), "0.4", "1 lek of 250 is 0.4 %");
}

/// THE PLACEMENT PATH: an order stamped by the real `tax_block::stamp` with a
/// 75-lek promo, a 300-lek fee and a 50-lek tip, through the real `document`.
#[test]
fn a_stamped_order_with_cut_fee_and_tip_becomes_a_body_that_sums_to_its_total() {
    let mut o = json!({
        "id": "o9", "location_id": "v1", "status": "DELIVERED", "currency": "ALL", "channel": "storefront",
        "items": [{ "product_id": "cola", "name": "Cola", "quantity": 3, "unit_price": 250 }],
        "discount": 75, "delivery_fee": 300, "tip": 50, "total": 1025,
        "payments": [{ "method": "cash", "amount": 1025, "currency": "ALL" }],
    });
    stamp(&mut o, &V20, 300, 50, 75).expect("the real stamp");
    let d = document(&o, "ALL", 1).expect("document");
    assert_eq!((d.discount, d.tip, d.total), (75, 50, 1025));
    let b = body(&d, &till(), None).expect("cut, fee and tip build");
    let (recs, sum) = records(&b);
    assert_eq!(sum, lek(1025), "675 + 300 + 50");
    let fee = recs.iter().find(|r| r["itemInSale"]["itemCode"] == json!("F")).expect("the fee line");
    assert_eq!(scaled(&fee["totalValue"]), lek(300), "an inclusive fee block's base IS its gross");
}

#[test]
fn what_the_lines_cannot_reach_is_refused_by_name_and_the_twin_builds() {
    let base = || doc(vec![line("Cola", "cola", 1, 200, 200_000)], 0, 50);
    assert!(body(&base(), &till(), None).is_ok(), "the twin");
    // A total that is not its parts.
    let mut d = base();
    d.total = 260;
    assert_eq!(body(&d, &till(), None), Err(BodyRefusal::Total { lines: 250, total: 260 }));
    // A tip with no till item, or one the till taxes.
    let mut t = till();
    t.tip_code = None;
    assert!(matches!(body(&base(), &t, None), Err(BodyRefusal::Tip(_))));
    let mut t = till();
    t.items[3] = row("T", 31, "VAT_20");
    let why = body(&base(), &t, None).unwrap_err();
    assert!(matches!(&why, BodyRefusal::Tip(w) if w.contains("VAT_20")), "{why:?}");
    // Comps worth more than the whole cut; a cut larger than the paid lines.
    let mut free = line("Bread", "bread", 1, 100, 60_000);
    free.comped = true;
    let mut d = doc(vec![line("Cola", "cola", 1, 200, 200_000), free], 50, 0);
    d.total = 250;
    assert!(matches!(body(&d, &till(), None), Err(BodyRefusal::Discount(_))));
    let mut d = doc(vec![line("Cola", "cola", 1, 200, 200_000)], 0, 0);
    (d.discount, d.total) = (201, -1);
    assert!(matches!(body(&d, &till(), None), Err(BodyRefusal::Corrective | BodyRefusal::Discount(_))));
    let mut d = doc(vec![line("Cola", "cola", 1, 200, 200_000)], 0, 0);
    (d.discount, d.total, d.tip) = (201, 0, 1);
    assert!(matches!(body(&d, &till(), None), Err(BodyRefusal::Discount(_))));
    assert!(BodyRefusal::Discount("x".into()).line().contains('x'));
}
