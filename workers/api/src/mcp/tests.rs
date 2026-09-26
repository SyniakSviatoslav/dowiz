//! The MCP door per role: which tools each key is offered, what a key row
//! lets through, and what it never does. Every refusal has its positive twin.

use super::rolekey::{self as rk, Holder};
use super::tools::{self, Role};
use super::*;
use crate::auth::{hash_opaque, verify_opaque, Claims, Principal};
use dowiz_hub::caps::{Caps, Preset};
use serde_json::{json, Value};

const NOW: i64 = 1_800_000_000_000;

fn staff(p: Preset) -> Role { Role::Staff(p.caps()) }
fn names(role: Role) -> Vec<&'static str> { tools::tools_for(role).iter().map(|t| t.name).collect() }

// ── tools per role ──────────────────────────────────────────────────────────

#[test]
fn every_tool_schema_parses_and_names_are_unique_per_role() {
    for table in [tools::OWNER_TOOLS, tools::STAFF_TOOLS, tools::COURIER_TOOLS] {
        let mut seen = std::collections::BTreeSet::new();
        for t in table {
            assert!(serde_json::from_str::<Value>(t.schema).is_ok(), "{}", t.name);
            assert!(seen.insert(t.name), "duplicate {}", t.name);
        }
    }
}

#[test]
fn a_waiter_key_cannot_call_an_owner_tool_and_the_owner_can() {
    let w = staff(Preset::Waiter);
    for owner_only in ["dashboard", "orders", "customers", "set_dish", "backup_to_cloud", "assign_courier"] {
        assert!(tools::find(w, owner_only).is_none(), "{owner_only}");
        assert!(tools::find(Role::Owner, owner_only).is_some(), "{owner_only}");
    }
    assert_eq!(names(w), ["menu", "room", "floor", "table_cleared", "guest_round", "amend_round", "take_payment"]);
}

#[test]
fn staff_tools_follow_the_capabilities_of_the_word() {
    let (w, k, c) = (staff(Preset::Waiter), staff(Preset::Kitchen), staff(Preset::CounterManager));
    // The kitchen moves orders, and (operator Q8) runs the menu and the shelf;
    // a waiter does none of it.
    assert_eq!(names(k), ["menu", "kitchen_board", "order_action", "kitchen_ack", "dishes", "set_dish", "stock", "stock_move", "waste_report"]);
    for kitchen_only in ["kitchen_board", "dishes", "set_dish", "stock", "stock_move", "waste_report"] {
        assert!(tools::find(w, kitchen_only).is_none(), "a waiter got {kitchen_only}");
    }
    assert!(tools::find(w, "order_action").is_none());
    assert_eq!(tools::find(k, "order_action").unwrap().path, "/api/owner/orders/{id}/action");
    // The till's numbers are the counter-manager's, not the waiter's.
    assert!(tools::find(c, "tips").is_some());
    assert!(tools::find(w, "tips").is_none());
    // A set with no capability at all is offered only the menu.
    assert_eq!(names(Role::Staff(Caps::none())), ["menu"]);
}

#[test]
fn every_staff_tool_is_a_room_route_the_kitchen_route_or_the_menu() {
    for t in tools::STAFF_TOOLS {
        use dowiz_hub::caps::Cap;
        // An OWNER route is in a staff list only behind a word its guard
        // (`staff_at` / `staff::guard`) admits: the kitchen's three.
        let ok = t.path.starts_with("/api/staff/")
            || (t.path.starts_with("/api/owner/") && matches!(t.need, Some(Cap::Advance | Cap::Catalog | Cap::Stock)))
            || t.path.starts_with("/api/public/");
        assert!(ok, "{} -> {}", t.name, t.path);
    }
}

/// A COURIER KEY CANNOT SEE ANOTHER COURIER'S ORDER: every courier tool is a
/// `/api/courier/*` route, each of which names the courier by the token, and
/// none reads the venue's orders or an order by id.
#[test]
fn a_courier_key_reaches_only_courier_routes() {
    for t in tools::COURIER_TOOLS {
        assert!(t.path.starts_with("/api/courier/"), "{} -> {}", t.name, t.path);
        assert!(!t.loc_in_body, "{}", t.name);
    }
    for other in ["orders", "dashboard", "room", "menu", "couriers", "order_action"] {
        assert!(tools::find(Role::Courier, other).is_none(), "{other}");
    }
    assert!(tools::find(Role::Courier, "tasks").is_some());
}

#[test]
fn describe_lists_every_role_word_with_its_own_tools() {
    let d = describe_at("https://x.dowiz.org");
    assert_eq!(d["endpoint"], "https://x.dowiz.org/api/mcp");
    let roles = d["roles"].as_object().unwrap();
    let keys: Vec<&str> = roles.keys().map(String::as_str).collect();
    for want in ["owner", "waiter", "kitchen", "counter-manager", "courier"] {
        assert!(keys.contains(&want), "{want}");
    }
    let of = |r: &str| roles[r].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert!(of("owner").contains(&"dashboard".to_string()));
    assert!(!of("waiter").contains(&"dashboard".to_string()));
    assert_eq!(of("courier"), names(Role::Courier));
    assert_eq!(d["tools"].as_array().unwrap().len(), tool_count());
    assert_eq!(tool_names().len(), tool_count());
    let listed = tools::listed(staff(Preset::Kitchen));
    assert_eq!(listed.as_array().unwrap().len(), names(staff(Preset::Kitchen)).len());
    assert_eq!(listed.as_array().unwrap().len(), 9, "the pass, the menu and the shelf");
    assert!(listed[0]["inputSchema"].is_object());
}

// ── planning a call ─────────────────────────────────────────────────────────

#[test]
fn a_post_carries_the_keys_venue_over_whatever_the_agent_sent() {
    let t = tools::find(staff(Preset::Waiter), "guest_round").unwrap();
    let p = tools::plan(t, json!({ "id": "o 1", "action": "confirm", "location_id": "someone-else" }), "v1", "s", "https://h").unwrap();
    assert_eq!(p.url, "https://h/api/staff/orders/o%201/guest");
    let body: Value = serde_json::from_str(p.body.as_deref().unwrap()).unwrap();
    assert_eq!(body, json!({ "action": "confirm", "location_id": "v1" }));
    // A courier route takes the venue from the token and gets none.
    let t = tools::find(Role::Courier, "accept").unwrap();
    let p = tools::plan(t, json!({ "id": "o1" }), "v1", "s", "https://h").unwrap();
    assert_eq!((p.url.as_str(), p.body.as_deref()), ("https://h/api/courier/orders/o1/accept", Some("{}")));
}

#[test]
fn a_get_names_the_venue_and_passes_its_arguments() {
    let t = tools::find(Role::Owner, "orders").unwrap();
    let p = tools::plan(t, json!({ "status": "ready" }), "v 1", "s", "https://h").unwrap();
    assert_eq!(p, tools::Plan { url: "https://h/api/owner/orders?location_id=v%201".into(), body: None, status_filter: Some("READY".into()) });
    let t = tools::find(Role::Courier, "tasks").unwrap();
    assert_eq!(tools::plan(t, Value::Null, "v1", "s", "").unwrap().url, "/api/courier/tasks?location_id=v1");
    let t = tools::find(staff(Preset::CounterManager), "tips").unwrap();
    assert_eq!(tools::plan(t, json!({ "from_ms": 5 }), "v1", "s", "").unwrap().url, "/api/staff/till/tips?location_id=v1&from_ms=5");
    let t = tools::find(Role::Owner, "menu").unwrap();
    assert_eq!(tools::plan(t, json!({ "lang": "uk" }), "v1", "sushi", "").unwrap().url, "/api/public/locations/sushi/menu?location_id=v1&locale=uk");
    assert!(tools::plan(t, json!([1]), "v1", "s", "").is_err());
    let t = tools::find(Role::Owner, "order_action").unwrap();
    assert_eq!(tools::plan(t, json!({ "action": "ready" }), "v1", "s", "").unwrap_err(), "missing argument: id");
}

#[test]
fn holes_are_filled_from_arguments_and_removed() {
    let mut args = serde_json::Map::new();
    args.insert("id".into(), json!("a b"));
    args.insert("action".into(), json!("confirm"));
    let p = tools::fill_path("/api/owner/orders/{id}/action", &mut args, "s").unwrap();
    assert_eq!(p, "/api/owner/orders/a%20b/action");
    assert!(args.get("id").is_none() && args.get("action").is_some());
    args.insert("n".into(), json!(7));
    assert_eq!(tools::fill_path("/x/{n}", &mut args, "s").unwrap(), "/x/7");
    assert_eq!(tools::fill_path("/api/public/locations/{slug}/menu", &mut args, "sushi").unwrap(), "/api/public/locations/sushi/menu");
    assert!(tools::fill_path("/x/{peer}", &mut args, "s").is_err());
}

// ── the door ────────────────────────────────────────────────────────────────

#[test]
fn the_door_gives_each_principal_its_own_role_and_venue() {
    let owner = |l: Option<&str>| Principal::Owner { user_id: "u".into(), active_location_id: l.map(str::to_string) };
    assert_eq!(role_of(&owner(Some("v1"))), Ok((Role::Owner, "v1".into())));
    assert_eq!(role_of(&owner(None)), Err((403, "this key names no venue")));
    let s = Principal::Staff { person_id: "p".into(), active_location_id: "v2".into(), session_id: "s".into(), caps: Preset::Kitchen.caps() };
    assert_eq!(role_of(&s), Ok((staff(Preset::Kitchen), "v2".into())));
    let c = Principal::Courier { courier_id: "c".into(), active_location_id: "v3".into(), session_id: "s".into() };
    assert_eq!(role_of(&c), Ok((Role::Courier, "v3".into())));
    let g = Principal::Customer { customer_id: "g".into(), order_id: "o".into(), location_id: "v1".into() };
    assert_eq!(role_of(&g), Err((403, "an agent key is required")));
}

// ── the key ─────────────────────────────────────────────────────────────────

#[test]
fn a_key_is_parsed_only_in_its_own_spelling() {
    assert_eq!(rk::parse("dowizs_abc.s3cret"), Some((Holder::Staff, "abc", "s3cret")));
    assert_eq!(rk::parse("dowizc_abc.s3cret"), Some((Holder::Courier, "abc", "s3cret")));
    assert_eq!(rk::parse(&rk::spell(Holder::Courier, "i", "x")), Some((Holder::Courier, "i", "x")));
    for not_a_person_key in ["dowiz_abc.s3cret", "eyJhbGciOi.x.y", "dowizs_abc", "dowizs_.x", "dowizc_abc.", ""] {
        assert_eq!(rk::parse(not_a_person_key), None, "{not_a_person_key}");
    }
    assert_eq!(Holder::from_str("courier"), Some(Holder::Courier));
    assert_eq!(Holder::from_str("staff").map(Holder::kind), Some("ssession"));
    assert_eq!(Holder::from_str("owner"), None);
}

fn waiter_row() -> Value { rk::staff_row("p1", "v1", "waiter", "laptop", &hash_opaque("good"), NOW) }
fn courier_row() -> Value { rk::courier_row("c1", "v1", "k1", "phone", &hash_opaque("good"), NOW) }

#[test]
fn a_staff_key_becomes_a_five_minute_token_of_its_own_person() {
    let later = NOW + 1000;
    let c = rk::verdict(Holder::Staff, "k1", Some(&waiter_row()), "good", later).unwrap();
    assert_eq!(c, Claims::Staff { sub: "p1".into(), active_location_id: "v1".into(), jti: "k1".into(),
        caps: Preset::Waiter.caps().to_string(), iat: later, exp: later + rk::TOOL_TOKEN_TTL_MS });
    // Never past the key's own expiry.
    let edge = NOW + rk::KEY_TTL_MS - 10;
    assert_eq!(rk::verdict(Holder::Staff, "k1", Some(&waiter_row()), "good", edge).unwrap().exp(), NOW + rk::KEY_TTL_MS);
}

#[test]
fn a_courier_key_names_only_the_courier_in_its_row() {
    let c = rk::verdict(Holder::Courier, "k1", Some(&courier_row()), "good", NOW).unwrap();
    assert_eq!(c, Claims::Courier { sub: "c1".into(), active_location_id: "v1".into(), jti: "k1".into(), iat: NOW, exp: NOW + rk::TOOL_TOKEN_TTL_MS });
    // Another courier's row with this courier's secret is no key at all.
    let theirs = rk::courier_row("c2", "v1", "k2", "phone", &hash_opaque("theirs"), NOW);
    assert_eq!(rk::verdict(Holder::Courier, "k2", Some(&theirs), "good", NOW), Err("no such key"));
}

#[test]
fn a_key_is_refused_when_wrong_revoked_expired_or_not_a_key() {
    let row = waiter_row();
    assert_eq!(rk::verdict(Holder::Staff, "k1", None, "good", NOW), Err("no such key"));
    assert_eq!(rk::verdict(Holder::Staff, "k1", Some(&row), "bad", NOW), Err("no such key"));
    let mut gone = row.clone();
    gone["revoked_at_ms"] = json!(NOW);
    assert_eq!(rk::verdict(Holder::Staff, "k1", Some(&gone), "good", NOW), Err("that key was revoked"));
    assert_eq!(rk::verdict(Holder::Staff, "k1", Some(&row), "good", NOW + rk::KEY_TTL_MS), Err("that key has expired"));
    // A SIGN-IN session row is not a key, whatever id and secret are sent.
    let signin = crate::services::identity::staff_rules::session_record("p1", "v1", Preset::Waiter, NOW);
    assert_eq!(rk::verdict(Holder::Staff, "k1", Some(&signin), "", NOW), Err("no such key"));
    let mut courier_word = row.clone();
    courier_word["role"] = json!("courier");
    assert_eq!(rk::verdict(Holder::Staff, "k1", Some(&courier_word), "good", NOW), Err("that key names no staff role"));
    let mut nobody = row;
    nobody["person_id"] = json!("");
    assert_eq!(rk::verdict(Holder::Staff, "k1", Some(&nobody), "good", NOW), Err("no such key"));
}

#[test]
fn a_courier_key_row_is_never_a_refresh_token() {
    let row = courier_row();
    assert_eq!((row["token_hash"].as_str(), row["family_id"].as_str()), (Some(rk::NOT_A_REFRESH), Some("k1")));
    assert!(!verify_opaque("good", rk::NOT_A_REFRESH) && !verify_opaque("", rk::NOT_A_REFRESH));
    // Its twin: the key's own hash does verify the key's secret.
    assert!(verify_opaque("good", row["mcp_key_hash"].as_str().unwrap()));
}

#[test]
fn a_key_is_shown_without_its_hash_only_while_it_works() {
    let s = rk::shown(Holder::Staff, "k1", &waiter_row(), NOW).unwrap();
    assert_eq!(s, json!({ "id": "k1", "holder": "staff", "person": "p1", "label": "laptop", "role": "waiter",
        "createdMs": NOW, "expiresMs": NOW + rk::KEY_TTL_MS }));
    assert_eq!(rk::shown(Holder::Courier, "k1", &courier_row(), NOW).unwrap()["role"], "courier");
    let mut gone = waiter_row();
    gone["revoked_at_ms"] = json!(NOW);
    assert!(rk::shown(Holder::Staff, "k1", &gone, NOW).is_none());
    assert!(rk::shown(Holder::Staff, "k1", &waiter_row(), NOW + rk::KEY_TTL_MS).is_none());
    let signin = crate::services::identity::staff_rules::session_record("p1", "v1", Preset::Waiter, NOW);
    assert!(rk::shown(Holder::Staff, "s1", &signin, NOW).is_none());
}

#[test]
fn only_the_person_or_their_venues_owner_may_revoke() {
    let (w, c) = (waiter_row(), courier_row());
    assert!(rk::may_revoke_own(Holder::Staff, Some(&w), "p1", "v1"));
    assert!(!rk::may_revoke_own(Holder::Staff, Some(&w), "p2", "v1"));
    assert!(!rk::may_revoke_own(Holder::Staff, Some(&w), "p1", "v2"));
    assert!(rk::may_revoke_own(Holder::Courier, Some(&c), "c1", "v1"));
    assert!(!rk::may_revoke_own(Holder::Courier, Some(&c), "c2", "v1"));
    assert!(!rk::may_revoke_own(Holder::Courier, None, "c1", "v1"));
    let signin = crate::services::identity::staff_rules::session_record("p1", "v1", Preset::Waiter, NOW);
    assert!(!rk::may_revoke_own(Holder::Staff, Some(&signin), "p1", "v1"));
    let mut gone = w.clone();
    gone["revoked_at_ms"] = json!(NOW);
    assert!(!rk::may_revoke_own(Holder::Staff, Some(&gone), "p1", "v1"));
    assert!(rk::may_revoke_at(Holder::Staff, Some(&w), "v1"));
    assert!(rk::may_revoke_at(Holder::Courier, Some(&c), "v1"));
    assert!(!rk::may_revoke_at(Holder::Courier, Some(&c), "v2"));
    assert!(!rk::may_revoke_at(Holder::Staff, Some(&signin), "v1"));
}

#[test]
fn a_key_needs_a_label_someone_can_decide_about() {
    assert_eq!(rk::label_of("  laptop "), Ok("laptop".into()));
    assert_eq!(rk::label_of("   "), Err("say what this key is for"));
    assert_eq!(rk::label_of(&"x".repeat(rk::LABEL_MAX_CHARS + 1)), Err("say what this key is for"));
    assert!(rk::label_of(&"x".repeat(rk::LABEL_MAX_CHARS)).is_ok());
}
