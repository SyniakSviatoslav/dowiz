//! The sign-in routes, through the route seam (W-COV C2): refresh rotation, logout, and the
//! courier's password login -- each checked against what the session store then holds.

use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use serde_json::json;

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn login_full(site: &Site) -> serde_json::Value {
    site.run(
        crate::accounts::owner_login,
        post(&at("alpha", "/api/auth/login"), &json!({"email": "a@x.test", "password": "owner-password-1"})).on("alpha"),
        &[],
    )
    .body_value()
}

#[test]
fn a_refresh_token_rotates_once_and_a_replay_is_refused() {
    let mut site = Site::new();
    site.venue("alpha", "a@x.test");
    let first = login_full(&site);
    let rt = first["refresh_token"].as_str().expect("refresh").to_string();
    let r = site.run(crate::accounts::owner_refresh, post(&at("alpha", "/api/auth/refresh"), &json!({"refresh_token": rt})).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(r.body_value()["access_token"].is_string());
    let rotated = r.body_value()["refresh_token"].as_str().expect("rotated").to_string();
    // Within the grace window a second use is an honest race: 409, and nothing is revoked.
    let race = site.run(crate::accounts::owner_refresh, post(&at("alpha", "/api/auth/refresh"), &json!({"refresh_token": rt})).on("alpha"), &[]);
    assert_eq!(race.status_code(), 409, "{}", race.body_str());
    // Past it, a spent token is a stolen one: 401 and the WHOLE family goes, the rotated one too.
    site.now_ms += 60_000;
    let replay = site.run(crate::accounts::owner_refresh, post(&at("alpha", "/api/auth/refresh"), &json!({"refresh_token": rt})).on("alpha"), &[]);
    assert_eq!(replay.status_code(), 401, "a reused refresh token is refused: {}", replay.body_str());
    let heir = site.run(crate::accounts::owner_refresh, post(&at("alpha", "/api/auth/refresh"), &json!({"refresh_token": rotated})).on("alpha"), &[]);
    assert_eq!(heir.status_code(), 401, "the family was revoked: {}", heir.body_str());
    let junk = site.run(crate::accounts::owner_refresh, post(&at("alpha", "/api/auth/refresh"), &json!({"refresh_token": "nope"})).on("alpha"), &[]);
    assert_eq!(junk.status_code(), 401);
}

#[test]
fn logout_ends_every_refresh_token_of_the_owner() {
    let site = Site::new();
    site.venue("alpha", "a@x.test");
    let one = login_full(&site);
    let access = one["access_token"].as_str().unwrap().to_string();
    let rt = one["refresh_token"].as_str().unwrap().to_string();
    let r = site.run(crate::accounts::owner_logout, post(&at("alpha", "/api/auth/logout"), &json!({})).bearer(&access).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let after = site.run(crate::accounts::owner_refresh, post(&at("alpha", "/api/auth/refresh"), &json!({"refresh_token": rt})).on("alpha"), &[]);
    assert_eq!(after.status_code(), 401, "logged out: {}", after.body_str());
    let anon = site.run(crate::accounts::owner_logout, post(&at("alpha", "/api/auth/logout"), &json!({})).on("alpha"), &[]);
    assert_eq!(anon.status_code(), 401);
}

#[test]
fn a_courier_signs_in_with_the_password_it_claimed_and_not_with_another() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let (_, id) = site.courier("alpha", &owner, "+355693333333");
    let ok = site.run(
        crate::accounts::courier_login,
        post(&at("alpha", "/api/courier/auth/login"), &json!({"phone": "+355693333333", "password": "courier-password-1"})).on("alpha"),
        &[],
    );
    assert_eq!(ok.status_code(), 200, "{}", ok.body_str());
    assert!(ok.body_str().contains(&id));
    let bad = site.run(
        crate::accounts::courier_login,
        post(&at("alpha", "/api/courier/auth/login"), &json!({"phone": "+355693333333", "password": "wrong-password"})).on("alpha"),
        &[],
    );
    assert_eq!(bad.status_code(), 401);
    // A second claim of the same code is refused: it was used.
    let again = site.run(
        crate::accounts::courier_claim,
        post(&at("alpha", "/api/courier/auth/claim"), &json!({"phone": "+355693333333", "code": "000000", "password": "x-password-1"})).on("alpha"),
        &[],
    );
    assert_eq!(again.status_code(), 400);
}

#[test]
fn an_old_cost_password_still_signs_in_and_is_rewritten_at_the_new_cost_on_that_login() {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    let site = Site::new();
    site.venue("alpha", "a@x.test");
    let env = site.env();
    let old = argon2::Argon2::default()
        .hash_password(b"owner-password-1", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    assert!(crate::auth::needs_rehash(&old));
    let stored_hash = || {
        let t = crate::edge::mem::block_on(crate::identity_store::identity(&env)).unwrap();
        let id = crate::identity_store::user_id_for_email(&t, "a@x.test").expect("the owner exists");
        crate::identity_store::s_of(&crate::identity_store::rec(&t, crate::identity_store::K_USER, &id).unwrap(), "password_hash")
    };
    let o = old.clone();
    crate::edge::mem::block_on(crate::identity_store::with_identity(&env, move |t| {
        let id = crate::identity_store::user_id_for_email(t, "a@x.test").unwrap();
        let mut u = crate::identity_store::rec(t, crate::identity_store::K_USER, &id).unwrap();
        u["password_hash"] = json!(o);
        let index = vec![(crate::identity_store::user_by_email("a@x.test"), id.clone())];
        t.put(crate::identity_store::K_USER, &id, &u.to_string(), &index, &[]).map_err(|e| worker::Error::RustError(format!("{e:?}")))?;
        Ok(())
    }))
    .unwrap();
    assert_eq!(stored_hash(), old);
    // The account that exists signs in -- deploying the new cost locked nobody out ...
    let full = login_full(&site);
    assert!(full["access_token"].is_string(), "{full}");
    // ... and the plaintext that existed for that moment rewrote the hash.
    let now = stored_hash();
    assert_ne!(now, old, "the hash was rewritten on this login");
    assert!(!crate::auth::needs_rehash(&now), "{now}");
    assert!(crate::auth::verify_password("owner-password-1", &now));
    // A wrong password neither signs in nor rewrites anything.
    let r = site.run(crate::accounts::owner_login, post(&at("alpha", "/api/auth/login"), &json!({"email": "a@x.test", "password": "nope"})).on("alpha"), &[]);
    assert_eq!(r.status_code(), 401, "{}", r.body_str());
    assert_eq!(stored_hash(), now);
}

// ── W-SESSION 2026-10-12: the refresh family slides for 30 quiet days and ends 90 days after the login ──
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

fn refresh(site: &Site, rt: &str) -> crate::wire::Reply {
    site.run(crate::accounts::owner_refresh, post(&at("alpha", "/api/auth/refresh"), &json!({"refresh_token": rt})).on("alpha"), &[])
}

#[test]
fn a_refresh_token_still_works_after_twenty_quiet_days() {
    // The idle window is 30 days (auth::REFRESH_IDLE_MS): an owner back from a three-week holiday is still signed in.
    let mut site = Site::new();
    site.venue("alpha", "a@x.test");
    let rt = login_full(&site)["refresh_token"].as_str().unwrap().to_string();
    site.now_ms += 20 * DAY_MS;
    let r = refresh(&site, &rt);
    assert_eq!(r.status_code(), 200, "20 quiet days: {}", r.body_str());
    // Past the window it is over, however valid the token was.
    let rt = r.body_value()["refresh_token"].as_str().unwrap().to_string();
    site.now_ms += 31 * DAY_MS;
    let r = refresh(&site, &rt);
    assert_eq!(r.status_code(), 401, "31 quiet days: {}", r.body_str());
}

#[test]
fn a_refresh_family_ends_ninety_days_after_the_login_however_often_it_rotated() {
    let mut site = Site::new();
    site.venue("alpha", "a@x.test");
    let mut rt = login_full(&site)["refresh_token"].as_str().unwrap().to_string();
    // Every ten days, well inside the idle window, for eighty days: each rotation answers.
    for day in 1..=8 {
        site.now_ms += 10 * DAY_MS;
        let r = refresh(&site, &rt);
        assert_eq!(r.status_code(), 200, "day {}: {}", day * 10, r.body_str());
        rt = r.body_value()["refresh_token"].as_str().unwrap().to_string();
    }
    // Day 90: the family is as old as a session may be; the owner signs in again.
    site.now_ms += 10 * DAY_MS;
    let r = refresh(&site, &rt);
    assert_eq!(r.status_code(), 401, "day 90: {}", r.body_str());
    assert!(r.body_str().contains("sign in again"), "{}", r.body_str());
}

#[test]
fn a_removed_owner_cannot_refresh_and_is_refused_on_the_next_call_at_once() {
    let site = Site::new();
    site.venue("alpha", "a@x.test");
    let full = login_full(&site);
    let access = full["access_token"].as_str().unwrap().to_string();
    let rt = full["refresh_token"].as_str().unwrap().to_string();
    let venue = full["user"]["locationId"].as_str().unwrap().to_string();
    let env = site.env();
    let v = venue.clone();
    crate::edge::mem::block_on(crate::identity_store::with_identity(&env, move |t| {
        let id = crate::identity_store::user_id_for_email(t, "a@x.test").unwrap();
        assert!(t.remove(crate::identity_store::K_MEMBER, &crate::identity_store::member_id(&v, &id)));
        Ok(())
    }))
    .unwrap();
    // The still-valid access token is refused on its very next call: authority is re-derived, never trusted.
    let call = crate::edge::site::get(&at("alpha", "/api/owner/orders")).bearer(&access).on("alpha");
    let p = crate::edge::mem::block_on(crate::auth::authenticate(&call, &env, site.now_ms));
    assert!(matches!(p, Err(crate::auth::AuthError::Revoked(_))), "{:?}", p.as_ref().err());
    let r = refresh(&site, &rt);
    assert_eq!(r.status_code(), 401, "a removed owner does not roll forward: {}", r.body_str());
}

#[test]
fn a_platform_admin_refreshes_without_a_venue() {
    // The main hub signs an administrator in through the same door; their family must rotate too.
    let site = Site::new();
    let r = site.run(
        crate::accounts::owner_login,
        post(&format!("https://{PLATFORM_HOST}/api/auth/login"), &json!({"email": "admin@dowiz.org", "password": "admin-password"})),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let rt = r.body_value()["refresh_token"].as_str().unwrap().to_string();
    let r = site.run(crate::accounts::owner_refresh, post(&format!("https://{PLATFORM_HOST}/api/auth/refresh"), &json!({"refresh_token": rt})), &[]);
    assert_eq!(r.status_code(), 200, "admin refresh: {}", r.body_str());
    let claims = crate::auth::verify(&site.env(), r.body_value()["access_token"].as_str().unwrap(), site.now_ms).unwrap();
    assert!(matches!(claims, crate::auth::Claims::Owner { active_location_id: None, .. }), "{claims:?}");
}
