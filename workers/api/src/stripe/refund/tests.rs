//! W-REFUND: the units that cross to Stripe, the form, and how an answer is read.

use super::*;

fn job(amount: i64, currency: &str) -> Job {
    Job { venue: "v1".into(), order_id: "o1".into(), pi: "pi_1".into(), amount, currency: currency.into(), n: 1, key: "dowiz-rf-k".into() }
}

/// A LEK IS A HUNDRED OF STRIPE'S UNITS: 1500 lek goes out as 150000 and
/// comes back as 1500. Euros and dollars are already in cents on both sides.
#[test]
fn lek_crosses_to_stripe_times_a_hundred_and_cents_unchanged() {
    assert_eq!(to_stripe(1500, "ALL"), Some(150_000));
    assert_eq!(from_stripe(150_000, "all"), Some(1500), "Stripe's lowercase code");
    assert_eq!(to_stripe(981, "EUR"), Some(981));
    assert_eq!(from_stripe(981, "eur"), Some(981));
}

/// Refusals, each beside the twin above: an unknown currency, a fraction of
/// a lek, an overflow are `None`, never a guessed number.
#[test]
fn an_unknown_currency_a_fraction_of_a_lek_or_an_overflow_is_refused() {
    assert_eq!(to_stripe(100, "GBP"), None);
    assert_eq!(from_stripe(150_050, "ALL"), None);
    assert_eq!(to_stripe(i64::MAX / 10, "ALL"), None);
}

#[test]
fn the_form_carries_the_converted_amount_the_intent_and_the_key() {
    let f = form(&job(750, "ALL")).unwrap();
    assert!(f.starts_with("payment_intent=pi_1&amount=75000&"), "{f}");
    assert!(f.contains("metadata[venue]=v1") && f.contains("metadata[key]=dowiz-rf-k") && f.contains("metadata[attempt]=1"), "{f}");
    assert!(form(&job(0, "ALL")).is_err(), "a zero refund");
    assert!(form(&job(10, "GBP")).is_err(), "a currency dowiz does not price in");
}

#[test]
fn the_entry_is_keyed_by_the_idempotency_key_and_carries_the_job() {
    let e = entry(&job(750, "ALL"), 5);
    assert_eq!((e.id.as_str(), e.kind.as_str(), e.to.as_str()), ("dowiz-rf-k", KIND, "o1"));
    assert_eq!(serde_json::from_str::<Job>(&e.text).unwrap(), job(750, "ALL"));
}

#[test]
fn an_answer_is_made_final_or_transient() {
    let ok = r#"{"id":"re_1","object":"refund","status":"succeeded","amount":75000}"#;
    assert_eq!(classify(200, ok), Sent::Made { id: "re_1".into(), status: "succeeded".into(), failure: None });
    let over = r#"{"error":{"message":"Refund amount (ALL 2000.00) is greater than unrefunded amount on charge (ALL 1500.00)"}}"#;
    assert!(matches!(classify(400, over), Sent::Final(m) if m.contains("greater than unrefunded")));
    for s in [409, 429, 500, 503] {
        assert!(matches!(classify(s, "{}"), Sent::Transient(_)), "{s}");
    }
    assert!(matches!(classify(200, "{}"), Sent::Transient(_)), "a 200 with no refund id is not a refund");
}
