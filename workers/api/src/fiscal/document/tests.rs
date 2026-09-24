//! TAX §3.7 rules 1-4, each refusal beside its positive twin. The tax block is
//! stamped by the REAL placement code (`tax_block::stamp`), so these read the
//! block an order actually carries, not a hand-written imitation of it.

use super::*;
use crate::services::ordering::tax_block::stamp;
use crate::services::ordering::tax_cfg::VenueTax;
use dowiz_core::tax::RatePpm;
use serde_json::json;

const NOW: i64 = 1_790_000_000_000;
const V20: VenueTax = VenueTax { default: RatePpm(200_000), inclusive: true, fee: RatePpm(200_000) };

/// Three coffees at 250 lek + 300 delivery, 20 % inclusive, paid in cash.
fn taxed() -> Value {
    let mut o = json!({
        "id": "o1", "location_id": "v1", "status": "DELIVERED", "currency": "ALL",
        "items": [{ "product_id": "coffee", "name": "Kafe", "quantity": 3, "unit_price": 250 }],
        "total": 1050, "channel": "storefront",
        "payments": [{ "method": "cash", "amount": 1050, "currency": "ALL" }],
    });
    stamp(&mut o, &V20, 300, 0, 0).expect("the real stamp taxes it");
    o
}

#[test]
fn a_taxed_order_becomes_a_document_read_from_its_stamped_block() {
    let d = document(&taxed(), "ALL", NOW).expect("document");
    assert_eq!(d.order_id, "o1");
    assert_eq!(d.issued_at_ms, NOW);
    assert_eq!(d.groups, vec![Group { rate_ppm: 200_000, lines: 1, base: 750, tax: 125 }]);
    assert_eq!(d.fee, Some(Group { rate_ppm: 200_000, lines: 0, base: 300, tax: 50 }));
    assert_eq!((d.total_base, d.total_tax, d.total), (1050, 175, 1050));
    assert_eq!(d.lines[0], Line { name: "Kafe".into(), qty: 3, unit_as_priced: 250, gross: 750, rate_ppm: 200_000 });
    assert_eq!(d.kind, Kind::Cash);
    assert_eq!(d.payments, vec![Payment { method: "cash".into(), amount: 1050, currency: "ALL".into(), rate_ppm: None }]);
    assert!(d.inclusive);
    assert_eq!(d.corrects, None);
}

#[test]
fn rule_1_no_tax_block_is_no_tax() {
    let mut o = taxed();
    o.as_object_mut().unwrap().remove("tax");
    assert_eq!(document(&o, "ALL", NOW), Err(Refusal::NoTax));
}

#[test]
fn rule_2_an_order_that_came_from_the_platform_is_not_sent_back() {
    let mut o = taxed();
    o["external"] = json!({ "fic": "FIC-FROM-EBILLS" });
    assert_eq!(document(&o, "ALL", NOW), Err(Refusal::AlreadyFiscalised { by: "FIC-FROM-EBILLS".into() }));
    o["external"] = json!({ "source": "wolt" });
    assert!(document(&o, "ALL", NOW).is_ok(), "an external block without a fic is not fiscalised");
}

#[test]
fn rule_3_an_untrusted_price_is_never_invoiced() {
    let mut o = taxed();
    o["price_trusted"] = json!(false);
    assert_eq!(document(&o, "ALL", NOW), Err(Refusal::Untrusted));
    o["price_trusted"] = json!(true);
    assert!(document(&o, "ALL", NOW).is_ok());
}

#[test]
fn rule_4_took_money_is_the_kernels_question() {
    for s in ["REJECTED", "CANCELLED", "COMPENSATED_REFUND"] {
        let mut o = taxed();
        o["status"] = json!(s);
        assert_eq!(document(&o, "ALL", NOW), Err(Refusal::NotTaken), "{s}");
    }
    // PENDING and REFUNDING are the venue's money until they are not.
    for s in ["PENDING", "READY", "DELIVERED", "PICKED_UP", "REFUNDING"] {
        let mut o = taxed();
        o["status"] = json!(s);
        assert!(document(&o, "ALL", NOW).is_ok(), "{s}");
    }
    let mut o = taxed();
    o.as_object_mut().unwrap().remove("status");
    assert_eq!(document(&o, "ALL", NOW), Err(Refusal::NotTaken), "no status, no invoice");
}

#[test]
fn a_foreign_invoice_currency_is_refused_and_a_foreign_tender_is_a_payment() {
    let mut o = taxed();
    o["currency"] = json!("EUR");
    assert!(matches!(document(&o, "ALL", NOW), Err(Refusal::Currency(_))));

    let mut o = taxed();
    o["payments"] = json!([
        { "method": "cash", "amount": 500, "currency": "ALL" },
        { "method": "card", "amount": 550, "currency": "EUR", "rate_ppm": 97_500_000 },
    ]);
    let d = document(&o, "ALL", NOW).expect("a EUR tender on a lek bill");
    assert_eq!(d.currency, "ALL");
    assert_eq!(d.kind, Kind::NonCash);
    assert_eq!(d.payments[1].rate_ppm, Some(97_500_000));
}

#[test]
fn lines_the_groups_do_not_count_are_refused() {
    let mut o = taxed();
    o["items"].as_array_mut().unwrap().push(json!({ "name": "Uje", "quantity": 1, "unit_price": 100, "vat_ppm": 200000 }));
    assert!(matches!(document(&o, "ALL", NOW), Err(Refusal::Lines(_))), "2 lines, groups count 1");
    let mut o = taxed();
    o["items"][0].as_object_mut().unwrap().remove("vat_ppm");
    assert!(matches!(document(&o, "ALL", NOW), Err(Refusal::Lines(_))), "an unstamped line");
}

#[test]
fn a_door_payment_is_one_payment_of_the_total() {
    let mut o = taxed();
    o.as_object_mut().unwrap().remove("payments");
    o["payment"] = json!("card");
    let d = document(&o, "ALL", NOW).unwrap();
    assert_eq!(d.payments, vec![Payment { method: "card".into(), amount: 1050, currency: "ALL".into(), rate_ppm: None }]);
}

#[test]
fn the_uuid_is_the_retry_key_same_order_same_uuid_other_order_other_uuid() {
    let a = document(&taxed(), "ALL", NOW).unwrap();
    let b = document(&taxed(), "ALL", NOW + 60_000).unwrap();
    assert_eq!(a.uuid, b.uuid, "re-deriving later is the same invoice");
    let mut o = taxed();
    o["id"] = json!("o2");
    assert_ne!(document(&o, "ALL", NOW).unwrap().uuid, a.uuid);
    let mut o = taxed();
    o["location_id"] = json!("v2");
    assert_ne!(document(&o, "ALL", NOW).unwrap().uuid, a.uuid, "another venue's o1");
    assert_eq!(a.uuid[6] >> 4, 8, "version 8");
    assert_eq!(uuid_text(&a.uuid).len(), 36);
}

#[test]
fn the_corrective_negates_every_amount_and_names_the_original() {
    let d = document(&taxed(), "ALL", NOW).unwrap();
    let c = corrective(&d, NOW + 1);
    assert_eq!(c.corrects, Some(d.uuid));
    assert_ne!(c.uuid, d.uuid);
    assert_eq!((c.total_base, c.total_tax, c.total), (-1050, -175, -1050));
    assert_eq!(c.groups[0].tax, -125);
    assert_eq!(c.fee.as_ref().unwrap().tax, -50);
    assert_eq!(c.lines[0].gross, -750);
    assert_eq!(c.payments[0].amount, -1050);
    assert_eq!(corrective(&d, NOW + 2).uuid, c.uuid, "a re-derived corrective is the same one");
}
