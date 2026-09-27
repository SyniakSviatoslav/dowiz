use super::*;
use crate::auth::hash_password;
use dowiz_hub::caps::Preset;

/// `staff::staff_password` (the route, main 282d2c45) decides through
/// `self_change` and writes through `store_hash` -- the rules tested here are
/// the route's, not a copy of them.
#[test]
fn the_route_asks_these_rules() {
    let route = include_str!("../../staff.rs");
    let body = &route[route.find("pub async fn staff_password").expect("the route")..];
    let body = &body[..body.find("\n}\n").expect("its end")];
    for n in ["password::self_change(", "password::store_hash(", "password::end_sessions("] {
        assert!(body.contains(n), "staff_password does not call {n}");
    }
}

fn member(role: &str) -> Value {
    sr::member_record("u1", "a", Preset::from_str(role).unwrap_or(Preset::Kitchen), 0)
        .as_object()
        .cloned()
        .map(|mut m| {
            m.insert("role".into(), json!(role));
            Value::Object(m)
        })
        .unwrap()
}

#[test]
fn a_wrong_old_password_is_401_and_a_missing_account_costs_the_same() {
    let stored = hash_password("old-password-1").unwrap();
    assert_eq!(self_change(Some(&stored), "not-it-at-all", "new-password-1"), Err((401, "invalid credentials")));
    assert_eq!(self_change(None, "old-password-1", "new-password-1"), Err((401, "invalid credentials")));
}

#[test]
fn a_new_password_under_eight_characters_is_400() {
    let stored = hash_password("old-password-1").unwrap();
    assert_eq!(self_change(Some(&stored), "old-password-1", "short").map_err(|e| e.0), Err(400));
    assert_eq!(new_password_ok("1234567").map_err(|e| e.0), Err(400));
    // The length is asked first: a short password with a wrong old one is 400,
    // and no derivation is spent on it.
    assert_eq!(self_change(Some(&stored), "not-it-at-all", "short").map_err(|e| e.0), Err(400));
    assert_eq!(new_password_ok("12345678"), Ok(()));
    // Characters, not bytes: eight Cyrillic letters are eight.
    assert_eq!(new_password_ok("пароль12"), Ok(()));
}

/// THE WHOLE ROUND, through the login's own check: after the change the new
/// password signs in (200) and the old one is `invalid credentials` (401).
#[test]
fn after_a_change_the_new_password_signs_in_and_the_old_one_does_not() {
    let old = "old-password-1";
    let new = "new-password-2";
    let user = json!({ "id": "u1", "email": "cook@x.al", "display_name": "Cook", "password_hash": hash_password(old).unwrap() });
    let stored = ids::s_of(&user, "password_hash");
    assert_eq!(self_change(Some(&stored), old, new), Ok(()));
    let changed = with_hash(user.clone(), &hash_password(new).unwrap(), 42);
    let now = ids::s_of(&changed, "password_hash");
    assert!(verify_password_constant_work(new, Some(&now)), "login with the new password -> 200");
    assert!(!verify_password_constant_work(old, Some(&now)), "login with the old password -> 401");
    assert_eq!(changed["email"], "cook@x.al", "the record keeps its address (and so its login index)");
    assert_eq!(changed["display_name"], "Cook");
    assert_eq!(changed["password_changed_at_ms"], 42);
}

#[test]
fn the_owner_resets_only_a_member_of_staff_of_their_own_venue() {
    let kitchen = member("kitchen");
    // The positive twin: staff here, and nowhere else.
    assert_eq!(owner_may_reset(Some(&kitchen), &[("a".into(), "kitchen".into())], "a"), Ok(()));
    // Another venue's owner: the person is not a member at the owner's venue.
    assert_eq!(owner_may_reset(None, &[("b".into(), "kitchen".into())], "a").map_err(|e| e.0), Err(403));
    // Staff here AND elsewhere: the password opens the other venue too.
    assert_eq!(owner_may_reset(Some(&kitchen), &[("a".into(), "kitchen".into()), ("b".into(), "owner".into())], "a").map_err(|e| e.0), Err(403));
    // An owner is not reset through the staff door.
    assert_eq!(owner_may_reset(Some(&member("owner")), &[("a".into(), "owner".into())], "a").map_err(|e| e.0), Err(403));
    // A courier word is not staff.
    assert_eq!(owner_may_reset(Some(&member("courier")), &[("a".into(), "courier".into())], "a").map_err(|e| e.0), Err(403));
}
