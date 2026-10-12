//! Renewal for the roles without a refresh family: the courier's session secret and the staff
//! member's live token -- each slid while used, each bounded, each refused the moment the row
//! or the roster behind it says no.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use serde_json::json;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const HOUR_MS: i64 = 60 * 60 * 1000;

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn courier_login(site: &Site) -> serde_json::Value {
    let r = site.run(
        crate::accounts::courier_login,
        post(&at("alpha", "/api/courier/auth/login"), &json!({"phone": "+355693333333", "password": "courier-password-1"})).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

fn courier_refresh(site: &Site, token: &str) -> crate::wire::Reply {
    site.run(crate::accounts::session_renew::courier_refresh, post(&at("alpha", "/api/courier/auth/refresh"), &json!({"refreshToken": token})).on("alpha"), &[])
}

fn live(site: &Site, jwt: &str) -> Result<crate::auth::Principal, crate::auth::AuthError> {
    let call = get(&at("alpha", "/api/courier/tasks")).bearer(jwt).on("alpha");
    crate::edge::mem::block_on(crate::auth::authenticate(&call, &site.env(), site.now_ms))
}

#[test]
fn a_courier_refresh_mints_a_new_jwt_on_the_same_session_and_slides_it() {
    let mut site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    site.courier("alpha", &owner, "+355693333333");
    let d = courier_login(&site);
    let jwt = d["jwt"].as_str().unwrap().to_string();
    let rt = d["refreshToken"].as_str().unwrap().to_string();
    // Twenty days off shift: the day-long jwt is dead, the session is not.
    site.now_ms += 20 * DAY_MS;
    assert!(matches!(live(&site, &jwt), Err(crate::auth::AuthError::Expired)), "{:?}", live(&site, &jwt).err());
    let r = courier_refresh(&site, &rt);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let jwt2 = r.body_value()["jwt"].as_str().unwrap().to_string();
    assert!(live(&site, &jwt2).is_ok());
    // Day 40 is past the row's original 30 days: the refresh slid it.
    site.now_ms += 20 * DAY_MS;
    let r = courier_refresh(&site, &rt);
    assert_eq!(r.status_code(), 200, "slid: {}", r.body_str());
    // Day 91: ninety days after the login the session is over, used or not.
    site.now_ms += 51 * DAY_MS;
    let r = courier_refresh(&site, &rt);
    assert_eq!(r.status_code(), 401, "day 91: {}", r.body_str());
}

#[test]
fn a_courier_refresh_is_refused_when_revoked_off_the_roster_or_not_a_token() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let (_, cid) = site.courier("alpha", &owner, "+355693333333");
    let d = courier_login(&site);
    let rt = d["refreshToken"].as_str().unwrap().to_string();
    let jwt = d["jwt"].as_str().unwrap().to_string();
    let venue = d["courier"]["locationId"].as_str().unwrap().to_string();
    for junk in ["nope", "a.b", &format!("{}.wrong", rt.split('.').next().unwrap())] {
        assert_eq!(courier_refresh(&site, junk).status_code(), 401, "{junk}");
    }
    // Off the roster: the live call and the refresh are refused at once.
    let (v, c) = (venue.clone(), cid.clone());
    crate::edge::mem::block_on(crate::identity_store::with_couriers(&site.env(), move |t| {
        assert!(t.remove(crate::identity_store::K_ROSTER, &crate::identity_store::roster_id(&v, &c)));
        Ok(())
    }))
    .unwrap();
    assert!(matches!(live(&site, &jwt), Err(crate::auth::AuthError::Revoked(_))));
    let r = courier_refresh(&site, &rt);
    assert_eq!(r.status_code(), 401, "off the roster: {}", r.body_str());
}

fn staff_refresh(site: &Site, jwt: &str) -> crate::wire::Reply {
    site.run(crate::accounts::session_renew::staff_refresh, post(&at("alpha", "/api/staff/refresh"), &json!({})).bearer(jwt).on("alpha"), &[])
}

#[test]
fn a_staff_token_is_renewed_while_in_use_and_ends_a_week_after_the_login() {
    let mut site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let (jwt, _) = site.staff_full("alpha", &owner, "k@x.test", "kitchen");
    // Ten hours into a twelve-hour token the console renews it: a new token, the same session.
    site.now_ms += 10 * HOUR_MS;
    let r = staff_refresh(&site, &jwt);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let mut jwt = r.body_value()["jwt"].as_str().unwrap().to_string();
    // Hour 14: the original token is dead, the renewed one is alive and renews again.
    site.now_ms += 4 * HOUR_MS;
    assert!(matches!(live(&site, &r.body_value()["jwt"].as_str().unwrap()), Ok(_)));
    // Every seven hours, inside each token's twelve: hour 21 to hour 161.
    for h in 0..21 {
        site.now_ms += 7 * HOUR_MS;
        let r = staff_refresh(&site, &jwt);
        assert_eq!(r.status_code(), 200, "renewal {h}: {}", r.body_str());
        jwt = r.body_value()["jwt"].as_str().unwrap().to_string();
    }
    // Hour 168, a week after the login: the token is alive, the session is not renewed.
    site.now_ms += 7 * HOUR_MS;
    let r = staff_refresh(&site, &jwt);
    // The guard refuses first (the slid row ends exactly at the cap), so the
    // word is "expired"; either way the cook signs in again.
    assert_eq!(r.status_code(), 401, "a week: {}", r.body_str());
}

#[test]
fn a_staff_renewal_is_refused_for_an_ended_session_or_a_dead_token() {
    let mut site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let (jwt, id) = site.staff_full("alpha", &owner, "k@x.test", "kitchen");
    let anon = site.run(crate::accounts::session_renew::staff_refresh, post(&at("alpha", "/api/staff/refresh"), &json!({})).on("alpha"), &[]);
    assert_eq!(anon.status_code(), 401);
    // The owner ends the session: no renewal rides over it.
    let r = site.run(
        crate::services::identity::staff_admin::set_staff,
        post(&at("alpha", "/api/owner/staff/{id}"), &json!({"active": false})).bearer(&owner).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(staff_refresh(&site, &jwt).status_code(), 401);
    // A token past its own expiry cannot be renewed either: renewal is for the living.
    let (jwt, _) = site.staff_full("alpha", &owner, "w@x.test", "waiter");
    site.now_ms += 13 * HOUR_MS;
    assert_eq!(staff_refresh(&site, &jwt).status_code(), 401);
}
