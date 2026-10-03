//! Who hears what: each rule with its positive twin.
use super::*;
use serde_json::Value;

fn sub(lang: &str, n: u8) -> Sub {
    Sub { endpoint: format!("https://fcm.googleapis.com/fcm/send/{n}"), p256dh: format!("P{n}"), auth: format!("A{n}"), lang: lang.into(), at_ms: 1 }
}

fn held() -> Vec<(&'static str, String, Sub)> {
    vec![
        (K_CUSTOMER, "ord1/aaaa".into(), sub("uk", 1)),
        (K_CUSTOMER, "ord2/bbbb".into(), sub("en", 2)),
        (K_COURIER, "cour1/cccc".into(), sub("sq", 3)),
        (K_COURIER, "cour2/dddd".into(), sub("en", 4)),
        (K_STAFF, "owner1/eeee".into(), sub("ru", 5)),
        (K_STAFF, "owner1/ffff".into(), sub("en", 6)),
        (K_STAFF, "cook7/gggg".into(), sub("sq", 7)),
    ]
}

fn msg(e: &Entry) -> Value {
    serde_json::from_str::<Value>(&e.text).unwrap()["msg"].clone()
}

#[test]
fn a_new_order_rings_every_staff_device_and_no_customer_or_courier() {
    let o = owed(Ev::Placed { order: "ord9xxxxxxx" }, &held(), 5);
    let to: Vec<&str> = o.entries.iter().map(|e| e.to.as_str()).collect();
    assert_eq!(to.len(), 3);
    for n in [5, 6, 7] {
        assert!(to.contains(&format!("https://fcm.googleapis.com/fcm/send/{n}").as_str()));
    }
    let ru = o.entries.iter().find(|e| e.to.ends_with("/5")).unwrap();
    assert_eq!(msg(ru)["body"], "Новый заказ");
    assert_eq!(msg(ru)["title"], "Заказ #ord9xxxx");
    assert_eq!(msg(ru)["url"], "/admin/");
    assert!(o.remove.is_empty());
}

#[test]
fn a_status_tells_only_that_orders_customer_in_their_language() {
    let o = owed(Ev::Status { order: "ord1", status: "READY", courier: None }, &held(), 5);
    assert_eq!(o.entries.len(), 1);
    let e = &o.entries[0];
    assert_eq!(e.kind, KIND);
    assert_eq!(e.to, "https://fcm.googleapis.com/fcm/send/1");
    assert_eq!(e.id, "ord1/push/ready/pc/ord1/aaaa");
    assert_eq!(msg(e)["body"], "Готове");
    assert_eq!(msg(e)["url"], "/?order=ord1");
    let v: Value = serde_json::from_str(&e.text).unwrap();
    assert_eq!((v["k"].as_str(), v["id"].as_str(), v["p256dh"].as_str(), v["auth"].as_str()), (Some("pc"), Some("ord1/aaaa"), Some("P1"), Some("A1")));
    assert!(o.remove.is_empty(), "READY is not the end");
}

#[test]
fn pending_tells_nobody_and_confirmed_tells_the_customer() {
    assert!(owed(Ev::Status { order: "ord1", status: "PENDING", courier: None }, &held(), 5).entries.is_empty());
    assert_eq!(owed(Ev::Status { order: "ord1", status: "CONFIRMED", courier: None }, &held(), 5).entries.len(), 1);
}

#[test]
fn ready_with_a_courier_also_tells_that_courier_and_no_other() {
    let o = owed(Ev::Status { order: "ord1", status: "READY", courier: Some("cour1") }, &held(), 5);
    assert_eq!(o.entries.len(), 2);
    let c = o.entries.iter().find(|e| e.to.ends_with("/3")).expect("cour1's phone");
    assert_eq!(msg(c)["body"], "Gati për t'u marrë");
    assert!(!o.entries.iter().any(|e| e.to.ends_with("/4")));
    // and PREPARING with a courier does not ring the courier
    let p = owed(Ev::Status { order: "ord1", status: "PREPARING", courier: Some("cour1") }, &held(), 5);
    assert_eq!(p.entries.len(), 1);
}

#[test]
fn the_end_of_an_order_sends_the_last_word_and_removes_its_customer_devices_only() {
    let o = owed(Ev::Status { order: "ord1", status: "DELIVERED", courier: Some("cour1") }, &held(), 5);
    assert_eq!(o.entries.len(), 1, "the customer hears Delivered");
    assert_eq!(o.remove, vec![(K_CUSTOMER, "ord1/aaaa".to_string())]);
    let live = owed(Ev::Status { order: "ord1", status: "IN_DELIVERY", courier: None }, &held(), 5);
    assert!(live.remove.is_empty());
}

#[test]
fn an_assignment_tells_the_assigned_courier_only() {
    let o = owed(Ev::Assigned { order: "ord2", courier: "cour2" }, &held(), 5);
    assert_eq!(o.entries.len(), 1);
    assert_eq!(o.entries[0].to, "https://fcm.googleapis.com/fcm/send/4");
    assert_eq!(msg(&o.entries[0])["body"], "An order was assigned to you");
    assert!(owed(Ev::Assigned { order: "ord2", courier: "nobody" }, &held(), 5).entries.is_empty());
}

#[test]
fn a_key_that_is_a_prefix_of_another_does_not_hear_its_messages() {
    let mut h = held();
    h.push((K_CUSTOMER, "ord10/zzzz".into(), sub("en", 9)));
    let o = owed(Ev::Status { order: "ord1", status: "READY", courier: None }, &h, 5);
    assert_eq!(o.entries.len(), 1, "ord10 is not ord1");
}

#[test]
fn no_message_carries_more_than_the_order_number() {
    for ev in [Ev::Placed { order: "ord1" }, Ev::Status { order: "ord1", status: "READY", courier: Some("cour1") }, Ev::Assigned { order: "ord1", courier: "cour1" }] {
        for e in owed(ev, &held(), 5).entries {
            let m = msg(&e);
            let mut keys: Vec<&String> = m.as_object().unwrap().keys().collect();
            keys.sort();
            assert_eq!(keys, vec!["body", "tag", "title", "url"]);
        }
    }
}
