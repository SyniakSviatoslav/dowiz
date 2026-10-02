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
