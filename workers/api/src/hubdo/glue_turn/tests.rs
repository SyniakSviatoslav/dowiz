//! `/fold/glue` and `/fold/desk` over the in-memory object (W-COV C2): what was STORED is read
//! back by a cold object, not taken from the reply.

use super::super::host::mem::{Harness, T0};
use dowiz_hub::crypto::hex;
use dowiz_hub::glue::tablet;
use serde_json::{json, Value};

/// A tablet's request body for `/fold/glue`.
fn body(img: &[u8], base: [u8; 32]) -> Value {
    let t = tablet::tail(img, "kitchen", &base).unwrap();
    json!({"ctx": "kitchen", "base": hex(&t.base), "tip": hex(&t.tip), "tail": t.recs, "now_ms": T0})
}

/// The reply replayed onto the tablet, as tablet.js does it.
fn replay(img: &[u8], base: [u8; 32], sent: usize, r: &Value) -> (Vec<u8>, [u8; 32]) {
    let entries: Vec<dowiz_hub::glue::merge::Entry> = r["tail"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| match e.as_u64() {
            Some(i) => dowiz_hub::glue::merge::Entry::Echo(i as usize),
            None => dowiz_hub::glue::merge::Entry::Rec(e.as_str().unwrap().to_string()),
        })
        .collect();
    let tip: [u8; 32] = dowiz_hub::crypto::unhex(r["tip"].as_str().unwrap()).unwrap().try_into().unwrap();
    (tablet::rebase(img, &base, false, sent, &entries, &tip).unwrap(), tip)
}

/// The hub's shelf, moved as the object's own doors move it (the stock image, under its generation).
fn hub_moves(h: &Harness, evs: &[dowiz_hub::stock::StockEvent]) {
    let r = h.get("/img/stock");
    let gen = Harness::gen_of(&r);
    let mut log = if r.status_code() == 204 {
        dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap()
    } else {
        dowiz_hub::stock::StockLog::load(r.body()).unwrap()
    };
    log.append_all(evs).unwrap();
    assert_eq!(h.put("stock", gen, &log.to_bytes_trimmed()).status_code(), 200);
}

fn stocked(h: &Harness) {
    use dowiz_hub::stock::StockEvent::Received;
    hub_moves(h, &[Received { item: "salmon".into(), qty: 17 }, Received { item: "rice".into(), qty: 5000 }]);
}

#[test]
fn a_tablet_syncs_its_offline_sale_and_the_oversale_waits_for_a_person() {
    let h = Harness::new();
    stocked(&h);
    // First sync: the hub's log and the tablet's rights.
    let r0 = h.post("/fold/glue", &json!({"ctx": "kitchen", "base": hex(&[0u8; 32]), "tip": hex(&[0u8; 32]), "tail": [], "now_ms": T0}));
    assert_eq!(r0.status_code(), 200, "{}", r0.body_str());
    let (img, base) = replay(&[], [0u8; 32], 0, &r0.body_value());
    // Offline: 2 salmon sold on the tablet; online, 16 at the hub.
    let img = tablet::record(&img, "kitchen", r#"{"k":"served","item":"salmon","qty":2,"order":"k-1"}"#, T0 + 1).unwrap().0;
    hub_moves(&h, &[dowiz_hub::stock::StockEvent::Served { item: "salmon".into(), qty: 16, order_id: "hub-1".into() }]);
    let r1 = h.post("/fold/glue", &body(&img, base));
    assert_eq!(r1.status_code(), 200, "{}", r1.body_str());
    let v = r1.body_value();
    assert_eq!((v["merged"].as_u64(), v["held"].as_array().map(Vec::len)), (Some(1), Some(0)));
    assert!(v["conflicts"].as_array().unwrap().iter().any(|c| c["kind"] == "oversold" && c["item"] == "salmon"), "{v}");
    let (img2, tip) = replay(&img, base, 1, &v);
    assert_eq!(dowiz_hub::stock::StockLog::load(&img2).unwrap().tip_id(), tip, "the tablet lands on the hub's tip");
    // A cold object reads the decision from what was stored, and a person closes it.
    let cold = h.cold();
    let open = cold.post("/fold/desk", &json!({"did": null, "by": "k1"})).body_value();
    let did = open["open"].as_array().unwrap().iter().find(|d| d["kind"] == "oversold").expect("stored")["did"].as_str().unwrap().to_string();
    let closed = cold.post("/fold/desk", &json!({"did": did, "by": "k1"})).body_value();
    assert_eq!(closed["closed"], true);
    assert!(h.cold().post("/fold/desk", &json!({"did": null, "by": "k1"})).body_value()["open"].as_array().unwrap().iter().all(|d| d["did"] != did.as_str()));
}

#[test]
fn a_refused_tail_writes_nothing_and_names_why() {
    let h = Harness::new();
    stocked(&h);
    let r0 = h.post("/fold/glue", &json!({"ctx": "kitchen", "base": hex(&[0u8; 32]), "tip": hex(&[0u8; 32]), "tail": [], "now_ms": T0}));
    let (img, base) = replay(&[], [0u8; 32], 0, &r0.body_value());
    let img = tablet::record(&img, "kitchen", r#"{"k":"served","item":"rice","qty":1,"order":"k-1"}"#, T0).unwrap().0;
    let mut b = body(&img, base);
    b["tail"][0] = json!(b["tail"][0].as_str().unwrap().replace("\"qty\":1", "\"qty\":7"));
    let before = h.cold().post("/fold/desk", &json!({"did": null, "by": "k1"})).body_str().to_string();
    let r = h.post("/fold/glue", &b);
    assert_eq!(r.status_code(), 422, "{}", r.body_str());
    assert!(r.body_str().contains("TipMismatch"), "{}", r.body_str());
    assert_eq!(h.cold().post("/fold/desk", &json!({"did": null, "by": "k1"})).body_str(), before);
    let mut gone = body(&img, base);
    gone["base"] = json!(hex(&[7u8; 32]));
    assert_eq!(h.post("/fold/glue", &gone).status_code(), 409, "a base the hub never held: start over");
    assert_eq!(h.post("/fold/glue", &body(&img, base)).status_code(), 200, "the twin");
}

#[test]
fn the_night_writes_the_stock_checkpoint_a_glue_deferred() {
    let h = Harness::new();
    // 600 records and no checkpoint: what a day's glue leaves (it defers below 2 x 500).
    let mut log = dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap();
    log.set_checkpoint_every(usize::MAX);
    log.append(&dowiz_hub::stock::StockEvent::Received { item: "rice".into(), qty: 5000 }).unwrap();
    for i in 0..599 {
        log.append(&dowiz_hub::stock::StockEvent::Served { item: "rice".into(), qty: 1, order_id: format!("o-{i}") }).unwrap();
    }
    assert_eq!(h.put("stock", 0, &log.to_bytes_trimmed()).status_code(), 200);
    let wrote = crate::edge::mem::block_on(h.obj.night_stock_checkpoint()).unwrap();
    assert!(wrote, "600 >= 500: the night writes it");
    let back = dowiz_hub::stock::StockLog::load(h.cold().get("/img/stock").body()).unwrap();
    assert_eq!(back.verify_checkpoints().unwrap(), 1, "stored, and true to the fold from genesis");
    assert!(!crate::edge::mem::block_on(h.obj.night_stock_checkpoint()).unwrap(), "the twin: not due again, nothing written");
}
