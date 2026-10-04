use super::*;

#[test]
fn national_spellings_are_completed_from_the_orders_currency() {
    assert_eq!(e164("069 123 4567", "ALL").as_deref(), Some("+355691234567"));
    assert_eq!(e164("067 123 45 67", "UAH").as_deref(), Some("+380671234567"));
    assert_eq!(e164("(069) 123-4567", "ALL").as_deref(), Some("+355691234567"));
}

#[test]
fn international_spellings_pass_through() {
    assert_eq!(e164("+355 69 123 4567", "EUR").as_deref(), Some("+355691234567"));
    assert_eq!(e164("00355691234567", "EUR").as_deref(), Some("+355691234567"));
    assert_eq!(e164("355691234567", "EUR").as_deref(), Some("+355691234567"));
    assert_eq!(e164("380671234567", "EUR").as_deref(), Some("+380671234567"));
}

#[test]
fn a_guess_is_refused() {
    assert_eq!(e164("069 123 4567", "EUR"), None, "national, and no country to complete it with");
    assert_eq!(e164("", "ALL"), None);
    assert_eq!(e164("+12", "ALL"), None, "too short");
    assert_eq!(e164("+1234567890123456", "ALL"), None, "too long");
    assert_eq!(e164("call me", "ALL"), None);
    assert_eq!(e164("+355 69 abc", "ALL"), None);
    assert_eq!(e164("12345678", "ALL"), None, "no prefix we can read");
}

#[test]
fn a_stop_covers_every_number_the_typing_can_mean() {
    assert_eq!(candidates("+355 69 123 4567"), vec!["+355691234567"]);
    assert_eq!(candidates("069 123 4567"), vec!["+355691234567", "+380691234567"]);
    assert!(candidates("hello").is_empty());
}
