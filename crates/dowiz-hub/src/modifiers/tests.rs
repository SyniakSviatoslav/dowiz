use super::*;

const DISH: &str = r#"{"id":"p1","name":"Sake","price":900,"modifierGroups":[
  {"id":"size","name":"Size","min":1,"max":1,"options":[
    {"id":"s6","name":"6 pieces","priceDelta":0},
    {"id":"s8","name":"8 pieces","priceDelta":300}]},
  {"id":"extra","name":"Extras","min":0,"max":2,"options":[
    {"id":"wasabi","name":"Extra wasabi","priceDelta":50},
    {"id":"ginger","name":"Extra ginger","priceDelta":50},
    {"id":"caviar","name":"Tobiko","priceDelta":400,"available":false}]},
  {"id":"hold","name":"Leave out","min":0,"max":0,"options":[
    {"id":"no_onion","name":"No onion","priceDelta":0},
    {"id":"no_avo","name":"No avocado","priceDelta":-50}]}]}"#;

fn g() -> Vec<Group> {
    groups_of(DISH)
}
fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn the_groups_read_back_including_the_nested_options() {
    let gs = g();
    assert_eq!(gs.len(), 3, "the walk must not stop at the first options array");
    assert_eq!(gs[0].id, "size");
    assert_eq!(gs[0].options.len(), 2);
    assert!(gs[0].required());
    assert!(!gs[1].required());
    assert_eq!(gs[1].max, 2);
    assert_eq!(gs[2].max, 0, "zero means no ceiling");
    assert_eq!(gs[0].options[1].price_delta, 300);
}

#[test]
fn a_valid_selection_prices_correctly() {
    let p = price(&g(), &ids(&["s8", "wasabi"])).expect("valid");
    assert_eq!(p.delta, 350, "300 for the size + 50 for the wasabi");
    assert_eq!(p.chosen.len(), 2);
    // The names travel, so the kitchen ticket reads in words.
    assert!(p.chosen.iter().any(|(_, n, _)| n == "8 pieces"));
}

/// A negative delta is a real offer: "no avocado, minus fifty".
#[test]
fn a_negative_delta_reduces_the_price() {
    let p = price(&g(), &ids(&["s6", "no_avo"])).expect("valid");
    assert_eq!(p.delta, -50);
}

/// A kitchen that receives a roll with no size has to guess.
#[test]
fn a_required_group_must_be_chosen_from() {
    match price(&g(), &ids(&["wasabi"])) {
        Err(ModError::Missing { group, need }) => {
            assert_eq!(group, "Size");
            assert_eq!(need, 1);
        }
        other => panic!("{other:?}"),
    }
    assert!(price(&g(), &[]).is_err());
}

/// And one with two sizes has to phone.
#[test]
fn a_group_cannot_be_over_chosen() {
    match price(&g(), &ids(&["s6", "s8"])) {
        Err(ModError::TooMany { group, max, got }) => {
            assert_eq!((group.as_str(), max, got), ("Size", 1, 2));
        }
        other => panic!("{other:?}"),
    }
    assert!(price(&g(), &ids(&["s6", "wasabi", "ginger", "no_onion"])).is_ok(),
            "two extras is the limit and a separate group does not count toward it");
}

#[test]
fn a_ceiling_of_zero_means_no_ceiling() {
    assert!(price(&g(), &ids(&["s6", "no_onion", "no_avo"])).is_ok());
}

/// An id from a different dish, or invented. Same answer either way.
#[test]
fn an_unknown_option_is_refused_before_anything_else() {
    match price(&g(), &ids(&["s6", "gold_leaf"])) {
        Err(ModError::Unknown { option }) => assert_eq!(option, "gold_leaf"),
        other => panic!("{other:?}"),
    }
    // Checked BEFORE the group rules: an unknown id must not be reported as
    // a missing size, which would send the customer looking in the wrong
    // place.
    assert!(matches!(
        price(&g(), &ids(&["gold_leaf"])),
        Err(ModError::Unknown { .. })
    ));
}

#[test]
fn an_option_the_kitchen_has_run_out_of_is_refused() {
    match price(&g(), &ids(&["s6", "caviar"])) {
        Err(ModError::Unavailable { option }) => assert_eq!(option, "Tobiko"),
        other => panic!("{other:?}"),
    }
}

/// A dish with no groups is the normal case and must price at zero rather
/// than fail.
#[test]
fn a_plain_dish_needs_no_options() {
    let plain = groups_of(r#"{"id":"p","name":"Water","price":100}"#);
    assert!(plain.is_empty());
    assert_eq!(price(&plain, &[]).expect("fine").delta, 0);
    // And naming an option it does not have is still refused.
    assert!(price(&plain, &ids(&["whatever"])).is_err());
}

#[test]
fn a_malformed_group_is_skipped_rather_than_fatal() {
    for junk in [
        r#"{"modifierGroups":"nonsense"}"#,
        r#"{"modifierGroups":[]}"#,
        r#"{"modifierGroups":[{"id":"g","options":[]}]}"#,
        r#"{"modifierGroups":[{"name":"no id","options":[{"id":"x"}]}]}"#,
        r#"{"id":"p"}"#,
    ] {
        let gs = groups_of(junk);
        assert!(gs.is_empty(), "accepted {junk}: {gs:?}");
    }
    // A group whose id appears only inside its options must NOT borrow it:
    // two such groups would share one id and the rules would apply to the
    // wrong one. This is the case the first version got wrong.
    assert!(groups_of(r#"{"modifierGroups":[{"name":"no id","options":[{"id":"x","name":"X"}]}]}"#).is_empty());

    // A group with SOME valid options survives with those.
    let gs = groups_of(r#"{"modifierGroups":[{"id":"g","name":"G","options":[
        {"id":"ok","name":"Fine","priceDelta":10},{"name":"no id"}]}]}"#);
    assert_eq!(gs.len(), 1);
    assert_eq!(gs[0].options.len(), 1);
}

/// The errors are for a customer to read, not a log to swallow.
#[test]
fn every_refusal_says_what_to_do() {
    let cases = [
        price(&g(), &ids(&["wasabi"])),
        price(&g(), &ids(&["s6", "s8"])),
        price(&g(), &ids(&["s6", "caviar"])),
        price(&g(), &ids(&["nope"])),
    ];
    for c in cases {
        let msg = c.expect_err("should refuse").to_string();
        assert!(msg.len() > 12, "too terse to act on: {msg}");
        assert!(!msg.contains("Err"), "leaking a debug shape: {msg}");
    }
}
