//! The key listings, over a real session table written the way `mint`
//! writes it: a person sees only their own keys, the owner sees every key of
//! their venue, and nobody sees another venue's -- or a sign-in row.

use super::*;
use dowiz_hub::table::Table;

const NOW: i64 = 1_800_000_000_000;

fn put(t: &mut Table, h: Holder, id: &str, row: Value) {
    let index = index_of(h, &row, id);
    t.put(h.kind(), id, &row.to_string(), &index, &[]).unwrap();
}

fn ids_of(v: &[Value]) -> Vec<String> {
    let mut out: Vec<String> = v.iter().map(|k| k["id"].as_str().unwrap().to_string()).collect();
    out.sort();
    out
}

fn table() -> Table {
    let mut t = Table::create(1 << 16).unwrap();
    put(&mut t, Holder::Staff, "s1", rk::staff_row("ana", "v1", "waiter", "tablet", "h", NOW));
    put(&mut t, Holder::Staff, "s2", rk::staff_row("ben", "v1", "kitchen", "screen", "h", NOW));
    put(&mut t, Holder::Staff, "s3", rk::staff_row("ana", "v2", "waiter", "elsewhere", "h", NOW));
    put(&mut t, Holder::Courier, "c1", rk::courier_row("cora", "v1", "c1", "phone", "h", NOW));
    put(&mut t, Holder::Courier, "c2", rk::courier_row("dan", "v1", "c2", "phone", "h", NOW));
    put(&mut t, Holder::Courier, "c3", rk::courier_row("cora", "v2", "c3", "phone", "h", NOW));
    // A courier SIGN-IN row (no key hash) and a revoked key: never listed.
    let signin = json!({ "courier_id": "cora", "active_location_id": "v1", "expires_at_ms": NOW + 1000, "revoked_at_ms": null });
    t.put(Holder::Courier.kind(), "c4", &signin.to_string(), &[], &[]).unwrap();
    let mut gone = rk::courier_row("cora", "v1", "c5", "old", "h", NOW);
    gone["revoked_at_ms"] = json!(NOW);
    put(&mut t, Holder::Courier, "c5", gone);
    t
}

#[test]
fn a_courier_lists_only_their_own_keys_here_and_the_owner_every_courier_key_of_the_venue() {
    let t = table();
    assert_eq!(ids_of(&keys_in(&t, Holder::Courier, "v1", Some("cora"), NOW)), ["c1"]);
    assert_eq!(ids_of(&keys_in(&t, Holder::Courier, "v1", Some("dan"), NOW)), ["c2"]);
    assert_eq!(ids_of(&keys_in(&t, Holder::Courier, "v1", None, NOW)), ["c1", "c2"]);
    assert_eq!(ids_of(&keys_in(&t, Holder::Courier, "v2", None, NOW)), ["c3"]);
    let shown = keys_in(&t, Holder::Courier, "v1", Some("cora"), NOW);
    assert!(shown[0].get("mcp_key_hash").is_none() && shown[0]["label"] == "phone");
}

#[test]
fn a_member_of_staff_lists_only_their_own_keys_here_and_the_owner_every_staff_key_of_the_venue() {
    let t = table();
    assert_eq!(ids_of(&keys_in(&t, Holder::Staff, "v1", Some("ana"), NOW)), ["s1"]);
    assert_eq!(ids_of(&keys_in(&t, Holder::Staff, "v1", Some("ben"), NOW)), ["s2"]);
    assert_eq!(ids_of(&keys_in(&t, Holder::Staff, "v1", None, NOW)), ["s1", "s2"]);
    assert_eq!(ids_of(&keys_in(&t, Holder::Staff, "v2", Some("ana"), NOW)), ["s3"]);
    // Expired keys drop off the list.
    assert!(keys_in(&t, Holder::Staff, "v1", None, NOW + rk::KEY_TTL_MS).is_empty());
}

#[test]
fn a_staff_key_is_indexed_under_its_persons_sessions_and_a_courier_key_under_nothing() {
    let row = rk::staff_row("ana", "v1", "waiter", "t", "h", NOW);
    assert_eq!(index_of(Holder::Staff, &row, "s1"), vec![(sr::ssession_of("v1", "ana", "s1"), "s1".to_string())]);
    assert!(index_of(Holder::Courier, &rk::courier_row("c", "v1", "c1", "t", "h", NOW), "c1").is_empty());
}
