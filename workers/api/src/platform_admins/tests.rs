//! The first administrator: no secret, a wrong secret -> 404; the right one -> an account that
//! signs in and may call `/api/platform/compact`; an address already held -> 409, not adopted.

use crate::edge::site::{post, As, Site, PLATFORM_HOST, T0};
use serde_json::json;

const SECRET: &str = "bootstrap-secret-bootstrap-secret-0"; // `edge/site.rs`
const PASSWORD: &str = "a-long-admin-password";

fn url() -> String {
    format!("https://{PLATFORM_HOST}/api/platform/admins")
}

fn ask(site: &Site, secret: Option<&str>, email: &str) -> crate::wire::Reply {
    let mut c = post(&url(), &json!({ "email": email, "password": PASSWORD }));
    if let Some(s) = secret {
        c = c.with_header("x-dowiz-bootstrap", s);
    }
    site.run(super::create, c, &[])
}

fn login(site: &Site, email: &str, password: &str) -> crate::wire::Reply {
    site.run(
        crate::accounts::owner_login,
        post(&format!("https://{PLATFORM_HOST}/api/auth/login"), &json!({ "email": email, "password": password }))
            .with_header("host", PLATFORM_HOST),
        &[],
    )
}

#[test]
fn without_the_secret_the_route_does_not_exist() {
    let site = Site::new();
    assert_eq!(ask(&site, None, "root@dowiz.org").status_code(), 404, "no header");
    assert_eq!(ask(&site, Some("bootstrap-secret-bootstrap-secret-X"), "root@dowiz.org").status_code(), 404, "wrong secret");
    assert_eq!(ask(&site, Some(""), "root@dowiz.org").status_code(), 404, "empty secret");
    assert_eq!(login(&site, "root@dowiz.org", PASSWORD).status_code(), 401, "no account was made");
}

#[test]
fn a_worker_without_a_bootstrap_secret_answers_404_even_to_the_right_header() {
    // The site WITHOUT `BOOTSTRAP_SECRET`, as an unconfigured Worker is.
    let mut w = crate::edge::mem::World::new(T0);
    w.secrets.insert("JWT_SECRET".into(), "0123456789abcdef0123456789abcdef-test".into());
    w.vars.insert("PLATFORM_HOST".into(), PLATFORM_HOST.into());
    let site = Site { world: std::rc::Rc::new(w), now_ms: T0 };
    assert_eq!(ask(&site, Some(SECRET), "root@dowiz.org").status_code(), 404);
}

#[test]
fn the_right_secret_makes_an_administrator_who_signs_in_and_may_compact() {
    let site = Site::new();
    let r = ask(&site, Some(SECRET), " Root@Dowiz.org ");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(!r.body_str().contains(PASSWORD), "the password is never echoed");

    assert_eq!(login(&site, "root@dowiz.org", "wrong-password-1").status_code(), 401);
    let r = login(&site, "root@dowiz.org", PASSWORD);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let token = r.body_value()["access_token"].as_str().expect("token").to_string();

    let r = site.run(
        crate::hubdo::compact::fan::platform_compact,
        post(&format!("https://{PLATFORM_HOST}/api/platform/compact"), &json!({})).bearer(&token),
        &[],
    );
    assert_eq!(r.status_code(), 200, "the new administrator passes admin_only: {}", r.body_str());

    // Twin: an address already held is refused, not promoted -- neither the new admin's nor an owner's.
    assert_eq!(ask(&site, Some(SECRET), "root@dowiz.org").status_code(), 409);
    site.venue("alpha", "owner@x.test");
    assert_eq!(ask(&site, Some(SECRET), "owner@x.test").status_code(), 409);
    let owner = site.login("alpha", "owner@x.test");
    let r = site.run(
        crate::hubdo::compact::fan::platform_compact,
        post(&format!("https://{PLATFORM_HOST}/api/platform/compact"), &json!({})).bearer(&owner),
        &[],
    );
    assert_eq!(r.status_code(), 404, "the owner was not made an administrator");
}

#[test]
fn a_short_password_or_an_unknown_field_is_refused_and_writes_nothing() {
    let site = Site::new();
    let c = post(&url(), &json!({ "email": "a@dowiz.org", "password": "short" })).with_header("x-dowiz-bootstrap", SECRET);
    assert_eq!(site.run(super::create, c, &[]).status_code(), 400);
    let c = post(&url(), &json!({ "email": "a@dowiz.org", "password": PASSWORD, "admin": true })).with_header("x-dowiz-bootstrap", SECRET);
    assert_eq!(site.run(super::create, c, &[]).status_code(), 400);
    assert_eq!(login(&site, "a@dowiz.org", PASSWORD).status_code(), 401);
}
