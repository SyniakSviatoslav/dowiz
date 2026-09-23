//! The rate, its unit and its rounding; every refusal beside its twin.

use super::*;

#[test]
fn the_worked_example_in_the_doc_comment_holds() {
    // €20.00 at 1 EUR = 97.50 ALL: 2000 cents × 975 000 ppm = 1950 lek.
    assert_eq!(convert(2000, 975_000), Ok(1950));
    // The other way: 1950 lek at 1 025 641 ppm is 2000 cents (1999.99995 → half-up).
    assert_eq!(convert(1950, 1_025_641), Ok(2000));
}

/// THE KERNEL'S ROUNDING: exactly half goes up, just under half goes down.
#[test]
fn half_up_with_an_integer_half() {
    assert_eq!(convert(1, 500_000), Ok(1), "0.5 → 1");
    assert_eq!(convert(1, 499_999), Ok(0), "0.499999 → 0");
    assert_eq!(convert(3, 500_000), Ok(2), "1.5 → 2");
}

#[test]
fn a_rate_below_one_is_invalid_and_one_is_the_smallest_rate() {
    assert!(matches!(convert(100, 0), Err(Refused::Invalid(_))));
    assert!(matches!(convert(100, -975_000), Err(Refused::Invalid(_))));
    assert_eq!(convert(1_000_000, 1), Ok(1));
}

#[test]
fn an_overflowing_amount_is_invalid_not_wrapped() {
    assert!(matches!(convert(i64::MAX, i64::MAX), Err(Refused::Invalid(_))));
    assert_eq!(convert(i64::MAX / 2, RATE_SCALE), Ok(i64::MAX / 2));
}

#[test]
fn no_currency_named_is_the_orders_and_needs_no_rate() {
    let s = settle("ALL", None, None, 1500).unwrap();
    assert_eq!(s, Settled { currency: Currency::All, rate_ppm: None, in_order_currency: 1500 });
}

#[test]
fn a_rate_on_a_same_currency_payment_is_invalid_and_without_it_lands() {
    assert!(matches!(settle("ALL", Some("ALL"), Some(975_000), 1500), Err(Refused::Invalid(_))));
    assert!(settle("ALL", Some("ALL"), None, 1500).is_ok());
}

#[test]
fn a_foreign_payment_without_a_rate_is_invalid_and_with_one_converts() {
    assert!(matches!(settle("ALL", Some("EUR"), None, 2000), Err(Refused::Invalid(_))));
    let s = settle("ALL", Some("EUR"), Some(975_000), 2000).unwrap();
    assert_eq!(s, Settled { currency: Currency::Eur, rate_ppm: Some(975_000), in_order_currency: 1950 });
}

#[test]
fn an_unknown_currency_is_invalid_and_every_known_one_is_taken() {
    assert!(matches!(settle("ALL", Some("GBP"), Some(1), 1), Err(Refused::Invalid(_))));
    assert!(matches!(settle("XYZ", None, None, 1), Err(Refused::Invalid(_))));
    for c in Currency::EVERY {
        let rate = (c != Currency::All).then_some(RATE_SCALE);
        assert!(settle("ALL", Some(c.code()), rate, 10).is_ok(), "{}", c.code());
    }
}

#[test]
fn a_payment_worth_less_than_one_minor_unit_is_invalid() {
    assert!(matches!(settle("EUR", Some("ALL"), Some(400_000), 1), Err(Refused::Invalid(_))));
    assert_eq!(settle("EUR", Some("ALL"), Some(400_000), 3).map(|s| s.in_order_currency), Ok(1));
}
