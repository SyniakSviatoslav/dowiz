//! The route's own rule: a body with a field the contract has no name for is refused by name.

use super::*;

#[test]
fn a_body_with_an_unknown_field_is_refused_by_name() {
    let good = r#"{"location_id":"v1","ctx":"kitchen","tail":[]}"#;
    assert!(crate::body::from_text::<Body>(good).is_ok(), "the twin");
    assert!(crate::body::refusal::<Body>(r#"{"location_id":"v1","ctx":"kitchen","tail":[],"by":"x"}"#).contains("by"), "the signer is the token's");
    assert!(crate::body::refusal::<Body>(r#"{"location_id":"v1","ctx":"kitchen","tail":[],"phone":"1"}"#).contains("phone"));
}
