//! The one parser every handler goes through (W-STRICT). Each refusal with its
//! positive twin, through `from_text` -- the function `parse` and `strict` call.

use super::from_text;
use serde::Deserialize;

/// The shape most owner writes take: the venue plus a field or two.
#[derive(Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct Strict {
    location_id: String,
    #[serde(default)]
    name: Option<String>,
}

/// A body that never declared `deny_unknown_fields` keeps ignoring extras.
#[derive(Deserialize, Debug, PartialEq)]
struct Loose {
    location_id: String,
}

#[test]
fn an_unknown_field_is_refused_and_named() {
    // THE LIVE BUG: `nmae` for `name` answered ok:true and renamed nothing.
    let e = from_text::<Strict>(r#"{"location_id":"v1","nmae":"Tuna"}"#).unwrap_err();
    assert!(e.contains("unknown field `nmae`"), "{e}");
    assert!(e.contains("`name`"), "the refusal lists what it does accept: {e}");
}

#[test]
fn the_declared_fields_pass() {
    let b: Strict = from_text(r#"{"location_id":"v1","name":"Tuna"}"#).unwrap();
    assert_eq!(b, Strict { location_id: "v1".into(), name: Some("Tuna".into()) });
    let b: Strict = from_text(r#"{"location_id":"v1"}"#).unwrap();
    assert_eq!(b.name, None, "a defaulted field may be left out");
}

#[test]
fn a_missing_field_is_named() {
    let e = from_text::<Strict>(r#"{"name":"Tuna"}"#).unwrap_err();
    assert!(e.contains("missing field `location_id`"), "{e}");
}

#[test]
fn an_empty_body_is_a_refusal_not_a_default() {
    assert!(from_text::<Strict>("").is_err());
    assert!(from_text::<Strict>("   ").is_err());
    let e = from_text::<Strict>("{").unwrap_err();
    assert!(e.contains("EOF"), "{e}");
}

#[test]
fn a_loose_struct_still_ignores_what_it_never_named() {
    let b: Loose = from_text(r#"{"location_id":"v1","anything":1}"#).unwrap();
    assert_eq!(b.location_id, "v1");
}

#[test]
fn a_nested_struct_is_held_to_its_own_attribute() {
    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    struct Inner {
        #[allow(dead_code)]
        qty: i64,
    }
    #[derive(Deserialize, Debug)]
    struct Outer {
        #[allow(dead_code)]
        lines: Vec<Inner>,
    }
    let e = from_text::<Outer>(r#"{"lines":[{"qty":1,"unit":"g"}]}"#).unwrap_err();
    assert!(e.contains("unknown field `unit`"), "{e}");
    assert!(from_text::<Outer>(r#"{"lines":[{"qty":1}],"note":"x"}"#).is_ok(), "the outer struct did not deny");
}

/// ITEM 7 (operator, 2026-10-16): the console sent `?location_id=` in the
/// query and a body without it -> `400 missing field location_id`. A body
/// without it takes the query's; both present and different is refused.
#[test]
fn a_body_without_the_venue_takes_it_from_the_query() {
    let s: Strict = super::from_text_at(r#"{"name":"Tuna"}"#, Some("v1")).unwrap();
    assert_eq!(s, Strict { location_id: "v1".into(), name: Some("Tuna".into()) });
    let e = super::from_text_at::<Strict>(r#"{"location_id":"v1","name":"Tuna"}"#, Some("v2")).err().unwrap();
    assert_eq!(e, "location_id in the query and the body differ");
    // TWINS: the same venue in both lands; no query is the old parse exactly.
    assert!(super::from_text_at::<Strict>(r#"{"location_id":"v1"}"#, Some("v1")).is_ok());
    assert!(super::from_text_at::<Strict>(r#"{"name":"Tuna"}"#, None).unwrap_err().contains("missing field `location_id`"));
    // A body that has no venue field is NOT given one: strictness holds.
    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    struct NoVenue {
        #[allow(dead_code)]
        name: String,
    }
    assert!(super::from_text_at::<NoVenue>(r#"{"name":"x"}"#, Some("v1")).is_ok());
    assert!(super::from_text_at::<NoVenue>(r#"{"name":"x","zzz":1}"#, Some("v1")).unwrap_err().contains("zzz"));
}
