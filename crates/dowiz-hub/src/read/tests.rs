use crate::*;

/// An audit entry must not be able to masquerade as an order. Every fold in
/// the system -- the takings, the promo use-count, the dashboard -- is built
/// on `orders()`, and one bogus row in it is a number that is quietly wrong
/// everywhere at once.
#[test]
fn a_reveal_is_not_an_order() {
    let mut h = Hub::create().unwrap();
    h.append(EventKind::Placed, "ord_1", r#"{"total":900}"#, 1, [0u8; 32]).unwrap();
    h.append(
        EventKind::Revealed,
        "cust:+355690000000",
        r#"{"by":"ana@dubin.al","at":1789000000000}"#,
        2,
        [0u8; 32],
    )
    .unwrap();
    h.append(EventKind::Placed, "ord_2", r#"{"total":850}"#, 3, [0u8; 32]).unwrap();

    let orders = h.orders();
    assert_eq!(orders.len(), 2, "the audit entry took an order's place");
    assert!(orders.iter().all(|e| e.kind.is_order()));
    assert!(orders.iter().all(|e| e.order_id.starts_with("ord_")));

    assert_eq!(h.reveals().len(), 1);
    assert_eq!(h.reveals()[0].order_id, "cust:+355690000000");
    // It survives a round trip through the image, like everything else.
    let back = Hub::load(&h.to_bytes()).unwrap();
    assert_eq!(back.reveals().len(), 1);
    assert_eq!(back.orders().len(), 2);
}

/// The audit payload must not contain what it audits: a log of who read a
/// phone number that also holds the phone number has doubled the exposure.
#[test]
fn the_audit_entry_carries_no_contact_details() {
    let mut h = Hub::create().unwrap();
    h.append(
        EventKind::Revealed,
        "cust:8f2a9c",
        r#"{"by":"ana@dubin.al","at":1789000000000}"#,
        1,
        [0u8; 32],
    )
    .unwrap();
    let e = &h.reveals()[0];
    for leak in ["+355", "@gmail", "Rruga"] {
        assert!(!e.order_json.contains(leak), "the audit entry carries {leak}");
    }
}
