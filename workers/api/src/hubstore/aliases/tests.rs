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

/// Through a real handler: the public menu at the alias's address and path is the
/// target's menu. The handler passes the path's slug on to the venue's object,
/// which answered 404 to the alias while only `Place::of_slug` knew it.
#[test]
fn the_menu_at_an_alias_is_the_targets_menu() {
    use crate::edge::site::{get, As, Site, PLATFORM_HOST};
    let menu = |site: &Site, slug: &str| {
        let url = format!("https://{slug}.{PLATFORM_HOST}/api/public/locations/{slug}/menu?fresh=1");
        site.run(crate::storefront::menu, get(&url).on(slug), &[("slug", slug)])
    };
    let site = Site::with_env(&[], &[("SLUG_ALIASES", "old=alpha")]);
    site.venue("alpha", "a@x.test");
    let own = menu(&site, "alpha");
    assert_eq!(own.status_code(), 200, "{}", own.body_str());
    let via = menu(&site, "old");
    assert_eq!(via.status_code(), 200, "{}", via.body_str());
    assert_eq!(via.body_str(), own.body_str(), "one site: the same menu at both addresses");
    // TWIN: with no alias the same slug names no venue.
    let bare = Site::new();
    bare.venue("alpha", "a@x.test");
    assert_ne!(menu(&bare, "old").status_code(), 200);
}

#[test]
fn an_alias_to_itself_or_a_chain_is_not_followed() {
    assert_eq!(target("a=a", "a"), "a");
    assert!(!retired("a=a", "a"), "a venue cannot retire itself");
    // One hop only: a=b, b=c sends a to b, never on to c.
    assert_eq!(target("a=b,b=c", "a"), "b");
}
