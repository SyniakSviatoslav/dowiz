//! R4's CHECK, through the real `place::decide`: two receipts at 1,000 and
//! 1,200 per kg; an order placed between them stamps the 1,000 cost, keeps it
//! after the second, and law 8's rebuild reproduces it from the log.

use super::*;
use crate::command::place::{decide, PlaceIn};
use dowiz_hub::stock::cost::{rebuild, Price, Stamp};
use dowiz_hub::stock::{bom_of, StockLog};

const MAKI: &str = r#"{"id":"maki","name":"Maki","bom":[{"supply":"rice","qty":100}]}"#;
const WATER: &str = r#"{"id":"water","name":"Water"}"#;
const TUNA: &str = r#"{"id":"tuna","bom":[{"supply":"tuna","qty":50}]}"#;

fn price(c: i64) -> Price {
    Price { unit_cost: c, per: 1000, supplier: None, doc: None }
}

fn input(id: &str, lines: &[(&str, &str, i64)]) -> PlaceIn {
    let items: Vec<Value> = lines.iter().map(|(pid, _, q)| json!({"product_id": pid, "quantity": q, "unit_price": 500})).collect();
    PlaceIn {
        order_id: id.into(),
        envelope: json!({"order_id": id, "status": "PENDING", "items": items, "subtotal": 500, "total": 500}).to_string(),
        seq: 1,
        bom_lines: lines.iter().map(|(_, p, q)| (p.to_string(), *q)).collect(),
        promo: None,
        promo_code: None,
        subtotal: 500,
        fee: 0,
        tip: 0,
        now_ms: 1_790_000_000_000,
        notify_text: None,
        stamps: None, welcome: None,
    }
}

fn place(stock: &mut StockLog, i: &PlaceIn) -> Value {
    let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let stored = decide(&mut hub, stock, &[], &Ok(None), i).expect("placed");
    serde_json::from_str(&stored).unwrap()
}

#[test]
fn the_stamp_is_the_average_at_placement_and_does_not_move() {
    let mut stock = StockLog::create_sized(64 * 1024).unwrap();
    stock.receive_priced("rice", 1000, &price(1000)).unwrap();
    let at = stock.len();
    let o1 = place(&mut stock, &input("o1", &[("maki", MAKI, 2)]));
    assert_eq!(o1["items"][0]["unit_cost"], json!(100), "100 g at 1,000 per kg");
    assert_eq!(o1["items"][0]["cost_at"], json!(at), "folded at the log's length before the hold");
    assert_eq!(line_cost(&o1["items"][0], 2), Some(200));

    stock.receive_priced("rice", 1000, &price(1200)).unwrap();
    let o2 = place(&mut stock, &input("o2", &[("maki", MAKI, 1)]));
    assert_eq!(o2["items"][0]["unit_cost"], json!(110), "a NEW order stamps the new average");
    assert_eq!(o1["items"][0]["unit_cost"], json!(100), "the stored one is the one it was placed at");

    // LAW 8: from the persisted bytes alone.
    let back = StockLog::load(&stock.to_bytes_trimmed()).unwrap();
    let s = Stamp { cost: 100, at };
    assert_eq!(rebuild(&back, &s, &bom_of(MAKI)), Some(100));
}

#[test]
fn only_a_fully_priced_recipe_is_stamped() {
    let mut stock = StockLog::create_sized(64 * 1024).unwrap();
    stock.receive_priced("rice", 1000, &price(1000)).unwrap();
    stock.append(&dowiz_hub::stock::StockEvent::Received { item: "tuna".into(), qty: 500 }).unwrap();
    let o = place(&mut stock, &input("o1", &[("maki", MAKI, 1), ("water", WATER, 1), ("tuna", TUNA, 1)]));
    assert_eq!(o["items"][0]["unit_cost"], json!(100), "the priced recipe");
    assert!(o["items"][1].get("unit_cost").is_none(), "no recipe, no cost");
    assert!(o["items"][2].get("unit_cost").is_none(), "tuna was never priced: a partial sum is not a cost");
    assert_eq!(line_cost(&o["items"][2], 3), None);
    // A basket with no recipe at all reserves nothing and folds nothing.
    let before = stock.len();
    let w = place(&mut stock, &input("o2", &[("water", WATER, 1)]));
    assert!(w["items"][0].get("cost_at").is_none());
    assert_eq!(stock.len(), before);
}

#[test]
fn a_line_whose_product_is_not_in_the_basket_lines_is_left_alone() {
    let mut env = json!({"items": [{"product_id": "ghost", "quantity": 1}, {"quantity": 1}]});
    let book = dowiz_hub::stock::cost::CostBook::default();
    stamp_lines(&mut env, &[(MAKI.into(), 1), ("not json".into(), 1)], &book, 3, &Default::default());
    assert_eq!(env, json!({"items": [{"product_id": "ghost", "quantity": 1}, {"quantity": 1}]}));
}

#[test]
fn a_guest_never_reads_the_cost() {
    let mut o = json!({"items": [{"product_id": "maki", "unit_price": 500, "unit_cost": 100, "cost_at": 4}, "junk"], "total": 500});
    strip(&mut o);
    assert_eq!(o, json!({"items": [{"product_id": "maki", "unit_price": 500}, "junk"], "total": 500}));
    // Twin: an order with no lines is untouched.
    let mut bare = json!({"total": 1});
    strip(&mut bare);
    assert_eq!(bare, json!({"total": 1}));
}

/// W-PF3 T1: a kitchen where the seasoned rice is a semi-finished product
/// (card: 100 g of rice makes 100 g), priced so the batch and today's raw
/// rice differ: the first kilo at 1,000/kg is cooked into 500 g of batch,
/// then a kilo at 3,000/kg arrives -- the raw average is now above 2 per g,
/// the batch's own is 1.
fn pf_kitchen(cook: i64) -> (StockLog, String) {
    let supplies = [
        ("rice", r#"{"id":"rice","name":"Rice","unit":"g","kind":"food_ingredient"}"#),
        ("rice-s", r#"{"id":"rice-s","name":"Rice seasoned","unit":"g","kind":"prep","card":{"lines":[{"item":"rice","qty":100}],"yield":100}}"#),
    ];
    let look = |id: &str| supplies.iter().find(|(i, _)| *i == id).map(|(_, j)| j.to_string());
    let mut stock = StockLog::create_sized(64 * 1024).unwrap();
    stock.receive_priced("rice", 1000, &price(1000)).unwrap();
    if cook > 0 {
        let act = dowiz_hub::stock::act::Act { prep: "rice-s".into(), planned: cook, out: cook, act: "pa_1".into(), by: "cook-1".into(), lot: None, expiry: None };
        assert_eq!(stock.cook(&act, &look).unwrap().value, Some(cook), "the batch cost 1 per g");
    }
    stock.receive_priced("rice", 1000, &price(3000)).unwrap();
    let dish = dowiz_hub::prep::for_ledger(&look, r#"{"id":"roll","name":"Roll","bom":[{"supply":"rice-s","qty":100}]}"#).0;
    assert!(dish.contains(r#""tree""#));
    (stock, dish)
}

#[test]
fn a_sale_from_a_ready_batch_is_stamped_at_the_batch_cost() {
    let (mut stock, dish) = pf_kitchen(500);
    let o = place(&mut stock, &input("o1", &[("roll", &dish, 2)]));
    // Priced as all raw it would be 100 g at (500 x 1 + 1000 x 3) / 1500 g = 233.
    assert_eq!(o["items"][0]["unit_cost"], json!(100), "100 g from the batch at 1 per g, not today's raw average");
}

#[test]
fn a_short_batch_stamps_its_part_at_the_batch_cost_and_the_rest_at_raw() {
    // 150 g ready for two rolls of 100 g: each roll takes 75 g of batch at 1
    // and 25 g of rice at (850 x 1 + 1000 x 3) / 1850 = 2.081081 -> 75 + 52.03 = 127.
    let (mut stock, dish) = pf_kitchen(150);
    let o = place(&mut stock, &input("o1", &[("roll", &dish, 2)]));
    assert_eq!(o["items"][0]["unit_cost"], json!(127));
}

#[test]
fn with_no_batch_ready_the_stamp_is_the_raw_cost_as_before() {
    // Twin: nothing cooked, rice at (1,000 + 3,000) / 2 kg = 2 per g.
    let (mut stock, dish) = pf_kitchen(0);
    let o = place(&mut stock, &input("o1", &[("roll", &dish, 2)]));
    assert_eq!(o["items"][0]["unit_cost"], json!(200));
}

/// MEASURED, not asserted (`cargo test --release --lib measure_ -- --ignored
/// --nocapture`): the two folds a placement pays for, at the same history --
/// the order log's projection (`orders_view` refolds it after every write)
/// and the stock ledger (from genesis, and from the newest checkpoint).
#[test]
#[ignore]
fn measure_order_log_fold_against_the_stock_fold() {
    for orders in [1_000usize, 8_000] {
        let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
        let mut stock = StockLog::create_sized(64 * 1024).unwrap();
        stock.receive_priced("rice", 1_000_000_000, &price(1000)).unwrap();
        for k in 0..orders {
            let id = format!("ord_{k}");
            let i = input(&id, &[("maki", MAKI, 1)]);
            // As the object does (`hubdo.rs` place/advance): every record dated.
            stock.set_clock(1_790_000_000_000 + k as i64 * 60_000);
            decide(&mut hub, &mut stock, &[], &Ok(None), &i).unwrap();
            let led = stock.ledger().unwrap();
            stock.append_all(&dowiz_hub::stock::settle(&led, &id, true)).unwrap();
            for (n, st) in ["CONFIRMED", "PREPARING", "DELIVERED"].iter().enumerate() {
                let body = json!({"status": st}).to_string();
                hub.append(dowiz_hub::EventKind::Advanced, &id, &body, 2 + n as u64, [0u8; 32]).unwrap();
            }
        }
        let time = |f: &dyn Fn()| {
            let t = std::time::Instant::now();
            for _ in 0..10 {
                f();
            }
            t.elapsed().as_micros() / 10
        };
        let o = time(&|| { std::hint::black_box(crate::hubstore::orders_state(&hub)); });
        // The fold `ledger()` WAS until R7, over the same records.
        let g = time(&|| { std::hint::black_box(dowiz_hub::stock::StockLedger::fold(&stock.events()).unwrap()); });
        let c = time(&|| { std::hint::black_box(stock.ledger().unwrap()); });
        println!(
            "orders={orders} hub_events={} stock_records={}: orders_state={o}us ledger_genesis={g}us ledger_checkpoint={c}us hub_image={}B stock_image={}B",
            hub.len(), stock.len(), hub.to_bytes_trimmed().len(), stock.to_bytes_trimmed().len()
        );
    }
}
