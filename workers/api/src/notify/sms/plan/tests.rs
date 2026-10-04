use super::*;
use serde_json::json;

fn order(kind: &str, stamped: bool) -> Value {
    let mut o = json!({
        "currency": "ALL",
        "contact": {"name": "Arben Hoxha", "phone": "069 123 4567"},
        "fulfilment": {"kind": kind, "address": {"line": "Rruga Tregtare 1"}},
    });
    if stamped {
        o["sms"] = json!({"key": "k1", "lang": "sq", "venue": "Sushi Durrës", "to": "+355691234567"});
    }
    o
}

#[test]
fn a_ticked_box_owes_one_text_per_status_worth_one() {
    let e = owed("1a2b3c4d-x", &order("delivery", true), "CONFIRMED", 5).unwrap();
    assert_eq!((e.id.as_str(), e.kind.as_str(), e.to.as_str()), ("1a2b3c4d-x/sms/confirmed", KIND, "+355691234567"));
    let p: Payload = serde_json::from_str(&e.text).unwrap();
    assert_eq!(p.key, "k1");
    assert_eq!(p.body, "Sushi Durres: Porosia #1a2b3c4d - U konfirmua. STOP: thuajini lokalit");
    assert!(!e.text.contains("Arben") && !e.text.contains("Tregtare"), "no name, no address: {}", e.text);
    assert!(owed("o", &order("delivery", true), "IN_DELIVERY", 5).is_some());
    assert!(owed("o", &order("pickup", true), "READY", 5).is_some());
}

#[test]
fn nothing_is_owed_without_the_tick_or_for_a_quiet_status() {
    assert_eq!(owed("o", &order("delivery", false), "CONFIRMED", 5), None, "the box was not ticked");
    assert_eq!(owed("o", &order("delivery", true), "PREPARING", 5), None);
    assert_eq!(owed("o", &order("delivery", true), "READY", 5), None, "a delivery hears 'on the way'");
    assert_eq!(owed("o", &order("dine_in", true), "CONFIRMED", 5), None, "a table is in the room");
    let mut o = order("delivery", true);
    o["sms"]["to"] = json!("069 123 4567");
    assert_eq!(owed("o", &o, "CONFIRMED", 5), None, "a stamp that is not E.164 is not guessed at");
}
