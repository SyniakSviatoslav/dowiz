//! W-NOM: code, barcode and packs -- checked, stored lean, cleared by "".
use super::super::{check as check_supply, record, SupplyIn};
use super::*;

fn body(v: Value) -> SupplyIn {
    serde_json::from_value(v).unwrap()
}

#[test]
fn code_barcode_and_packs_are_stored_and_read_back() {
    let b = body(json!({ "id": "salmon", "code": " FSH-01 ", "barcode": "4006381333931",
                         "packs": [{ "name": "box 5 kg", "qty": 5000 }, { "name": "kg", "qty": 1000 }] }));
    assert_eq!(check_supply(&b).as_deref(), Ok("salmon"));
    let r = record("salmon", &b, &json!({}));
    assert_eq!(r["code"], "FSH-01");
    assert_eq!(r["barcode"], "4006381333931");
    assert_eq!(r["packs"], json!([{ "name": "box 5 kg", "qty": 5000 }, { "name": "kg", "qty": 1000 }]));
    // An edit that does not mention them keeps them; "" and [] clear them (absent, not null).
    let kept = record("salmon", &body(json!({ "id": "salmon", "name": "Salmon" })), &r);
    assert_eq!(kept["packs"], r["packs"]);
    assert_eq!(kept["code"], "FSH-01");
    let cleared = record("salmon", &body(json!({ "id": "salmon", "code": "", "barcode": " ", "packs": [] })), &r);
    for k in ["code", "barcode", "packs"] {
        assert!(cleared.get(k).is_none(), "{k} is left out, not stored as null");
    }
}

#[test]
fn a_bad_code_barcode_or_pack_is_refused_and_its_twin_passes() {
    let refused = |v: Value| check(&body(v)).is_err();
    assert!(refused(json!({ "id": "x", "code": "c".repeat(CODE_MAX + 1) })));
    assert!(!refused(json!({ "id": "x", "code": "c".repeat(CODE_MAX) })));
    assert!(refused(json!({ "id": "x", "barcode": "12345" })), "too short");
    assert!(refused(json!({ "id": "x", "barcode": "40063813339x1" })), "not digits");
    assert!(!refused(json!({ "id": "x", "barcode": "123456" })));
    assert!(!refused(json!({ "id": "x", "barcode": "" })), "empty clears, it is not wrong");
    assert!(refused(json!({ "id": "x", "packs": [{ "name": "box", "qty": 0 }] })));
    assert!(refused(json!({ "id": "x", "packs": [{ "name": "box", "qty": PACK_QTY_MAX + 1 }] })));
    assert!(!refused(json!({ "id": "x", "packs": [{ "name": "box", "qty": PACK_QTY_MAX }] })));
    assert!(refused(json!({ "id": "x", "packs": [{ "name": " ", "qty": 5 }] })));
    assert!(refused(json!({ "id": "x", "packs": [{ "name": "Box", "qty": 5 }, { "name": "box ", "qty": 6 }] })), "one name twice");
    let seven: Vec<Value> = (0..=PACKS_MAX).map(|i| json!({ "name": format!("p{i}"), "qty": 1 })).collect();
    assert!(refused(json!({ "id": "x", "packs": seven })));
    let six: Vec<Value> = (0..PACKS_MAX).map(|i| json!({ "name": format!("p{i}"), "qty": 1 })).collect();
    assert!(!refused(json!({ "id": "x", "packs": six })));
    assert!(serde_json::from_value::<SupplyIn>(json!({ "id": "x", "packs": [{ "name": "a", "qty": 1, "price": 3 }] })).is_err(), "a pack has two fields");
}
