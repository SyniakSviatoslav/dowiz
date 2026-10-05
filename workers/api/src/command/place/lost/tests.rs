//! A13 and R13 through the real `decide` (W-LOST): a basket the shelf refuses
//! leaves exactly one `refused` row and no order; an option with a recipe
//! reserves its supply with the dish's.

use super::*;
use dowiz_hub::stock::{StockEvent, StockLog};

const NOW: i64 = 1_759_000_000_000;

fn input(lines: Vec<(String, i64)>) -> PlaceIn {
    PlaceIn {
        order_id: "o1".into(),
        envelope: serde_json::json!({ "order_id": "o1", "status": "PENDING", "subtotal": 800, "total": 800 }).to_string(),
        seq: 1,
        bom_lines: lines,
        promo: None,
        promo_code: None,
        subtotal: 800,
        fee: 0,
        tip: 0,
        now_ms: NOW,
        notify_text: None,
        stamps: None,
        welcome: None,
    }
}

const ROLL: &str = r#"{"id":"roll","price":800,"bom":[{"supply":"salmon","qty":40}],"optionBom":{"xsalmon":[{"supply":"salmon","qty":20}]},
  "modifierGroups":[{"id":"extra","name":"Extras","min":0,"max":2,"options":[{"id":"xsalmon","name":"Extra salmon","priceDelta":200}]}]}"#;

fn images(salmon: i64) -> (dowiz_hub::Hub, StockLog) {
    let mut stock = StockLog::create_sized(64 * 1024).unwrap();
    stock.set_clock(NOW);
    stock.append(&StockEvent::Received { item: "salmon".into(), qty: salmon }).unwrap();
    (dowiz_hub::Hub::create_sized(64 * 1024).unwrap(), stock)
}

fn lines(chosen: &[&str]) -> Vec<(String, i64)> {
    let ids: Vec<String> = chosen.iter().map(|s| s.to_string()).collect();
    let mut l = vec![(ROLL.to_string(), 1)];
    l.extend(dowiz_hub::modifiers::bom::option_line("roll", ROLL, &ids, 1));
    l
}

#[test]
fn a_refused_basket_yields_exactly_one_refused_row_and_no_order() {
    // The probe's numbers: 30 g counted, a dish needing 40 g.
    let (mut hub, mut stock) = images(30);
    let before = stock.len();
    let out = decide(&mut hub, &mut stock, &[], &Ok(None), &input(lines(&[])));
    assert!(matches!(&out, Err(Refused::Stock(m)) if m.contains("salmon")), "refused, naming the supply: {out:?}");
    assert_eq!(hub.len(), 0, "no order");
    assert_eq!(stock.len(), before + 1, "one record: the refusal, no reservation");
    let rows = stock.lost_rows();
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].sale.item.as_str(), rows[0].sale.dish.as_str(), rows[0].sale.qty, rows[0].sale.price, rows[0].at), ("salmon", "roll", 1, 800, NOW));
    // The guest taps again a minute later: still one lost sale.
    stock.set_clock(NOW + 60_000);
    let again = input(lines(&[]));
    assert!(decide(&mut hub, &mut stock, &[], &Ok(None), &again).is_err());
    assert_eq!(stock.lost_rows().len(), 1, "rate-limited: one row per supply per ten minutes");
    assert_eq!(stock.ledger().unwrap().level("salmon").on_hand, 30, "and the shelf never moved");
}

#[test]
fn extra_salmon_reserves_twenty_grams_more_and_its_refusal_names_the_dish() {
    let (mut hub, mut stock) = images(100);
    decide(&mut hub, &mut stock, &[], &Ok(None), &input(lines(&["xsalmon"]))).expect("60 g of 100");
    assert_eq!(stock.ledger().unwrap().level("salmon").reserved, 60, "40 g for the roll + 20 g for the extra");
    let (mut hub2, mut stock2) = images(100);
    decide(&mut hub2, &mut stock2, &[], &Ok(None), &input(lines(&[]))).expect("40 g of 100");
    assert_eq!(stock2.ledger().unwrap().level("salmon").reserved, 40, "the twin: no option, the roll's 40 g");
    // 50 g: the roll fits, the extra does not -- the whole basket is refused,
    // and the lost sale is the roll at its price with the option's delta.
    let (mut hub3, mut stock3) = images(50);
    assert!(decide(&mut hub3, &mut stock3, &[], &Ok(None), &input(lines(&["xsalmon"]))).is_err());
    let rows = stock3.lost_rows();
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].sale.dish.as_str(), rows[0].sale.price), ("roll", 1000), "800 + the extra's 200");
    assert_eq!(stock3.ledger().unwrap().level("salmon").reserved, 0, "nothing held for a refused basket");
}

#[test]
fn the_cost_stamp_does_not_take_the_option_line_for_a_dish() {
    let (mut hub, mut stock) = images(100);
    let mut i = input(lines(&["xsalmon"]));
    i.envelope = serde_json::json!({ "order_id": "o1", "status": "PENDING", "subtotal": 1000, "total": 1000,
        "items": [{ "product_id": "roll", "quantity": 1, "unit_price": 1000 }] })
    .to_string();
    let stored = decide(&mut hub, &mut stock, &[], &Ok(None), &i).expect("placed");
    let v: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(v["items"].as_array().map(Vec::len), Some(1), "one line in, one line out");
}
