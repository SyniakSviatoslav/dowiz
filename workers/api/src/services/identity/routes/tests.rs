//! Staff administration through the routes (W-COV C2): the owner's list, a role change, a member
//! switched off (their sessions end), the person's own password change and the owner's reset.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use serde_json::json;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn login(site: &Site, email: &str, password: &str) -> u16 {
    site.run(
        crate::services::identity::staff::staff_login,
        post(&at("/api/staff/login"), &json!({"email": email, "password": password})).on("alpha"),
        &[],
    )
    .status_code()
}

#[test]
fn the_owner_lists_changes_and_switches_off_a_member_of_staff() {
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    let (_, id) = site.staff_full("alpha", &t, "w@x.test", "waiter");
    let list = site.run(crate::services::identity::staff_admin::list_staff, get(&at("/api/owner/staff")).bearer(&t).on("alpha"), &[]);
    assert!(list.body_str().contains("w@x.test") || list.body_str().contains(&id), "{}", list.body_str());
    let set = |body: serde_json::Value| {
        site.run(crate::services::identity::staff_admin::set_staff, post(&at(&format!("/api/owner/staff/{id}")), &body).bearer(&t).on("alpha"), &[("id", &id)])
    };
    assert_eq!(set(json!({"role": "owner"})).status_code(), 400, "staff cannot be made an owner here");
    let r = set(json!({"role": "kitchen"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(login(&site, "w@x.test", "staff-password-1"), 200);
    let r = set(json!({"active": false}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(login(&site, "w@x.test", "staff-password-1"), 403, "a switched-off member cannot sign in");
    let ghost = site.run(crate::services::identity::staff_admin::set_staff, post(&at("/api/owner/staff/ghost"), &json!({"active": true})).bearer(&t).on("alpha"), &[("id", "ghost")]);
    assert_eq!(ghost.status_code(), 404);
    let b = site.venue("beta", "b@x.test");
    let r = site.run(
        crate::services::identity::staff_admin::set_staff,
        post(&at(&format!("/api/owner/staff/{id}?location_id=alpha")), &json!({"active": true})).bearer(&b).on("alpha"),
        &[("id", &id)],
    );
    assert!(r.status_code() >= 400, "{}", r.body_str());
}

#[test]
fn a_password_is_changed_by_its_owner_and_reset_by_the_venue() {
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    let (_, id) = site.staff_full("alpha", &t, "w@x.test", "waiter");
    let change = |old: &str, new: &str| {
        site.run(
            crate::services::identity::staff::staff_password,
            post(&at("/api/staff/password"), &json!({"email": "w@x.test", "old_password": old, "new_password": new})).on("alpha"),
            &[],
        )
    };
    assert_eq!(change("wrong-password", "new-password-2").status_code(), 401);
    assert_eq!(change("staff-password-1", "short").status_code(), 400);
    let r = change("staff-password-1", "new-password-2");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(login(&site, "w@x.test", "staff-password-1"), 401);
    assert_eq!(login(&site, "w@x.test", "new-password-2"), 200);
    let r = site.run(
        crate::services::identity::staff::password::owner_reset,
        post(&at(&format!("/api/owner/staff/{id}/password")), &json!({"new_password": "reset-password-3"})).bearer(&t).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(login(&site, "w@x.test", "reset-password-3"), 200);
    let wrong_host = site.run(
        crate::services::identity::staff::staff_login,
        post(&format!("https://{PLATFORM_HOST}/api/staff/login"), &json!({"email": "w@x.test", "password": "reset-password-3"})),
        &[],
    );
    assert_eq!(wrong_host.status_code(), 400, "staff sign in at their venue's address: {}", wrong_host.body_str());
}
