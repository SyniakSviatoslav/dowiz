//! One site, several addresses: the alias list as the Worker reads it.
use super::*;

const SPEC: &str = " Sushi-Durres = dubin-sushi , broken, =x, y= ";

#[test]
fn an_alias_opens_its_target_and_every_other_slug_opens_itself() {
    assert_eq!(pairs(SPEC), vec![("sushi-durres".to_string(), "dubin-sushi".to_string())], "malformed items are skipped");
    assert_eq!(target(SPEC, "sushi-durres"), "dubin-sushi");
    assert_eq!(target(SPEC, "SUSHI-DURRES"), "dubin-sushi", "a host is case-free");
    assert_eq!(target(SPEC, "dubin-sushi"), "dubin-sushi");
    assert_eq!(target(SPEC, "qa-durres"), "qa-durres");
    assert_eq!(target("", "sushi-durres"), "sushi-durres", "no spec, no alias");
}

#[test]
fn the_venue_an_alias_replaced_is_retired_and_its_target_is_not() {
    assert!(retired(SPEC, "sushi-durres"));
    assert!(!retired(SPEC, "dubin-durres"));
    assert!(!retired(SPEC, "dubin-sushi"));
    assert!(!retired(SPEC, "qa-durres"));
    assert!(!retired("", "sushi-durres"));
}

#[test]
fn an_alias_to_itself_or_a_chain_is_not_followed() {
    assert_eq!(target("a=a", "a"), "a");
    assert!(!retired("a=a", "a"), "a venue cannot retire itself");
    // One hop only: a=b, b=c sends a to b, never on to c.
    assert_eq!(target("a=b,b=c", "a"), "b");
}
