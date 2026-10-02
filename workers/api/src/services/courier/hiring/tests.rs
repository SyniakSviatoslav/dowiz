//! W-AUDIT S8 (2026-09-27): an invitation replaces THIS venue's pending one
//! and nobody else's, through the real table turn.

use super::invite_turn;
use crate::identity_store as ids;
use dowiz_hub::table::Table;
use serde_json::json;

fn seeded(loc: &str) -> Table {
    let mut t = Table::create(1 << 20).unwrap();
    let rec = json!({"id": "inv_a", "location_id": loc, "role": "courier", "invited_phone_hash": "ph", "revoked_at_ms": null}).to_string();
    t.put(
        ids::K_INVITE,
        "inv_a",
        &rec,
        &[(ids::invite_by_phone("ph"), "inv_a".to_string()), (ids::invite_at(loc, "inv_a"), "inv_a".to_string())],
        &[],
    )
    .unwrap();
    t
}

#[test]
fn another_venues_pending_invite_is_left_alone_and_the_phone_reads_as_taken() {
    let mut t = seeded("venue-a");
    assert!(invite_turn(&mut t, "ph", "venue-b", "own", "nm", "ch", "inv_b", 5).unwrap(), "refused as taken");
    let a = ids::rec(&t, ids::K_INVITE, "inv_a").unwrap();
    assert!(a["revoked_at_ms"].is_null(), "venue A's invitation stands: {a}");
    assert!(ids::rec(&t, ids::K_INVITE, "inv_b").is_none(), "nothing was written for B");
    assert_eq!(t.lookup(&ids::invite_by_phone("ph")).as_deref(), Some("inv_a"));
}

/// The twin: the same venue inviting the same phone again replaces its own code.
#[test]
fn the_same_venue_replaces_its_own_pending_invite() {
    let mut t = seeded("venue-a");
    assert!(!invite_turn(&mut t, "ph", "venue-a", "own", "nm", "ch", "inv_a2", 5).unwrap());
    assert_eq!(ids::rec(&t, ids::K_INVITE, "inv_a").unwrap()["revoked_at_ms"], json!(5));
    assert_eq!(t.lookup(&ids::invite_by_phone("ph")).as_deref(), Some("inv_a2"));
    assert_eq!(ids::s_of(&ids::rec(&t, ids::K_INVITE, "inv_a2").unwrap(), "location_id"), "venue-a");
}

/// THE LINK (operator 2026-10-02): the invite answers a URL on the venue's own
/// host whose fragment carries the code, and the claim route on that host takes
/// the code. Before this the answer was `{code, expiresMs}` and went nowhere.
#[test]
fn an_invite_answers_a_link_on_the_venues_host_that_the_claim_route_accepts() {
    use crate::edge::site::{post, As, Site, PLATFORM_HOST, T0};
    use crate::services::courier::invite_link;
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    // Asked on the APEX, where the Host names no venue: the link still names alpha.
    let r = site.run(
        super::invite_courier,
        post(&format!("https://{PLATFORM_HOST}/api/owner/couriers/invite"), &json!({"phone": "+355691230000", "name": "Rider"}))
            .bearer(&owner)
            .with_header("host", PLATFORM_HOST),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    let url = v["url"].as_str().unwrap_or_else(|| panic!("an invite carries a link: {v}")).to_string();
    assert!(url.starts_with(&format!("https://alpha.{PLATFORM_HOST}/courier/")), "the venue's own host: {url}");
    assert_eq!(v["expiresMs"], json!(T0 + super::TTL_MS), "a week, shown to the owner");
    let code = v["code"].as_str().unwrap();
    assert_eq!(invite_link::code_in(&url), Some(code), "{url}");
    assert!(!url.split_once('#').unwrap().0.contains(code), "the code is in the fragment, not in a path or a query");
    // The courier opens the link on alpha's host and the code is accepted there.
    let c = site.run(
        crate::accounts::courier_claim,
        post(&format!("https://alpha.{PLATFORM_HOST}/api/courier/auth/claim"), &json!({"phone": "+355691230000", "code": code, "password": "courier-password-1"}))
            .on("alpha"),
        &[],
    );
    assert_eq!(c.status_code(), 200, "{}", c.body_str());
    assert!(c.body_value()["jwt"].as_str().is_some(), "signed in on the spot: {}", c.body_str());
    // Once. The same link again is a spent code.
    let again = site.run(
        crate::accounts::courier_claim,
        post(&format!("https://alpha.{PLATFORM_HOST}/api/courier/auth/claim"), &json!({"phone": "+355691230000", "code": code, "password": "courier-password-2"}))
            .on("alpha"),
        &[],
    );
    assert_eq!(again.status_code(), 400, "{}", again.body_str());
}
