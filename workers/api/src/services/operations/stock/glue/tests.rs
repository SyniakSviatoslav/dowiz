//! The route's own rules: a tablet's name, and a body with a field the contract has no name for.

use super::*;

#[test]
fn a_tablets_name_is_short_plain_and_never_the_hubs() {
    for ok in ["kitchen", "waiter-2", "bar_1"] {
        assert!(ctx_ok(ok), "{ok}");
    }
    for bad in ["", "hub", "kitchen tablet", "k\"x", &"k".repeat(33)] {
        assert!(!ctx_ok(bad), "{bad:?}");
    }
}

#[test]
fn a_body_with_an_unknown_field_is_refused_by_name() {
    let good = r#"{"location_id":"v1","ctx":"kitchen","base":"00","tip":"00","tail":[]}"#;
    assert!(crate::body::from_text::<GlueBody>(good).is_ok(), "the twin");
    let bad = r#"{"location_id":"v1","ctx":"kitchen","base":"00","tip":"00","tail":[],"phone":"1"}"#;
    assert!(crate::body::refusal::<GlueBody>(bad).contains("phone"));
    assert!(crate::body::refusal::<CloseBody>(r#"{"location_id":"v1","did":"a","by":"x"}"#).contains("by"), "the signer is the token's, never the body's");
}
