//! The normaliser, against the spellings a Durrës customer actually types, and
//! each refusal beside the spelling that is accepted.

use super::*;
use crate::services::customers::handlers::customer_key;

const E164: &str = "355691234567";
const SECRET: &[u8] = b"test-secret-key";

#[test]
fn the_four_spellings_of_one_albanian_mobile_are_one_number() {
    for s in ["+355 69 123 4567", "069 123 4567", "00355691234567", "355691234567"] {
        assert_eq!(canonical_digits(s, "355").as_deref(), Some(E164), "spelling {s:?}");
    }
}

#[test]
fn punctuation_a_person_types_does_not_matter() {
    for s in ["+355-69-123-4567", "+355 (69) 123 4567", "069.123.4567", "69 123 4567", " 069 123 4567\n"] {
        assert_eq!(canonical_digits(s, "+355").as_deref(), Some(E164), "spelling {s:?}");
    }
}

/// REFUSAL: letters, an empty box, a bare country code. TWIN: the same digits
/// written as a phone are accepted.
#[test]
fn garbage_is_none_and_its_twin_is_a_number() {
    for s in ["garbage", "", "+355", "069 123 456x", "069 12"] {
        assert_eq!(canonical_digits(s, "355"), None, "garbage {s:?}");
    }
    assert_eq!(canonical_digits("069 123 4567", "355").as_deref(), Some(E164));
}

/// REFUSAL: a Kosovan number written internationally is not an Albanian one.
/// TWIN: the Albanian number written the same way is.
#[test]
fn another_countrys_number_is_not_rewritten() {
    assert_eq!(canonical_digits("+383 44 123 456", "355"), None);
    assert_eq!(canonical_digits("00383 44 123 456", "355"), None);
    assert_eq!(canonical_digits("+355 69 123 4567", "355").as_deref(), Some(E164));
}

/// REFUSAL: a mobile one digit short or long. TWIN: a fixed Tirana line,
/// which is 8 digits, is a number.
#[test]
fn a_number_of_the_wrong_length_is_refused_and_a_fixed_line_is_not() {
    assert_eq!(canonical_digits("+355 69 123 456", "355"), None);
    assert_eq!(canonical_digits("+355 69 123 45678", "355"), None);
    assert_eq!(canonical_digits("04 223 4567", "355").as_deref(), Some("35542234567"));
}

/// REFUSAL: a venue in a country the normaliser does not know. TWIN: `AL`
/// is the same country as `355`.
#[test]
fn an_unsupported_country_answers_none() {
    assert_eq!(canonical_digits("069 123 4567", "1"), None);
    assert_eq!(canonical_digits("069 123 4567", "AL").as_deref(), Some(E164));
}

/// The placement's pair: a national spelling links to the E.164 key; the
/// E.164 spelling (and the `00` one, which `customer_key` already folds) has
/// nothing to link.
#[test]
fn the_rule_pair_names_the_national_key_and_the_e164_key() {
    let (from, to) = rule_pair(SECRET, "069 123 4567", "355").expect("national spelling links");
    assert_eq!(from, customer_key(SECRET, "069 123 4567"));
    assert_eq!(to, customer_key(SECRET, "+355 69 123 4567"));
    assert_ne!(from, to);
    assert_eq!(rule_pair(SECRET, "+355 69 123 4567", "355"), None);
    assert_eq!(rule_pair(SECRET, "00355691234567", "355"), None);
    assert_eq!(rule_pair(SECRET, "garbage", "355"), None);
}

/// P5 (audit D38): every spelling of one number is ONE key -- the booking's,
/// the wallet's and the alias target's all come from `person_key` -- and the
/// key of the spelling as typed resolves to it through the rule's alias.
#[test]
fn every_spelling_is_one_person_key_in_every_module() {
    let spellings = ["+355 69 123 4567", "069 123 4567", "00355691234567", "355691234567", "69 123 4567", "069-123-4567"];
    let want = customer_key(SECRET, E164);
    let mut cases = 0;
    for s in spellings {
        assert_eq!(person_key(SECRET, s), want, "person_key of {s:?}");
        // The spelling's own key, where it differs, is linked TO the person key.
        match rule_pair(SECRET, s, VENUE_DIAL) {
            Some((from, to)) => assert_eq!((from, to), (customer_key(SECRET, s), want.clone())),
            None => assert_eq!(customer_key(SECRET, s), want, "no alias only when already canonical"),
        }
        cases += 1;
    }
    assert_eq!(cases, spellings.len(), "every spelling ran");
    // TWIN: another country's number is its own person, as typed.
    assert_eq!(person_key(SECRET, "+383 44 123 456"), customer_key(SECRET, "+383 44 123 456"));
    assert_ne!(person_key(SECRET, "+383 44 123 456"), want);
}

/// P5, the wallet's half: `wallet::own_wallet_key` needs a live order and an
/// Env, so this reads its body. It must take the person key, never the key of
/// the spelling as typed -- which gave `069 …` and `+355 69 …` two wallets.
#[test]
fn the_wallet_takes_the_person_key() {
    let src = include_str!("../../../wallet.rs");
    let body = src.split("async fn own_wallet_key").nth(1).expect("the function exists");
    let body = body.split("\n}\n").next().unwrap();
    assert!(body.contains("identity::person_key("), "{body}");
    assert!(!body.contains("customer_key("), "the typed spelling's key is back: {body}");
}
