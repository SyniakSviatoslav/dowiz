//! The owner's door to the object's publish record (W-PUBUI), through the route seam (W-COV
//! C2): a platform in memory, a venue made by `create_hub`, its owner signed in by
//! `owner_login` -- and the callers who must be refused.

use crate::edge::site::{get, As, Site, PLATFORM_HOST};
use crate::wire::Call;
use worker::Method;

fn url(slug: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}/api/owner/publish")
}

fn post(url: &str) -> Call {
    Call::new(url, Method::Post).expect("url")
}

/// THE ROUTE ANSWERS THE OBJECT'S OWN RECORD: what the Worker hands the console is what
/// `GET /fold/publish` says, field for field -- `enabled`, the manifest's key, the record.
#[test]
fn the_owner_reads_the_objects_publish_record() {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    let r = site.run(super::status, get(&url("alpha")).bearer(&a).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    let own = site.fold("alpha", "/fold/publish").body_value();
    assert_eq!(v, own, "the Worker relays the object's record unchanged");
    assert_eq!(v["manifest"], "v/alpha/manifest.json");
    assert!(v["published"]["generation"].is_array(), "{v}");
    assert!(v["published"]["objects"].is_object(), "{v}");
    assert!(v["published"]["media"].is_array(), "{v}");
}

/// IN MEMORY NOTHING HOLDS A CDN BINDING (`sink()` answers None), which is production's state
/// until the bucket is attached: the record says `enabled: false`, and "publish now" is the
/// object's 503, relayed with the object's words -- the sentence the card shows the owner.
#[test]
fn without_a_cdn_binding_publishing_is_off_and_publish_now_is_503() {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    let r = site.run(super::status, get(&url("alpha")).bearer(&a).on("alpha"), &[]);
    assert_eq!(r.body_value()["enabled"], false, "{}", r.body_str());
    let r = site.run(super::publish_now, post(&url("alpha")).bearer(&a).on("alpha"), &[]);
    assert_eq!(r.status_code(), 503, "{}", r.body_str());
    assert!(r.body_str().contains("publishing is off"), "{}", r.body_str());
    // `?all=1` reaches the object too, and is the same refusal while nothing can be written.
    let r = site.run(super::publish_now, post(&format!("{}?all=1", url("alpha"))).bearer(&a).on("alpha"), &[]);
    assert_eq!(r.status_code(), 503, "{}", r.body_str());
}

/// A NON-OWNER IS REFUSED: a member of staff holds a venue token and is not the owner (403 on
/// both methods); no token at all is 401. Nothing of the record crosses in either case.
#[test]
fn a_member_of_staff_and_the_anonymous_are_refused() {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    let staff = site.staff("alpha", &a, "k@x.test", "kitchen");
    let r = site.run(super::status, get(&url("alpha")).bearer(&staff).on("alpha"), &[]);
    assert_eq!(r.status_code(), 403, "{}", r.body_str());
    assert!(!r.body_str().contains("published"), "{}", r.body_str());
    let r = site.run(super::publish_now, post(&url("alpha")).bearer(&staff).on("alpha"), &[]);
    assert_eq!(r.status_code(), 403, "{}", r.body_str());
    let anon = site.run(super::status, get(&url("alpha")).on("alpha"), &[]);
    assert_eq!(anon.status_code(), 401, "{}", anon.body_str());
}

/// THE TENANT BOUNDARY: beta's owner naming alpha is refused -- 404, never 403, because a 403
/// would confirm alpha exists (`owner_and_venue`). And beta's owner without naming anyone reads
/// BETA's record (the venue they are authorised for), never alpha's.
#[test]
fn another_venues_owner_is_refused_or_reads_their_own_venue() {
    let site = Site::new();
    let _a = site.venue("alpha", "a@x.test");
    let b = site.venue("beta", "b@x.test");
    let named = format!("{}?location_id=alpha", url("alpha"));
    let r = site.run(super::status, get(&named).bearer(&b).on("alpha"), &[]);
    assert_eq!(r.status_code(), 404, "{}", r.body_str());
    let r = site.run(super::status, get(&url("alpha")).bearer(&b).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["manifest"], "v/beta/manifest.json", "the token's venue, not the host's");
}
