//! The currency rule. The Worker's `command/pay/fx/tests.rs` holds the full
//! table; these pin the worked example and one refusal with its twin.

use super::*;

#[test]
fn twenty_euro_at_97_50_is_1950_lek() {
    let s = settle("ALL", Some("EUR"), Some(975_000), 2000).unwrap();
    assert_eq!(s, Settled { currency: Currency::Eur, rate_ppm: Some(975_000), in_order_currency: 1950 });
}

#[test]
fn a_foreign_payment_states_its_rate_and_a_local_one_does_not() {
    assert!(matches!(settle("ALL", Some("EUR"), None, 2000), Err(Refused::Invalid(_))));
    assert!(matches!(settle("ALL", None, Some(RATE_SCALE), 2000), Err(Refused::Invalid(_))));
    assert_eq!(settle("ALL", None, None, 2000).unwrap().in_order_currency, 2000);
    assert_eq!(convert(1, 1), Ok(0), "half-up with an integer half: 1e-6 rounds to zero");
}
