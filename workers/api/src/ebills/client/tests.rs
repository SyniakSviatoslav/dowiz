//! THE ALLOW-LIST IS CLOSED, and every way ebills.al can say "no" is named.
//! BLUEPRINT-EBILLS §2: `GET /api/close-shift` closes the shift. The verb is
//! not evidence; the path is the lock, and these tests are its proof.

use super::super::judge::{logged_in, read, Answer, Fail};
use super::*;

const SESSION_COOKIES: [&str; 3] = [
    "JSESSIONID=js1; path=/; secure; HttpOnly; SameSite=Lax",
    "XSRF-TOKEN=x2; path=/; secure",
    "remember-me=rm; Max-Age=2678400; path=/; HttpOnly",
];

fn every_path() -> Vec<Path> {
    vec![
        Path::Root,
        Path::Account,
        Path::Sales { begin: "2026-09-22".into(), end: "2026-09-23".into(), pos: 1, size: 500 },
        Path::Sale { id: 8602, pos: 1 },
        Path::Tables { pos: 1 },
        Path::Items { size: 500 },
    ]
}

/// Every variant of the type is a read of an allow-listed path, with no body.
#[test]
fn every_path_the_type_can_name_is_an_allow_listed_read() {
    let s = Session::default();
    for p in every_path() {
        let w = Wire::get(&p, &s).expect("a well-formed path builds");
        assert_eq!(w.verb(), Verb::Get, "{p:?}");
        assert_eq!(w.body(), None, "{p:?}: a read never carries a body");
        assert!(allowed(w.url(), false), "{p:?} -> {}", w.url());
        assert!(w.sendable(), "{p:?}");
    }
}

/// THE REFUSALS. The paths a reader might be tempted by, each one a write
/// or a 6.5 MB dump on this platform, and the tricks that would reach them.
#[test]
fn nothing_off_the_list_is_allowed() {
    for path in [
        "/api/close-shift",
        "/api/sale-pays",
        "/api/sales/summary-invoice",
        "/api/sales/get-sale-unit-orders",
        "/api/sales/close-sale-unit",
        "/api/sales-cancel/8602",
        "/api/sales/8602/removeItems",
        "/api/sales/8602/",
        "/api/sales/",
        "/api/sales/abc",
        "/api/sales/-1",
        "/api/sales/1/../../close-shift",
        "/api/sales/%2e%2e/close-shift",
        "//api/sales",
        "/api/extra-users",
        "/api/businesses",
        "/v3/api-docs",
        "/management/health",
    ] {
        assert!(!allowed(&format!("{ORIGIN}{path}"), false), "GET {path} must be refused");
    }
    assert!(!allowed("https://evil.example/api/sales", false), "another host");
    assert!(!allowed("http://www.ebills.al/api/sales", false), "plain http is another origin");
}

/// Its positive twin: the listed reads pass, with any query.
#[test]
fn the_listed_reads_pass() {
    for path in ["/", "/api/account", "/api/sales?x=1", "/api/sales/8602?currentPosId=1", "/api/sale-units-tables?pointOfSaleId=1", "/api/item-in-sales"] {
        assert!(allowed(&format!("{ORIGIN}{path}"), false), "GET {path}");
    }
}

/// THE ONE POST, and only with its body; no other path takes a POST.
#[test]
fn the_only_post_is_the_login() {
    assert!(allowed(&format!("{ORIGIN}/api/authentication"), true));
    for path in ["/api/sales", "/api/sales/get-sale-unit-orders", "/api/close-shift", "/", "/api/account"] {
        assert!(!allowed(&format!("{ORIGIN}{path}"), true), "POST {path} must be refused");
    }
    let mut s = Session::default();
    assert_eq!(Wire::login("u", "p", &s), None, "no CSRF cookie, no login");
    s.absorb(&["XSRF-TOKEN=tok1; path=/".to_string()]);
    let w = Wire::login("owner@x.al", "p&w=d +%", &s).expect("login builds");
    assert_eq!(w.verb(), Verb::Post);
    assert_eq!(w.url(), format!("{ORIGIN}/api/authentication"));
    assert!(w.sendable());
    assert_eq!(w.body(), Some("username=owner%40x.al&password=p%26w%3Dd+%2B%25&remember-me=true&submit=Login"));
    assert!(w.headers().iter().any(|(k, v)| k == "x-xsrf-token" && v == "tok1"));
    assert!(w.headers().iter().any(|(k, v)| k == "content-type" && v == "application/x-www-form-urlencoded"));
}

/// A path whose fields would put anything but a number or a date on the wire
/// does not build at all.
#[test]
fn a_malformed_path_does_not_build() {
    let s = Session::default();
    for p in [
        Path::Sales { begin: "2026-9-22".into(), end: "2026-09-23".into(), pos: 1, size: 5 },
        Path::Sales { begin: "2026-09-22".into(), end: "2026-09-23&x=/close-shift".into(), pos: 1, size: 5 },
        Path::Sales { begin: "2026-09-22".into(), end: "2026-09-23".into(), pos: 0, size: 5 },
        Path::Sale { id: 0, pos: 1 },
        Path::Sale { id: 5, pos: -1 },
        Path::Tables { pos: 0 },
    ] {
        assert_eq!(Wire::get(&p, &s), None, "{p:?}");
    }
}

#[test]
fn a_read_carries_the_browser_agent_the_cookies_and_the_tenant() {
    let mut s = Session { tenant: Some("ab".repeat(32)), ..Session::default() };
    s.absorb(&SESSION_COOKIES.map(String::from));
    let w = Wire::get(&Path::Account, &s).unwrap();
    let h = |k: &str| w.headers().iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    assert_eq!(h("user-agent").as_deref(), Some(USER_AGENT));
    assert!(USER_AGENT.starts_with("Mozilla/5.0"), "the WAF 403s curl's agent");
    assert_eq!(h("cookie").as_deref(), Some("JSESSIONID=js1; XSRF-TOKEN=x2; remember-me=rm"));
    assert_eq!(h("x-tenant-identifier"), s.tenant);
}

#[test]
fn cookies_rotate_and_expire() {
    let mut s = Session::default();
    assert!(s.absorb(&SESSION_COOKIES.map(String::from)));
    assert!(s.live());
    assert!(!s.absorb(&["JSESSIONID=js1; path=/".to_string()]), "the same value is no change");
    assert!(s.absorb(&["JSESSIONID=js9; path=/".to_string()]));
    assert_eq!(s.cookie("JSESSIONID"), Some("js9"));
    assert!(s.absorb(&["JSESSIONID=; Max-Age=0".to_string(), "remember-me=x; Max-Age=0; path=/".to_string()]));
    assert!(!s.live(), "both session cookies were cleared");
}

fn answer(status: u16, ct: &str, body: &str) -> Answer {
    Answer { status, content_type: ct.into(), body: body.into(), ..Answer::default() }
}

/// NEVER "NO SALES": every refusal shape is a named failure, and only JSON
/// on a 2xx is an answer.
#[test]
fn every_refusal_is_named_and_none_is_an_empty_answer() {
    assert_eq!(read(&answer(200, "application/json", "{\"sales\":[]}")), Ok("{\"sales\":[]}"));
    let problem = r#"{"type":"https://fiskalizim.al/problem-with-message","message":"error.http.401"}"#;
    assert_eq!(read(&answer(401, "application/problem+json", problem)), Err(Fail::Auth("401 error.http.401".into())));
    assert!(matches!(read(&answer(403, "text/html", "<html>blocked</html>")), Err(Fail::Auth(_))));
    assert!(matches!(read(&answer(302, "", "")), Err(Fail::Auth(_))));
    assert!(matches!(read(&answer(200, "text/html; charset=utf-8", "<!doctype html>")), Err(Fail::NotJson(_))));
    assert!(matches!(read(&answer(200, "application/problem+json", problem)), Err(Fail::Refused(200, _))));
    assert_eq!(read(&answer(404, "application/json", "{}")), Err(Fail::NotFound));
    assert_eq!(
        read(&answer(400, "application/json", r#"{"errorKey":"unpaidSales"}"#)),
        Err(Fail::Refused(400, "unpaidSales".into()))
    );
    assert!(Fail::Auth("x".into()).is_auth() && !Fail::NotFound.is_auth());
}

#[test]
fn a_login_needs_a_session_cookie_and_the_tenant() {
    let mut ok = answer(200, "", "");
    ok.set_cookies = SESSION_COOKIES.map(String::from).to_vec();
    ok.tenant = Some("f".repeat(64));
    let s = logged_in(&ok, Session::default()).expect("the measured login");
    assert!(s.live());
    assert_eq!(s.tenant.as_deref(), Some("f".repeat(64).as_str()));

    let mut no_tenant = ok.clone();
    no_tenant.tenant = None;
    assert!(matches!(logged_in(&no_tenant, Session::default()), Err(Fail::Auth(_))));
    let mut no_cookie = ok.clone();
    no_cookie.set_cookies.clear();
    assert!(matches!(logged_in(&no_cookie, Session::default()), Err(Fail::Auth(_))));
    let mfa = answer(401, "application/json", r#"{"mfaRequired":true,"activeMethod":"TOTP"}"#);
    assert_eq!(logged_in(&mfa, Session::default()), Err(Fail::Mfa));
    let mut many = ok.clone();
    many.tenant_needed = true;
    assert_eq!(logged_in(&many, Session::default()), Err(Fail::TenantNeeded));
    let mut shift = ok;
    shift.shift_error = Some("outOfShift".into());
    assert!(matches!(logged_in(&shift, Session::default()), Err(Fail::Auth(_))));
}
