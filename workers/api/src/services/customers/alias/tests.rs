//! §3.4's refusals, each beside the positive twin that proves the refusal is
//! not the only thing the rule can do. Every test runs the real `Table`.

use super::*;
use crate::auth::Claims;

const A: &str = "aaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbb";
const C: &str = "cccccccccccccccc";

fn table() -> Table {
    Table::create(64 * 1024).unwrap()
}

fn by_of(t: &Table, from: &str) -> String {
    let v: Value = serde_json::from_str(&t.get(KIND, from).unwrap()).unwrap();
    v["by"].as_str().unwrap().to_string()
}

/// REFUSAL: staff, a courier and a customer may not link people. TWIN: the
/// owner may (and `owner_and_venue` then proves which venue).
#[test]
fn only_an_owner_links_and_an_owner_does() {
    let staff = Claims::Staff { sub: "s".into(), active_location_id: "v".into(), jti: "j".into(), caps: "void".into(), iat: 0, exp: 1 };
    let courier = Claims::Courier { sub: "c".into(), active_location_id: "v".into(), jti: "j".into(), iat: 0, exp: 1 };
    let customer = Claims::Customer { sub: "u".into(), order_id: "o".into(), location_id: "v".into(), iat: 0, exp: 1 };
    for c in [&staff, &courier, &customer] {
        assert!(!may_link(c), "{c:?} must be refused");
    }
    let owner = Claims::Owner { sub: "o".into(), user_id: "own_1".into(), active_location_id: Some("v".into()), iat: 0, exp: 1 };
    assert!(may_link(&owner));
}

/// REFUSAL: a customer linked to themselves. TWIN: two distinct customers.
#[test]
fn a_link_to_self_is_refused_and_a_distinct_pair_is_linked() {
    let mut t = table();
    assert!(link(&mut t, A, A, By::Owner, "same", 1).unwrap_err().contains("themselves"));
    assert!(!t.has(KIND, A), "a refusal writes nothing");
    link(&mut t, A, B, By::Owner, "same person, two phones", 1).expect("distinct pair");
    let v: Value = serde_json::from_str(&t.get(KIND, A).unwrap()).unwrap();
    assert_eq!(v, json!({ "canonical": B, "by": "owner", "at_ms": 1, "reason": "same person, two phones" }));
}

/// REFUSAL: a key already linked is not re-pointed silently. TWIN: after an
/// unlink it can be linked again.
#[test]
fn a_linked_key_must_be_unlinked_before_it_is_linked_again() {
    let mut t = table();
    link(&mut t, A, B, By::Owner, "x", 1).unwrap();
    assert!(link(&mut t, A, C, By::Owner, "x", 2).unwrap_err().contains("already linked"));
    assert!(unlink(&mut t, A));
    assert!(!unlink(&mut t, A), "a second unlink finds nothing");
    link(&mut t, A, C, By::Owner, "x", 3).expect("free again");
}

/// REFUSAL: B -> A after A -> B would be a cycle. TWIN: B -> C is a chain,
/// and A resolves through it.
#[test]
fn a_cycle_is_refused_and_a_chain_resolves_to_its_end() {
    let mut t = table();
    link(&mut t, A, B, By::Owner, "x", 1).unwrap();
    assert!(link(&mut t, B, A, By::Owner, "x", 2).unwrap_err().contains("already one"));
    link(&mut t, B, C, By::Owner, "x", 3).expect("a chain");
    let all = Aliases::of(&t);
    assert_eq!(all.resolve(A), C);
    assert_eq!(all.resolve(B), C);
    assert_eq!(all.resolve(C), C);
    assert_eq!(all.members(C), vec![A.to_string(), B.to_string()]);
    assert!(all.members(A).is_empty());
}

/// REFUSAL: a chain may not grow past `MAX_HOPS`. TWIN: one shorter may.
#[test]
fn a_chain_longer_than_max_hops_is_refused() {
    let mut t = table();
    let keys: Vec<String> = (0..=MAX_HOPS + 1).map(|i| format!("{i:016x}")).collect();
    for w in keys.windows(2).take(MAX_HOPS) {
        link(&mut t, &w[1], &w[0], By::Owner, "x", 1).expect("within the bound");
    }
    let tip = &keys[MAX_HOPS + 1];
    assert!(link(&mut t, tip, &keys[MAX_HOPS], By::Owner, "x", 1).unwrap_err().contains("at most"));
}

/// A CORRUPTED IMAGE CANNOT SPIN A REQUEST: a cycle written behind the rules'
/// back still resolves, in at most `MAX_HOPS` steps.
#[test]
fn resolve_terminates_on_a_cycle_it_did_not_write() {
    let mut t = table();
    t.put(KIND, A, &json!({ "canonical": B }).to_string(), &[], &[]).unwrap();
    t.put(KIND, B, &json!({ "canonical": A }).to_string(), &[], &[]).unwrap();
    let r = Aliases::of(&t).resolve(A);
    assert!(r == A || r == B);
}

/// REFUSAL: the rule never overwrites an owner's link, and never relinks a
/// spelling that has ordered before (its card exists). TWIN: a first
/// appearance is linked, by "rule".
#[test]
fn the_rule_links_a_first_appearance_only() {
    let mut t = table();
    assert!(rule_link(&mut t, A, B, 1), "first appearance");
    assert_eq!(by_of(&t, A), "rule");

    let mut t = table();
    link(&mut t, A, C, By::Owner, "x", 1).unwrap();
    assert!(!rule_link(&mut t, A, B, 2), "an owner's link stands");
    assert_eq!(by_of(&t, A), "owner");

    let mut t = table();
    t.put(super::super::record::KIND, A, "{}", &[], &[]).unwrap();
    assert!(!rule_link(&mut t, A, B, 3), "an old spelling is not a first appearance");
    assert!(!t.has(KIND, A));
}

/// The `Linked` audit: one entry per person, each a `Revealed`-shaped payload
/// (`by`, `at`, `reason`) naming the act and the other key.
#[test]
fn a_link_is_audited_on_both_people() {
    let [(s1, e1), (s2, e2)] = audit("linked", "own_1", 7, "same phone", A, B);
    assert_eq!(s1, format!("cust:{A}"));
    assert_eq!(s2, format!("cust:{B}"));
    let (v1, v2): (Value, Value) = (serde_json::from_str(&e1).unwrap(), serde_json::from_str(&e2).unwrap());
    assert_eq!(v1, json!({ "by": "own_1", "at": 7, "reason": "same phone", "act": "linked", "with": B }));
    assert_eq!(v2["with"], A);
}

fn card(t: &mut Table, key: &str, allergens: &[&str]) {
    let v = json!({ "allergens": allergens });
    t.put(super::super::record::KIND, key, &v.to_string(), &[], &[]).unwrap();
}

/// AN ALLERGY ON ONE SPELLING'S CARD BINDS EVERY LINKED SPELLING, including a
/// spelling's FIRST order, whose link is only `pending`. REFUSAL TWIN: an
/// unlinked key reads only its own card.
#[test]
fn a_linked_persons_allergies_are_the_union_of_their_cards() {
    let mut t = table();
    card(&mut t, B, &["fish"]);
    card(&mut t, A, &["milk"]);
    assert_eq!(allergens_of(&t, A, None), vec!["milk".to_string()], "not linked: own card only");
    assert_eq!(allergens_of(&t, C, Some(B)), vec!["fish".to_string()], "first order: the pending link binds");
    link(&mut t, A, B, By::Owner, "x", 1).unwrap();
    let mut got = allergens_of(&t, A, None);
    got.sort();
    assert_eq!(got, vec!["fish".to_string(), "milk".to_string()]);
    assert_eq!(allergens_of(&t, B, None).len(), 2, "and read from the canonical side too");
    assert!(unlink(&mut t, A));
    assert_eq!(allergens_of(&t, B, None), vec!["fish".to_string()], "unlinked: apart again");
}

/// A `pending` link never overrides a link that is already there: the rule
/// never overwrites (`rule_link`), so the circle follows the written one.
#[test]
fn a_pending_link_does_not_override_a_written_one() {
    let mut t = table();
    link(&mut t, A, C, By::Owner, "x", 1).unwrap();
    assert_eq!(Aliases::of(&t).circle(A, Some(B)), vec![C.to_string(), A.to_string()]);
    assert_eq!(Aliases::of(&t).circle(B, None), vec![B.to_string()]);
}
