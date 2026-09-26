use super::*;

#[test]
fn every_verb_round_trips_exactly() {
    let a = arg(&["r1", "42", "marg", "2"]).unwrap();
    assert_eq!(decode("add", &a), Some(json!({ "orderId": "r1", "baseSeq": 42, "productId": "marg", "quantity": 2 })));
    let p = place("5", &[("cola".into(), 2), ("tira".into(), 1)]).unwrap();
    assert_eq!(
        decode("place", &p),
        Some(json!({ "table": "5", "items": [{ "product_id": "cola", "quantity": 2 }, { "product_id": "tira", "quantity": 1 }] }))
    );
    let pay = arg(&["r2", "9", "1500", "cash"]).unwrap();
    assert_eq!(decode("pay", &pay), Some(json!({ "orderId": "r2", "baseSeq": 9, "amount": 1500, "method": "cash" })));
    assert_eq!(decode("dish_off", "marg"), Some(json!({ "productId": "marg" })));
    assert_eq!(decode("dish_on", "marg"), Some(json!({ "productId": "marg" })));
    assert_eq!(decode("venue", "busy"), Some(json!({ "state": "busy" })));
}

/// A field that would break the framing is refused, not escaped.
#[test]
fn a_field_holding_a_separator_is_refused() {
    assert_eq!(arg(&["r1", "4|2"]), None);
    assert_eq!(arg(&["", "x"]), None);
    assert_eq!(arg(&[&"x".repeat(65)]), None);
    assert_eq!(items(&[("co,la".into(), 1)]), None);
    assert_eq!(items(&[("co*la".into(), 1)]), None);
    assert!(clean("T5") && clean("стіл 5"));
    assert_eq!(place("5|6", &[("cola".into(), 1)]), None);
    assert_eq!(place("5", &[]), None);
}

/// A malformed arg decodes to nothing -- never to half an instruction.
#[test]
fn a_malformed_arg_decodes_to_nothing() {
    assert_eq!(decode("add", "r1|x|marg|2"), None);
    assert_eq!(decode("add", "r1|1|marg"), None);
    assert_eq!(decode("place", "5|cola"), None);
    assert_eq!(decode("place", "5|cola*two"), None);
    assert_eq!(decode("pay", "r1|1|lots|cash"), None);
    // The hub's own verbs are not this layer's to decode.
    assert_eq!(decode("confirm", "order-1"), None);
}

#[test]
fn only_the_new_verbs_are_ours() {
    for v in ["add", "place", "pay", "dish_off", "dish_on", "venue"] {
        assert!(is_ours(v), "{v}");
    }
    for v in ["confirm", "ready", "reject", "pickup", "deliver", "shift_open"] {
        assert!(!is_ours(v), "{v}");
    }
}
