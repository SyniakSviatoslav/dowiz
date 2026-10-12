//! The memo's window, at its edges, with no `Env` (the pure half of W-AUTHC).

use super::*;
use crate::auth::{Cap, Caps};

const T: i64 = 1_700_000_000_000;

fn courier() -> Principal {
    Principal::Courier { courier_id: "c1".into(), active_location_id: "v".into(), session_id: "s1".into() }
}

fn staff() -> Principal {
    Principal::Staff {
        person_id: "p1".into(),
        active_location_id: "v".into(),
        session_id: "s2".into(),
        caps: Caps::of(&[Cap::Advance]),
    }
}

fn owner() -> Principal {
    Principal::Owner { user_id: "u1".into(), active_location_id: Some("v".into()) }
}

fn is_courier(p: Option<Principal>) -> bool {
    matches!(p, Some(Principal::Courier { ref courier_id, .. }) if courier_id == "c1")
}

/// Inside the window the verdict is reused; at the window's edge it is not.
#[test]
fn a_verdict_is_reused_inside_the_window_and_not_at_its_edge() {
    let mut m = Memo::default();
    let e = m.begin();
    m.store("k".into(), &courier(), e, T, T + 10 * TTL_MS);
    assert!(is_courier(m.get("k", T)));
    assert!(is_courier(m.get("k", T + TTL_MS - 1)), "one ms before the window closes it still answers");
    assert!(m.get("k", T + TTL_MS).is_none(), "at the window's edge the live rows must be read again");
    assert!(m.get("k", T).is_none(), "an expired entry is gone, not revived by an earlier clock");
}

/// The session row's own expiry is never extended by the memo.
#[test]
fn a_verdict_never_outlives_the_session_it_was_read_from() {
    let mut m = Memo::default();
    let e = m.begin();
    m.store("k".into(), &staff(), e, T, T + 5);
    assert!(m.get("k", T + 4).is_some());
    assert!(m.get("k", T + 5).is_none(), "the session expired at T+5; the memo must not say otherwise");
}

/// A clock that runs backwards is a miss.
#[test]
fn an_earlier_clock_is_a_miss() {
    let mut m = Memo::default();
    let e = m.begin();
    m.store("k".into(), &courier(), e, T, T + TTL_MS * 10);
    assert!(m.get("k", T - 1).is_none());
}

/// OWNERS ARE NEVER REMEMBERED (W-CHEAP: an owner removed a moment ago is refused).
#[test]
fn an_owner_is_never_memoised() {
    let mut m = Memo::default();
    let e = m.begin();
    m.store("k".into(), &owner(), e, T, T + TTL_MS);
    assert!(m.get("k", T).is_none());
    assert_eq!(m.len(), 0);
    // THE TWIN: both session-bound roles are.
    assert!(memoisable(&courier()) && memoisable(&staff()) && !memoisable(&owner()));
}

/// A platform write empties the memo, and a verdict whose reads began before that write is
/// not stored afterwards (the read may predate the revocation).
#[test]
fn a_write_empties_the_memo_and_refuses_a_verdict_read_before_it() {
    let mut m = Memo::default();
    let e = m.begin();
    m.store("k".into(), &courier(), e, T, T + TTL_MS);
    let in_flight = m.begin();
    m.forget_all();
    assert!(m.get("k", T + 1).is_none(), "a write in this isolate must end the reuse at once");
    m.store("k".into(), &courier(), in_flight, T + 1, T + TTL_MS);
    assert!(m.get("k", T + 2).is_none(), "a verdict read before the write must not be put back");
    // THE TWIN: a verdict read after the write is.
    let now = m.begin();
    m.store("k".into(), &courier(), now, T + 3, T + TTL_MS);
    assert!(is_courier(m.get("k", T + 4)));
}

/// Bounded: a full memo is emptied, never grown past `CAP`.
#[test]
fn the_memo_is_bounded() {
    let mut m = Memo::default();
    let e = m.begin();
    for i in 0..CAP + 3 {
        m.store(format!("k{i}"), &courier(), e, T, T + TTL_MS);
        assert!(m.len() <= CAP);
    }
    assert!(m.get(&format!("k{}", CAP + 2), T).is_some(), "the newest entry survives the emptying");
}

/// THE GUARD ITSELF, over a platform in memory: what it reads from `__platform`, and how fast
/// a revocation reaches it. Every call is the real `auth::authenticate_token`.
mod live {
    use super::TTL_MS;
    use crate::auth::{authenticate_token, verify, AuthError, Claims, Principal};
    use crate::edge::mem::{block_on, object_paths};
    use crate::edge::site::{Site, T0};
    use crate::storefront::route_costs::is_venue;
    use crate::storefront::route_tests::open_venue;

    /// The platform-object requests `f` made, by path.
    fn platform_reads<T>(f: impl FnOnce() -> T) -> (Vec<String>, T) {
        let from = object_paths().len();
        let out = f();
        (object_paths()[from..].iter().filter(|p| !is_venue(p)).cloned().collect(), out)
    }

    fn auth(site: &Site, token: &str, now: i64) -> Result<Principal, AuthError> {
        block_on(authenticate_token(token, &site.env(), now))
    }

    /// A WRITE FROM ANOTHER ISOLATE: the platform's image changed through the object's own
    /// storage path, not through this isolate's `platform_store::with` -- the case the window is for.
    fn elsewhere(site: &Site, image: &str, f: impl FnMut(&mut dowiz_hub::table::Table) -> worker::Result<()>) {
        let stub = crate::platform_store::stub(&site.env()).expect("platform stub");
        block_on(crate::platform_store::with_at(&stub, image, crate::platform_store::ceiling(image), f)).expect("write elsewhere");
    }

    fn revoke_courier_session(t: &mut dowiz_hub::table::Table, jti: &str) -> worker::Result<()> {
        use crate::identity_store as ids;
        let mut r = ids::rec(t, ids::K_CSESSION, jti).expect("the courier's session row");
        r["revoked_at_ms"] = serde_json::json!(T0);
        t.put(ids::K_CSESSION, jti, &r.to_string(), &[], &[]).map_err(|e| worker::Error::RustError(format!("{e:?}")))
    }

    fn courier_jti(site: &Site, token: &str) -> String {
        match verify(&site.env(), token, T0) {
            Ok(Claims::Courier { jti, .. }) => jti,
            _ => panic!("not a courier token"),
        }
    }

    /// THE CUT: a courier's step reads the platform's two sign-in images once, and the steps
    /// after it inside the window read nothing from the platform at all.
    #[test]
    fn a_courier_reads_the_platform_once_per_window() {
        let site = Site::new();
        let (owner, _) = open_venue(&site, "alpha", "alpha@example.com");
        let (rider, _) = site.courier("alpha", &owner, "+355691111111");
        let (first, p) = platform_reads(|| auth(&site, &rider, T0));
        assert!(matches!(p, Ok(Principal::Courier { .. })));
        assert_eq!(first, vec!["/img/sessions", "/img/couriers"], "the first step reads the live rows");
        for dt in [1, 12_000, TTL_MS - 1] {
            let (again, p) = platform_reads(|| auth(&site, &rider, T0 + dt));
            assert!(matches!(p, Ok(Principal::Courier { .. })));
            assert!(again.is_empty(), "a step {dt} ms later read the platform: {again:?}");
        }
        let (after, _) = platform_reads(|| auth(&site, &rider, T0 + TTL_MS));
        assert_eq!(after.len(), 2, "past the window the live rows are read again");
    }

    /// The same for a cook.
    #[test]
    fn a_cook_reads_the_platform_once_per_window() {
        let site = Site::new();
        let (owner, _) = open_venue(&site, "alpha", "alpha@example.com");
        let cook = site.staff("alpha", &owner, "k@x.test", "kitchen");
        let (first, p) = platform_reads(|| auth(&site, &cook, T0));
        assert!(matches!(p, Ok(Principal::Staff { .. })), "{:?}", p.err());
        assert_eq!(first, vec!["/img/identity", "/img/sessions"]);
        let (again, p) = platform_reads(|| auth(&site, &cook, T0 + 30_000));
        assert!(matches!(p, Ok(Principal::Staff { .. })));
        assert!(again.is_empty(), "{again:?}");
    }

    /// OWNERS ARE NOT REMEMBERED: every owner step reads `identity`, and an owner removed by
    /// another isolate is refused on the very next request (W-CHEAP's rule, kept exact).
    #[test]
    fn an_owner_removed_elsewhere_is_refused_on_the_next_request() {
        let site = Site::new();
        let (owner, _) = open_venue(&site, "alpha", "alpha@example.com");
        for dt in [0, 1] {
            let (reads, p) = platform_reads(|| auth(&site, &owner, T0 + dt));
            assert!(matches!(p, Ok(Principal::Owner { .. })));
            assert_eq!(reads, vec!["/img/identity"], "an owner's every step re-derives the membership");
        }
        let Ok(Claims::Owner { user_id, .. }) = verify(&site.env(), &owner, T0) else { panic!("owner token") };
        elsewhere(&site, crate::platform_store::IDENTITY, |t| {
            use crate::identity_store as ids;
            let id = ids::member_id("alpha", &user_id);
            let mut m = ids::rec(t, ids::K_MEMBER, &id).expect("membership");
            m["status"] = serde_json::json!("suspended");
            t.put(ids::K_MEMBER, &id, &m.to_string(), &[], &[]).map_err(|e| worker::Error::RustError(format!("{e:?}")))
        });
        assert!(matches!(auth(&site, &owner, T0 + 2), Err(AuthError::Revoked(_))), "a removed owner was admitted");
    }

    /// THE REVOCATION GUARANTEE, CROSS-ISOLATE: a courier session revoked by another isolate is
    /// refused no later than `TTL_MS` after this isolate last read the live rows -- and, so the
    /// bound is shown to be the real one, not earlier than the window allows.
    #[test]
    fn a_revoked_courier_is_refused_within_the_window() {
        let site = Site::new();
        let (owner, _) = open_venue(&site, "alpha", "alpha@example.com");
        let (rider, _) = site.courier("alpha", &owner, "+355691111111");
        assert!(auth(&site, &rider, T0).is_ok());
        let jti = courier_jti(&site, &rider);
        elsewhere(&site, crate::platform_store::SESSIONS, |t| revoke_courier_session(t, &jti));
        assert!(auth(&site, &rider, T0 + TTL_MS - 1).is_ok(), "inside the documented window");
        assert!(
            matches!(auth(&site, &rider, T0 + TTL_MS), Err(AuthError::Revoked("courier session revoked"))),
            "a revoked courier was still admitted after the window"
        );
    }

    /// SAME-ISOLATE: a revocation written through this isolate's writers ends reuse at once.
    #[test]
    fn a_courier_revoked_through_this_isolate_is_refused_at_once() {
        let site = Site::new();
        let (owner, _) = open_venue(&site, "alpha", "alpha@example.com");
        let (rider, _) = site.courier("alpha", &owner, "+355691111111");
        assert!(auth(&site, &rider, T0).is_ok());
        let jti = courier_jti(&site, &rider);
        block_on(crate::identity_store::with_sessions(&site.env(), |t| revoke_courier_session(t, &jti))).expect("revoke");
        assert!(matches!(auth(&site, &rider, T0 + 1), Err(AuthError::Revoked(_))), "the memo outlived a local revocation");
    }

    /// A cook removed from the roster elsewhere: the same window, the same bound.
    #[test]
    fn a_cook_removed_elsewhere_is_refused_within_the_window() {
        let site = Site::new();
        let (owner, _) = open_venue(&site, "alpha", "alpha@example.com");
        let (cook, person) = site.staff_full("alpha", &owner, "k@x.test", "kitchen");
        assert!(auth(&site, &cook, T0).is_ok());
        elsewhere(&site, crate::platform_store::IDENTITY, |t| {
            use crate::identity_store as ids;
            let id = ids::member_id("alpha", &person);
            let mut m = ids::rec(t, ids::K_MEMBER, &id).expect("membership");
            m["status"] = serde_json::json!("suspended");
            t.put(ids::K_MEMBER, &id, &m.to_string(), &[], &[]).map_err(|e| worker::Error::RustError(format!("{e:?}")))
        });
        assert!(auth(&site, &cook, T0 + TTL_MS - 1).is_ok());
        assert!(matches!(auth(&site, &cook, T0 + TTL_MS), Err(AuthError::Revoked(_))));
    }

    /// THE ORDER'S BILL (`storefront::route_costs`, the walk W-BN4B measured): one delivery
    /// order, every step as the router runs it. Before the memo its sign-in steps read the
    /// platform 20 times; the owner's 6 stay (never memoised), the cook's and the courier's
    /// first steps read their two images once each.
    #[test]
    fn one_delivery_order_reads_the_platform_at_most_eight_times() {
        use crate::storefront::route_costs::{a_delivery_order, Via};
        let (steps, order) = a_delivery_order(&Site::new(), Via::Object);
        assert_eq!(order["status"], serde_json::json!("DELIVERED"));
        let mut total = 0;
        for s in &steps {
            let platform = s.paths.iter().filter(|p| !is_venue(p)).count();
            eprintln!("AUTHC {:<14} {} venue {} platform {} {:?}", s.name, s.status, s.venue(), platform, s.paths);
            assert!(s.status < 300, "{} answered {}", s.name, s.status);
            total += platform;
        }
        eprintln!("AUTHC platform requests per order {total}");
        assert!(total <= 8, "one order read the platform {total} times");
    }
}
