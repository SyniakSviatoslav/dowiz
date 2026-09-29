//! The top-up body is money: it is read through `crate::body` and refuses a
//! field it has no name for (W-STRICT). Each refusal with its twin.

use super::TopUpBody;
use crate::body::from_text;

const GOOD: &str = r#"{"user":"cust_1","amountMinor":12345,"currency":"ALL","providerRef":"stripe_pi_1","requestId":"r1"}"#;

#[test]
fn the_kit_body_is_accepted_whole() {
    let b: TopUpBody = from_text(GOOD).expect("the five declared fields");
    assert_eq!(b.amount_minor, 12_345);
    assert_eq!(b.provider_ref, "stripe_pi_1");
}

#[test]
fn a_misspelt_money_field_is_refused_by_name() {
    // `amount_minor` (snake) for `amountMinor`: before W-STRICT this parsed as
    // "missing field" only in the tests; live it was read as 0 or dropped.
    let e = crate::body::refusal::<TopUpBody>(r#"{"user":"cust_1","amount_minor":12345,"amountMinor":1,"currency":"ALL","providerRef":"x","requestId":"r1"}"#);
    assert!(e.contains("unknown field `amount_minor`"), "{e}");
}
