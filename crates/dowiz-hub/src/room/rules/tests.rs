//! The shared rules, each refusal beside its positive twin.

use super::*;
use serde_json::json;

fn round() -> Value {
    json!({"status": "PENDING", "location_id": "v1", "delivery_fee": 0, "tip": 0,
           "items": [{"product_id": "maki", "quantity": 2, "unit_price": 600}]})
}

#[test]
fn a_paid_round_is_not_changeable_and_an_unpaid_one_is() {
    let mut r = round();
    assert_eq!(changeable(&r, "v1", "p1"), Ok(Stage::BeforeKitchen));
    assert_eq!(changeable(&r, "v2", "p1"), Err(Refused::NotFound));
    assert!(matches!(changeable(&r, "v1", " "), Err(Refused::Invalid(_))));
    r["payment_status"] = json!("paid");
    assert!(matches!(changeable(&r, "v1", "p1"), Err(Refused::Conflict(_))));
}

#[test]
fn reprice_refuses_what_was_paid_above_the_new_total_and_takes_the_rest() {
    let mut r = round();
    r["payments"] = json!([{"amount": 700}]);
    let one = vec![json!({"product_id": "maki", "quantity": 1, "unit_price": 600})];
    assert!(matches!(reprice(&mut r.clone(), one, 0), Err(Refused::Conflict(_))));
    let two = vec![json!({"product_id": "maki", "quantity": 2, "unit_price": 600})];
    reprice(&mut r, two, 100).expect("1100 covers the 700 paid");
    assert_eq!((r["subtotal"].as_i64(), r["total"].as_i64()), (Some(1200), Some(1100)));
    assert!(matches!(reprice(&mut round(), vec![], 0), Err(Refused::Conflict(_))));
}

#[test]
fn a_void_reason_is_a_closed_word_or_a_bounded_other() {
    assert_eq!(VoidReason::parse("dropped"), Some(VoidReason::Dropped));
    assert_eq!(VoidReason::parse("other: fly in soup").map(|r| r.word()), Some("other:fly in soup".into()));
    assert_eq!(VoidReason::parse("because"), None);
    assert_eq!(VoidReason::parse(&format!("other:{}", "a".repeat(OTHER_MAX_CHARS + 1))), None);
}
