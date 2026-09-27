//! W-FIX H1: the courier's handover, through the real functions.

use super::{add, handover, said, Handover};

#[test]
fn an_unreadable_body_is_refused_not_read_as_everything_collected() {
    for raw in [r#"{"cash_collected":"0"}"#, r#"{"cash_collected":1200.0}"#, r#"{"cash_collected":"#, "zero"] {
        assert!(said(raw).is_err(), "{raw} must be a 400, not the full amount");
    }
}

/// The twin: an absent body or an absent field is "nothing said", and a number is that number.
#[test]
fn an_absent_amount_is_none_and_a_number_is_itself() {
    assert_eq!(said(""), Ok(None));
    assert_eq!(said("  "), Ok(None));
    assert_eq!(said("{}"), Ok(None));
    assert_eq!(said(r#"{"cash_collected":null}"#), Ok(None));
    assert_eq!(said(r#"{"cash_collected":700}"#), Ok(Some(700)));
}

#[test]
fn collected_is_bounded_by_zero_and_by_what_the_delivery_owes() {
    assert!(handover(1200, Some(-1)).is_err(), "negative");
    assert!(handover(1200, Some(1201)).is_err(), "more than owed");
    assert!(handover(1200, Some(99_999_999)).is_err(), "far more than owed");
    assert!(handover(-5, None).is_err(), "a damaged cash_due is not a negative handover");
}

/// The twin: every amount inside the range is recorded exactly, short and all.
#[test]
fn a_short_handover_is_recorded_and_nothing_said_is_the_full_amount() {
    assert_eq!(handover(1200, Some(700)), Ok(Handover { collected: 700, short: 500 }));
    assert_eq!(handover(1200, Some(0)), Ok(Handover { collected: 0, short: 1200 }));
    assert_eq!(handover(1200, Some(1200)), Ok(Handover { collected: 1200, short: 0 }));
    assert_eq!(handover(1200, None), Ok(Handover { collected: 1200, short: 0 }));
    assert_eq!(handover(0, None), Ok(Handover { collected: 0, short: 0 }));
}

#[test]
fn a_shift_total_that_would_overflow_is_refused() {
    assert_eq!(add(i64::MAX, 1), None);
    assert_eq!(add(i64::MAX - 10, 11), None);
}

/// The twin: an ordinary total moves by the amount.
#[test]
fn a_shift_total_moves_by_the_amount() {
    assert_eq!(add(3000, 1200), Some(4200));
    assert_eq!(add(i64::MAX - 1, 1), Some(i64::MAX));
}
