//! `/fold/glue_orders` over the in-memory object, with the REAL deciders: what was STORED is read
//! back by a cold object, not taken from the reply.

use super::super::super::host::mem::{Harness, T0};
use dowiz_hub::stock::{reservations_for, StockEvent, StockLog};
use dowiz_hub::{EventKind, Hub};
use serde_json::{json, Value};

const ROLL: &str = r#"{"id":"p1","bom":[{"supply":"salmon","qty":40}]}"#;

fn order(id: &str, status: &str) -> String {
    json!({
        "id": id, "order_id": id, "location_id": "v1", "status": status,
        "items": [{"product_id": "p1", "quantity": 2, "unit_price": 900, "name": "Sake"}],
        "subtotal": 1800, "total": 1800, "payment": "cash",
        "contact": {"name": "C", "phone": "+355690000000"}, "created_at_ms": T0 - 60_000,
    })
    .to_string()
}

/// o1 CONFIRMED with its salmon held; o2 placed and then CANCELLED by the hub while the tablet
/// was offline.
fn seeded() -> Harness {
    let h = Harness::new();
    let mut hub = Hub::create_sized(64 * 1024).unwrap();
    hub.append(EventKind::Placed, "o1", &order("o1", "CONFIRMED"), 1, [0; 32]).unwrap();
    hub.append(EventKind::Placed, "o2", &order("o2", "PENDING"), 2, [0; 32]).unwrap();
    hub.append(EventKind::Advanced, "o2", r#"{"_d":true,"status":"CANCELLED"}"#, 3, [0; 32]).unwrap();
    assert_eq!(h.put("log", 0, &hub.to_bytes_trimmed()).status_code(), 200);
    let mut stock = StockLog::create_sized(64 * 1024).unwrap();
    stock.append(&StockEvent::Received { item: "salmon".into(), qty: 1000 }).unwrap();
    stock.append_all(&reservations_for("o1", &[(ROLL.into(), 2)])).unwrap();
    assert_eq!(h.put("stock", 0, &stock.to_bytes_trimmed()).status_code(), 200);
    h
}

fn tail() -> Vec<String> {
    vec![
        r#"{"op":"ack","oid":"9.0","order":"o1","at":1699999990000}"#.into(),
        r#"{"op":"advance","oid":"9.1","order":"o1","from":"CONFIRMED","next":"PREPARING","at":1699999991000}"#.into(),
        r#"{"op":"advance","oid":"9.2","order":"o2","from":"PENDING","next":"CONFIRMED","at":1699999992000}"#.into(),
    ]
}

fn sync(h: &Harness, tail: &[String]) -> crate::wire::Reply {
    h.post("/fold/glue_orders", &json!({"ctx": "kitchen", "location_id": "v1", "by": "cook-1", "tail": tail, "now_ms": T0}))
}

fn stored_order(h: &Harness, id: &str) -> Value {
    let hub = Hub::load(h.cold().get("/img/log").body()).unwrap();
    serde_json::from_str(&dowiz_hub::room::view::current(&hub, id).unwrap().order_json).unwrap()
}

#[test]
fn offline_taps_are_decided_by_the_objects_own_deciders_and_none_is_lost() {
    let h = seeded();
    let r = sync(&h, &tail());
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!((v["sent"].as_u64(), v["merged"].as_u64(), v["applied"].as_u64(), v["notes"].as_u64()), (Some(3), Some(2), Some(2), Some(1)), "{v}");
    assert_eq!(v["held"][0]["pos"], 2);
    assert!(v["held"][0]["why"].as_str().unwrap().starts_with("IllegalTransitionError"), "{v}");
    let o1 = stored_order(&h, "o1");
    assert_eq!(o1["status"], "PREPARING", "stored, read by a cold object");
    assert_eq!(o1["kitchen"]["seen"]["by"], "cook-1", "the signer is the token's");
    assert_eq!(o1["kitchen"]["seen"]["at"], 1_699_999_990_000i64, "seen when the kitchen saw it");
    let shelf = StockLog::load(h.cold().get("/img/stock").body()).unwrap();
    assert_eq!(shelf.ledger().unwrap().level("salmon").reserved, 0, "PREPARING consumed the hold in the same turn");
    let open = h.cold().post("/fold/desk", &json!({"did": null, "by": "k1"})).body_value();
    assert!(open["open"].as_array().unwrap().iter().any(|d| d["did"] == "o-kitchen-9.2" && d["order"] == "o2"), "{open}");
    // The same tail again (a retry after a lost reply): adopted once, nothing written.
    let again = sync(&h, &tail()).body_value();
    assert_eq!((again["merged"].as_u64(), again["applied"].as_u64(), again["notes"].as_u64()), (Some(2), Some(0), Some(0)), "{again}");
    assert_eq!(again["held"].as_array().map(Vec::len), Some(1));
}

#[test]
fn a_tail_with_a_foreign_key_is_refused_whole_and_writes_nothing() {
    let h = seeded();
    let mut t = tail();
    t[1] = t[1].replace("\"at\"", "\"phone\":\"1\",\"at\"");
    let r = sync(&h, &t);
    assert_eq!(r.status_code(), 422, "{}", r.body_str());
    assert!(r.body_str().contains("phone"), "{}", r.body_str());
    assert_eq!(stored_order(&h, "o1")["status"], "CONFIRMED", "the ack before it was not half-adopted");
    assert_eq!(sync(&h, &tail()).status_code(), 200, "the twin");
}
