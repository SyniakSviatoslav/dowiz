//! The confirmation: the token's own instruction back, for its own speaker
//! and role only.

use super::*;
use dowiz_hub::token::{Claims, Role};

fn claims(subject: &str, role: Role, scope: &str) -> Claims {
    Claims {
        role,
        subject: subject.into(),
        session: String::new(),
        scope: scope.into(),
        caps: String::new(),
        issued_ms: 0,
        expires_ms: 1,
    }
}

#[test]
fn a_hub_verb_comes_back_as_the_order_it_named() {
    let out = confirmed(&claims("u1", Role::Owner, "voice:confirm:order-9"), "u1", Role::Owner);
    assert_eq!(out, json!({ "understood": true, "needsConfirmation": false, "action": "do", "verb": "confirm", "orderId": "order-9" }));
}

#[test]
fn a_room_verb_comes_back_with_its_arguments() {
    let out = confirmed(&claims("w1", Role::Staff, "voice:pay:r1|3|1100|card"), "w1", Role::Staff);
    assert_eq!(out["verb"], "pay");
    assert_eq!(out["args"], json!({ "orderId": "r1", "baseSeq": 3, "amount": 1100, "method": "card" }));
    assert!(out.get("orderId").is_none());
    let place = confirmed(&claims("w1", Role::Staff, "voice:place:5|marg*2"), "w1", Role::Staff);
    assert_eq!(place["args"]["items"][0], json!({ "product_id": "marg", "quantity": 2 }));
}

/// Somebody else's read-back, or the same person's in another role, is not
/// a confirmation -- and neither is a scope that is not a voice proposal.
#[test]
fn a_confirmation_that_is_not_yours_or_not_whole_is_refused() {
    let theirs = confirmed(&claims("w2", Role::Staff, "voice:pay:r1|3|1100|card"), "w1", Role::Staff);
    assert_eq!(theirs["understood"], false);
    let other_role = confirmed(&claims("u1", Role::Courier, "voice:pickup:o1"), "u1", Role::Owner);
    assert_eq!(other_role["understood"], false);
    for scope in ["order:o1", "voice:nothing", "voice:pay:r1|x|1|cash", "voice:place:5"] {
        let out = confirmed(&claims("u1", Role::Owner, scope), "u1", Role::Owner);
        assert_eq!(out["understood"], false, "{scope}");
    }
}
