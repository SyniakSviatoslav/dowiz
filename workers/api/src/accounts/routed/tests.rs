//! The sign-in doors through the platform object (W-LOGIN-DO): each costs the Worker ONE object
//! request and answers as its body did from the Worker; a door runs in `__platform` and nowhere
//! else; a turn that changes nothing rewrites nothing; and the sessions image, measured.

use crate::edge::mem::{block_on, object_paths};
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::hubdo::command::{Command, Kind};
use crate::platform_store::{self, PLATFORM, SESSIONS};
use crate::wire::{Call, Reply};
use serde_json::json;
use worker::Method;

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

/// The object requests `f` made, by path, and what it answered.
fn paid(f: impl FnOnce() -> Reply) -> (Vec<String>, Reply) {
    let from = object_paths().len();
    let r = f();
    (object_paths()[from..].to_vec(), r)
}

/// One request to the platform object, status 200 with `key` in the body; the answer. `then`
/// is what the platform object itself asks of ANOTHER object inside that turn (the test edge
/// counts those too, as the platform bills them): nothing, for every door but the courier's login.
fn one(name: &str, key: &str, then: &[&str], f: impl FnOnce() -> Reply) -> serde_json::Value {
    let (paths, r) = paid(f);
    assert_eq!(r.status_code(), 200, "{name}: {}", r.body_str());
    let mut want = vec!["/fold/command".to_string()];
    want.extend(then.iter().map(|p| p.to_string()));
    assert_eq!(paths, want, "{name}: the Worker's one request, then the object's own");
    let v = r.body_value();
    assert!(!v[key].is_null(), "{name}: no `{key}` in {v}");
    v
}

fn code_of(site: &Site, handler_name: &str, r: Reply) -> String {
    assert_eq!(r.status_code(), 200, "{handler_name}: {}", r.body_str());
    let _ = site;
    r.body_value()["code"].as_str().expect("code").to_string()
}

#[test]
fn every_sign_in_door_costs_the_worker_one_platform_request_and_answers_as_its_body_did() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let login = || post(&at("alpha", "/api/auth/login"), &json!({"email": "a@x.test", "password": "owner-password-1"})).on("alpha");
    // THE BODY FROM THE WORKER (what every login was before this lane): the identity image read,
    // the sessions image read and written back, each a request to the platform object.
    let (direct, r) = paid(|| site.run(crate::accounts::owner_login_at, login(), &[]));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(direct.len() >= 2 && direct.iter().all(|p| p.starts_with("/img/")), "from the Worker: {direct:?}");
    // ROUTED: one request, the same answer.
    let full = one("owner login", "refresh_token", &[], || site.run(crate::accounts::owner_login, login(), &[]));
    let rt = full["refresh_token"].as_str().unwrap().to_string();
    let access = full["access_token"].as_str().unwrap().to_string();
    let rot = one("owner refresh", "refresh_token", &[], || {
        site.run(crate::accounts::owner_refresh, post(&at("alpha", "/api/auth/refresh"), &json!({"refresh_token": rt})).on("alpha"), &[])
    });
    assert_ne!(rot["refresh_token"], full["refresh_token"], "rotated");
    one("owner logout", "ok", &[], || site.run(crate::accounts::owner_logout, post(&at("alpha", "/api/auth/logout"), &json!({})).bearer(&access).on("alpha"), &[]));

    // THE COURIER'S THREE DOORS.
    let code = code_of(&site, "invite courier", site.run(
        crate::services::courier::hiring::invite_courier,
        post(&at("alpha", "/api/owner/couriers/invite"), &json!({"phone": "+355694444444", "name": "Rider"})).bearer(&owner).on("alpha"),
        &[],
    ));
    let claimed = one("courier claim", "refreshToken", &[], || {
        site.run(crate::accounts::courier_claim, post(&at("alpha", "/api/courier/auth/claim"), &json!({"phone": "+355694444444", "code": code, "password": "courier-password-1"})).on("alpha"), &[])
    });
    let logged = one("courier login", "refreshToken", &["/img/audit", "/img/audit"], || { // the venue's audit log, appended by the platform object through the venue's own object
        site.run(crate::accounts::courier_login, post(&at("alpha", "/api/courier/auth/login"), &json!({"phone": "+355694444444", "password": "courier-password-1"})).on("alpha"), &[])
    });
    assert_eq!(logged["courier"]["id"], claimed["courier"]["id"]);
    let crt = logged["refreshToken"].as_str().unwrap().to_string();
    one("courier refresh", "jwt", &[], || {
        site.run(crate::accounts::session_renew::courier_refresh, post(&at("alpha", "/api/courier/auth/refresh"), &json!({"refreshToken": crt})).on("alpha"), &[])
    });

    // THE STAFF'S FIVE.
    let code = code_of(&site, "invite staff", site.run(
        crate::services::identity::staff_admin::invite_staff,
        post(&at("alpha", "/api/owner/staff/invite"), &json!({"email": "w@x.test", "name": "Staff", "role": "waiter"})).bearer(&owner).on("alpha"),
        &[],
    ));
    let staff = one("staff claim", "jwt", &[], || {
        site.run(crate::services::identity::staff::staff_claim, post(&at("alpha", "/api/staff/claim"), &json!({"email": "w@x.test", "code": code, "password": "staff-password-1"})).on("alpha"), &[])
    });
    let sid = staff["staff"]["id"].as_str().unwrap().to_string();
    let staff_login = |pw: &str| post(&at("alpha", "/api/staff/login"), &json!({"email": "w@x.test", "password": pw})).on("alpha");
    let jwt = one("staff login", "jwt", &[], || site.run(crate::services::identity::staff::staff_login, staff_login("staff-password-1"), &[]))["jwt"].as_str().unwrap().to_string();
    one("staff refresh", "jwt", &[], || site.run(crate::accounts::session_renew::staff_refresh, post(&at("alpha", "/api/staff/refresh"), &json!({})).bearer(&jwt).on("alpha"), &[]));
    one("staff password", "changed", &[], || {
        site.run(crate::services::identity::staff::staff_password, post(&at("alpha", "/api/staff/password"), &json!({"email": "w@x.test", "old_password": "staff-password-1", "new_password": "staff-password-2"})).on("alpha"), &[])
    });
    one("staff login after the change", "jwt", &[], || site.run(crate::services::identity::staff::staff_login, staff_login("staff-password-2"), &[]));
    one("owner reset", "ok", &[], || {
        site.run(crate::services::identity::staff::password::owner_reset, post(&at("alpha", &format!("/api/owner/staff/{sid}/password")), &json!({"new_password": "staff-password-3"})).bearer(&owner).on("alpha"), &[("id", &sid)])
    });
    one("staff login after the reset", "jwt", &[], || site.run(crate::services::identity::staff::staff_login, staff_login("staff-password-3"), &[]));
    // A wrong password is still refused, through the same one request.
    let (paths, r) = paid(|| site.run(crate::services::identity::staff::staff_login, staff_login("staff-password-1"), &[]));
    assert_eq!((r.status_code(), paths), (401, vec!["/fold/command".to_string()]), "{}", r.body_str());
}

/// `body` to the object named `to`, through the stub a Worker would hold.
fn to_object(site: &Site, to: &str, cmd: &Command) -> Reply {
    let stub = site.env().durable_object("HUB").unwrap().id_from_name(to).unwrap().get_stub().unwrap();
    let body = serde_json::to_vec(cmd).unwrap();
    block_on(stub.fetch_with_request(Call::new("https://hub/fold/command", Method::Post).unwrap().with_body(body))).expect("answered")
}

fn login_command(kind: Kind, venue: &str) -> Command {
    Command {
        kind,
        venue: venue.to_string(),
        params: vec![],
        method: "POST".into(),
        now_ms: crate::edge::site::T0,
        url: at("alpha", "/api/auth/login"),
        headers: vec![("host".into(), format!("alpha.{PLATFORM_HOST}")), ("content-type".into(), "application/json".into())],
        body: json!({"email": "a@x.test", "password": "owner-password-1"}).to_string(),
    }
}

#[test]
fn a_sign_in_runs_in_the_platform_object_and_nowhere_else() {
    let site = Site::new();
    site.venue("alpha", "a@x.test");
    let alpha = block_on(crate::hubstore::Place::of_slug(&site.ctx(&[]), "alpha")).expect("registry").venue;
    // The platform object, asked as the Worker asks: the login answers.
    let r = to_object(&site, PLATFORM, &login_command(Kind::OwnerLogin, PLATFORM));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(r.body_value()["access_token"].is_string());
    // A venue's object, told to run a login as itself: refused -- it would reach the platform's
    // images over a hop, from inside another tenant's object.
    let r = to_object(&site, &alpha, &login_command(Kind::OwnerLogin, &alpha));
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    assert!(r.body_str().contains("does not run in"), "{}", r.body_str());
    // A venue's object, handed a platform command: not that object (the tenancy check first).
    let r = to_object(&site, &alpha, &login_command(Kind::OwnerLogin, PLATFORM));
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    // The platform object, told to run a VENUE's command as the platform: refused the same way.
    let mut place = login_command(Kind::Place, PLATFORM);
    place.params = vec![("slug".into(), "alpha".into())];
    let r = to_object(&site, PLATFORM, &place);
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    assert!(r.body_str().contains("does not run in"), "{}", r.body_str());
}

/// The generation the platform object holds `image` at.
fn generation(site: &Site, image: &str) -> i64 {
    let r = site.fold(PLATFORM, &format!("/img/{image}"));
    assert_eq!(r.status_code(), 200, "{image}: {}", r.body_str());
    r.headers().get("x-generation").ok().flatten().and_then(|g| g.parse().ok()).expect("x-generation")
}

#[test]
fn a_turn_that_changes_nothing_does_not_rewrite_the_image() {
    let site = Site::new();
    site.venue("alpha", "a@x.test");
    let g0 = generation(&site, SESSIONS);
    // A read-modify-write that decided to change nothing: no PUT, the generation stays.
    block_on(platform_store::with(&site.env(), SESSIONS, |t| {
        assert!(!t.dirty(), "a freshly loaded table is clean");
        Ok(())
    }))
    .unwrap();
    assert_eq!(generation(&site, SESSIONS), g0, "nothing changed, nothing written");
    // The real-world shapes: a refresh with a token nobody issued, and a second logout.
    let r = site.run(crate::accounts::owner_refresh, post(&at("alpha", "/api/auth/refresh"), &json!({"refresh_token": "nobody-issued-this"})).on("alpha"), &[]);
    assert_eq!(r.status_code(), 401);
    assert_eq!(generation(&site, SESSIONS), g0, "a missing token writes nothing");
    // A write that does change the table moves the generation by one -- the twin.
    block_on(platform_store::with(&site.env(), SESSIONS, |t| {
        t.put("probe", "p1", "{}", &[], &[]).map_err(|e| worker::Error::RustError(e.to_string()))?;
        assert!(t.dirty());
        Ok(())
    }))
    .unwrap();
    assert_eq!(generation(&site, SESSIONS), g0 + 1, "a change is written once");
    // A remove of what is not there is not a change either (`Table::remove` answers false).
    block_on(platform_store::with(&site.env(), SESSIONS, |t| {
        assert!(!t.remove("probe", "absent"));
        assert!(!t.dirty(), "removing nothing dirties nothing");
        Ok(())
    }))
    .unwrap();
    assert_eq!(generation(&site, SESSIONS), g0 + 1);
}

#[test]
fn the_sessions_image_is_measured() {
    // (b) of the lane card: what the sessions image weighs, natively. Two venues' owners each
    // signed in to the cap (`REFRESH_RECORDS_KEPT`), one courier and one member of staff.
    let mut site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    site.venue("beta", "b@x.test");
    let bytes = |site: &Site| site.fold(PLATFORM, &format!("/img/{SESSIONS}")).body().len();
    let before = bytes(&site);
    let n = crate::accounts::REFRESH_RECORDS_KEPT;
    for _ in 0..n {
        site.now_ms += 60_000;
        site.login("alpha", "a@x.test");
        site.login("beta", "b@x.test");
    }
    let owners = bytes(&site);
    site.courier("alpha", &owner, "+355695555555");
    site.staff("alpha", &owner, "w@x.test", "waiter");
    let all = bytes(&site);
    let per_refresh = (owners - before) / (2 * n - 2); // the two logins `venue` made are already in `before`
    eprintln!("sessions image: {before} B after two venues; {owners} B after 2 x {n} owner logins ({per_refresh} B per refresh record); {all} B with a courier and a staff session");
    assert!(per_refresh > 100 && per_refresh < 1024, "a refresh record and its family key: {per_refresh} B");
    assert!(all < 64 * 1024, "two venues at the cap fit in 64 KiB: {all} B");
}
