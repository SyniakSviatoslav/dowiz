//! The kitchen's ticket carries the dishes and never the customer. Every
//! refusal here has a positive twin: the planted value is gone AND the dish,
//! table and time are still there.

use super::*;

const PHONE: &str = "+355691234567";
const NAME: &str = "Arta Hoxha";
const STREET: &str = "Rruga Taulantia 14";

/// An order as `storefront.rs` writes it, with personal data planted in every
/// place the envelope carries it.
fn planted(status: &str, venue: &str, at: i64) -> Value {
    json!({
        "id": "o1", "status": status, "location_id": venue, "created_at_ms": at,
        "customer_id": "cust_77", "total": 2400, "subtotal": 2000, "tip": 100, "delivery_fee": 300,
        "payment": "card", "crypto": { "wallet": { "address": "0xabc" } },
        "contact": { "name": NAME, "phone": PHONE },
        "fulfilment": {
            "kind": "delivery", "table": null, "note": "pa susam", "fee": 300,
            "address": { "line": STREET, "note": "kati 3", "lat_udeg": 41_311_000, "lon_udeg": 19_441_000 }
        },
        "items": [
            { "product_id": "p1", "name": "Dragon roll", "quantity": 2, "unit_price": 1000,
              "vat_ppm": 200000, "modifier_ids": ["m1"], "station": "bar" }
        ],
        "kitchen": { "seen": { "by": "staff_1", "at": at + 5 } },
        "at": { "CONFIRMED": at + 10 },
        "placed_by": "staff_9", "cash_pay_with": "5000",
    })
}

#[test]
fn a_planted_phone_name_address_or_price_never_reaches_the_ticket() {
    let t = ticket(&planted("PENDING", "v", 1)).to_string();
    for leak in [PHONE, NAME, STREET, "kati 3", "cust_77", "0xabc", "unit_price", "total", "payment",
                 "vat_ppm", "cash_pay_with", "lat_udeg", "\"tip\"", "delivery_fee", "placed_by", "5000"] {
        assert!(!t.contains(leak), "{leak} reached the kitchen: {t}");
    }
}

#[test]
fn the_ticket_keeps_what_the_pass_cooks_from() {
    let t = ticket(&planted("PENDING", "v", 1));
    assert_eq!(t["items"][0]["name"], "Dragon roll");
    assert_eq!(t["items"][0]["quantity"], 2);
    assert_eq!(t["items"][0]["station"], "bar");
    assert_eq!(t["items"][0]["modifier_ids"], json!(["m1"]));
    assert_eq!(t["fulfilment"], json!({ "kind": "delivery", "table": null, "note": "pa susam" }));
    assert_eq!(t["kitchen"]["seen"]["by"], "staff_1");
    assert_eq!(t["at"]["CONFIRMED"], 11);
    assert_eq!(t["status"], "PENDING");
}

/// A DELTA IS STRIPPED THE SAME WAY and stays a delta: its marker survives,
/// a removal of a field the kitchen may know survives, a removal naming a
/// private one is dropped (it names what exists).
#[test]
fn a_delta_is_stripped_and_stays_a_delta() {
    let d = json!({ "_d": 1, "_x": ["contact", "rejection_reason"], "status": "READY",
                    "contact": { "phone": PHONE }, "fulfilment": { "address": { "line": STREET } } });
    let t = ticket(&d);
    assert_eq!(t["_d"], 1);
    assert_eq!(t["_x"], json!(["rejection_reason"]));
    assert_eq!(t["status"], "READY");
    let s = t.to_string();
    assert!(!s.contains(PHONE) && !s.contains(STREET) && !s.contains("contact"), "{s}");
}

#[test]
fn unreadable_payload_text_is_an_empty_delta_not_the_original_bytes() {
    let raw = format!("{{not json {PHONE}");
    assert_eq!(ticket_text(&raw), r#"{"_d":1}"#);
    assert!(!ticket_text(&planted("PENDING", "v", 1).to_string()).contains(PHONE));
    assert_eq!(ticket(&json!("a string")), json!({}));
}

/// The socket frame is the console's shape with the stripped payload.
#[test]
fn the_socket_frame_carries_no_customer() {
    let f = frame(1, "o1", &planted("PENDING", "v", 1).to_string(), 42);
    let v: Value = serde_json::from_str(&f).expect("a frame is json");
    assert_eq!(v["t"], "event");
    assert_eq!(v["orderId"], "o1");
    assert_eq!(v["generation"], 42);
    assert!(v["payload"].as_str().expect("text").contains("Dragon roll"));
    assert!(!f.contains(PHONE) && !f.contains(STREET) && !f.contains(NAME), "{f}");
}

#[test]
fn the_board_is_this_venues_open_tickets_oldest_first() {
    let o = |id: &str, st: &str, venue: &str, at: i64| (id.to_string(), planted(st, venue, at).to_string());
    let listed = vec![
        o("late", "PREPARING", "v", 30),
        o("early", "PENDING", "v", 10),
        o("other", "PENDING", "w", 5),
        o("done", "PICKED_UP", "v", 1),
        o("gone", "DELIVERED", "v", 2),
        o("ready", "READY", "v", 20),
        ("bad".to_string(), "{not json".to_string()),
    ];
    let b = board(&listed, "v");
    let ids: Vec<&str> = b.iter().map(|t| t["id"].as_str().unwrap_or("")).collect();
    assert_eq!(ids, vec!["early", "ready", "late"]);
    assert!(!serde_json::to_string(&b).expect("json").contains(PHONE));
    assert!(is_open(&json!({ "status": "CONFIRMED" })));
    assert!(!is_open(&json!({ "status": "IN_DELIVERY" })) && !is_open(&json!({})));
}

#[test]
fn the_catch_up_is_fenced_to_one_venue_and_stripped() {
    let ch = |id: &str, payload: Value| crate::hubdo::Change {
        generation: 7, kind: 2, order_id: id.into(), payload: payload.to_string(),
    };
    let got = changes(
        &[
            ch("mine", planted("PENDING", "v", 1)),
            ch("theirs", planted("PENDING", "w", 1)),
            ch("delta", json!({ "_d": 1, "status": "READY" })),
        ],
        "v",
    );
    let ids: Vec<&str> = got.iter().map(|c| c["order_id"].as_str().unwrap_or("")).collect();
    assert_eq!(ids, vec!["mine", "delta"]);
    assert!(!serde_json::to_string(&got).expect("json").contains(PHONE));
    assert_eq!(got[0]["generation"], 7);
}

#[test]
fn the_kitchen_tag_is_its_own_audience() {
    assert_ne!(TAG_KITCHEN, crate::hubdo::TAG_CONSOLE);
    assert_ne!(TAG_KITCHEN, crate::hubdo::TAG_COURIER);
    assert!(!TAG_KITCHEN.starts_with("order:"));
}
