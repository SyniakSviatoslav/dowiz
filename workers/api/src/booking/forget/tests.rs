//! G8: the person's bookings are found under every key of their circle and
//! emptied of them; the reservation, its events and its table stay.

use super::super::store::{write_new, NewBooking};
use super::super::guest::Side;
use super::super::{events_of, fold_status};
use super::*;

const CEIL: usize = 256 * 1024;
const SLOT: i64 = 29_000_000;
const NOW: i64 = SLOT - 600;

fn booking(id: &str, name: &str, phone: &str, phone_key: Option<&str>) -> NewBooking {
    NewBooking {
        id: id.into(),
        venue: "v1".into(),
        party: 2,
        slot_min: SLOT,
        occasion: String::new(),
        name: name.into(),
        phone: phone.into(),
        phone_key: phone_key.map(str::to_string),
        user_id: None,
        table: None,
        side: Side::Guest,
        now_ms: NOW * 60_000,
    }
}

fn image() -> Table {
    let mut t = Table::create(CEIL).unwrap();
    let plan = dowiz_hub::tables::Plan::default();
    for b in [
        booking("b1", "Arben Hoxha", "+355 69 111 1111", Some("k_e164")),
        // Made before `phone_key` existed: only the typed number finds it.
        booking("b2", "Arben", "069 111 1111", None),
        booking("b3", "Stranger", "+355 69 222 2222", Some("k_other")),
    ] {
        write_new(&mut t, &b, &plan, NOW).unwrap().unwrap();
    }
    t
}

/// The fake HMAC: every spelling of Arben's number maps to its circle key.
fn key_of(p: &str) -> Vec<String> {
    let d: String = p.chars().filter(char::is_ascii_digit).collect();
    match d.as_str() {
        "0691111111" => vec!["k_national".into()],
        "355691111111" => vec!["k_e164".into()],
        _ => vec![format!("k_{d}")],
    }
}

fn circle() -> BTreeSet<String> {
    ["k_e164", "k_national"].iter().map(|s| s.to_string()).collect()
}

#[test]
fn every_spelling_of_the_person_finds_their_bookings_and_no_stranger() {
    let t = image();
    assert_eq!(ids_of(&t, &circle(), key_of), vec!["b1".to_string(), "b2".to_string()]);
    // The twin: one spelling alone finds only its own booking.
    let only: BTreeSet<String> = ["k_national".to_string()].into();
    assert_eq!(ids_of(&t, &only, key_of), vec!["b2".to_string()]);
}

#[test]
fn the_contact_goes_and_the_reservation_and_its_history_stay() {
    let mut t = image();
    let ids = ids_of(&t, &circle(), key_of);
    let before_events = events_of(&t, "b1").len();
    assert!(!t.scan("rsv.phone/k_e164/").is_empty(), "the fixture indexes the phone key");
    assert_eq!(redact(&mut t, &ids).unwrap(), 2);
    for id in ["b1", "b2"] {
        let r: Value = serde_json::from_str(&t.get(K_RSV, id).unwrap()).unwrap();
        assert_eq!(r["contact_name"], json!(""), "{id}");
        assert_eq!(r["contact_phone"], json!(""), "{id}");
        assert_eq!(r["phone_key"], Value::Null, "{id}");
        assert_eq!(r["party"], json!(2), "{id}: the booking itself stays");
        assert_eq!(fold_status(&events_of(&t, id)).unwrap().as_str(), "REQUESTED", "{id}");
    }
    assert_eq!(events_of(&t, "b1").len(), before_events, "no event is touched");
    assert!(t.scan("rsv.phone/k_e164/").is_empty(), "the phone index row goes with the key");
    let bytes = t.to_bytes().unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("Arben") && !text.contains("111 1111"), "the person is out of the image");
    // The stranger is untouched.
    let s: Value = serde_json::from_str(&t.get(K_RSV, "b3").unwrap()).unwrap();
    assert_eq!(s["contact_name"], json!("Stranger"));
    assert!(!t.scan("rsv.phone/k_other/").is_empty());
    // Idempotent: a second run changes nothing.
    assert_eq!(redact(&mut t, &ids).unwrap(), 0);
}

/// With the REAL keys: a booking typed as `069 111 1111` is filed under the
/// canonical key `guest::phone_key` writes, so forgetting the `+355` row finds
/// it even with nothing linked -- and a stranger's booking is not found.
#[test]
fn the_real_keys_find_a_national_spelling_booking_from_the_e164_row() {
    const SECRET: &[u8] = b"test-secret-key";
    let mut t = Table::create(CEIL).unwrap();
    let plan = dowiz_hub::tables::Plan::default();
    for (id, phone) in [("b1", "069 111 1111"), ("b2", "069 222 2222")] {
        let pk = super::super::guest::phone_key(SECRET, phone);
        write_new(&mut t, &booking(id, "Arben", phone, pk.as_deref()), &plan, NOW).unwrap().unwrap();
    }
    let row = crate::services::customers::handlers::customer_key(SECRET, "+355691111111");
    let keys = BTreeSet::from([row]);
    assert_eq!(ids_of(&t, &keys, |p| keys_of_phone(SECRET, p)), vec!["b1".to_string()]);
}
