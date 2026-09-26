//! The capability law, proved. Every one of these failed before `caps.rs` had
//! an implementation (52 compile errors, `cargo test --lib caps`, exit 101).

use super::*;

/// DENY BY DEFAULT. The empty set grants nothing, and that is the state a
/// capability starts in — not "everything until somebody takes it away".
#[test]
fn a_capability_not_granted_is_refused() {
    let none = Caps::none();
    for c in Cap::ALL {
        assert!(!none.allows(c), "the empty set granted {}", c.as_str());
    }
    assert!(none.is_empty());
    assert_eq!(Caps::default(), Caps::none(), "the default must be the empty set, not a full one");
}

/// The four ruled staff words and their capability sets (BLUEPRINT-POS-THE-ROOM §2.8).
#[test]
fn the_four_ruled_presets_are_exactly_their_cap_sets() {
    let kitchen = Preset::Kitchen.caps();
    assert!(kitchen.allows(Cap::Advance));
    for c in [Cap::TakeOrders, Cap::TakePayment, Cap::Void, Cap::OpenTill] {
        assert!(!kitchen.allows(c), "kitchen must not hold {}", c.as_str());
    }

    let waiter = Preset::Waiter.caps();
    assert!(waiter.allows(Cap::TakeOrders));
    assert!(waiter.allows(Cap::TakePayment));
    for c in [Cap::Advance, Cap::Void, Cap::OpenTill] {
        assert!(!waiter.allows(c), "waiter must not hold {}", c.as_str());
    }

    let counter = Preset::CounterManager.caps();
    for c in [Cap::TakeOrders, Cap::TakePayment, Cap::Void, Cap::OpenTill] {
        assert!(counter.allows(c), "counter-manager must hold {}", c.as_str());
    }
    assert!(
        !counter.allows(Cap::Advance),
        "moving a round through the kitchen is the kitchen's capability"
    );

    for c in Cap::ALL {
        assert!(Preset::Owner.caps().allows(c), "the owner holds {}", c.as_str());
    }
}

/// UNKNOWN WORDS ARE REFUSED. A roster row spelling a word the operator has
/// not ruled grants nothing at all — including `courier`, which is a word the
/// roster already writes and which is not a staff preset.
#[test]
fn a_roster_word_the_operator_has_not_ruled_grants_nothing() {
    for word in ["server", "courier", "counter", "kitchen-manager", "", "OWNER"] {
        assert_eq!(Preset::from_str(word), None, "{word:?} is not a ruled preset");
    }
    assert_eq!(Preset::from_str("owner"), Some(Preset::Owner));
    assert_eq!(Preset::from_str("kitchen"), Some(Preset::Kitchen));
    assert_eq!(Preset::from_str("waiter"), Some(Preset::Waiter));
    assert_eq!(Preset::from_str("counter-manager"), Some(Preset::CounterManager));
}

/// A NAME THE SET DOES NOT HOLD REFUSES THE WHOLE TOKEN. Dropping the unknown
/// one and honouring the rest is how a typo becomes a silent downgrade — and
/// how a capability minted by a newer build is quietly discarded by an older
/// one.
#[test]
fn an_unknown_capability_name_refuses_the_whole_set() {
    assert_eq!(Caps::parse("advance,teleport"), None);
    assert_eq!(Caps::parse("teleport"), None);
    assert_eq!(Caps::parse("Advance"), None);
    assert_eq!(Caps::parse("advance;void"), None);
    // The empty string is not an unknown name; it is no capabilities, which is
    // a set this type can hold and which `admit` refuses on its own terms.
    assert_eq!(Caps::parse(""), Some(Caps::none()));
}

#[test]
fn caps_round_trip_through_their_canonical_spelling() {
    let all = Caps::of(&Cap::ALL);
    assert_eq!(Caps::parse(&all.to_string()), Some(all));
    let some = Caps::of(&[Cap::TakePayment, Cap::TakeOrders]);
    // Canonical ORDER, not the order they were named in: two spellings of one
    // set would be two tokens that are not byte-comparable.
    assert_eq!(some.to_string(), "take_orders,take_payment");
    assert_eq!(Caps::parse("take_payment,take_orders"), Some(some));
    assert_eq!(Caps::none().to_string(), "");
    for c in Cap::ALL {
        assert_eq!(Cap::from_str(c.as_str()), Some(c));
    }
    assert_eq!(Cap::from_str("open-till"), None);
}

/// Each capability owns its own bit. A collision would make two rights one
/// right, silently, and every test above would still pass.
#[test]
fn no_two_capabilities_share_a_bit() {
    for c in Cap::ALL {
        let only = Caps::of(&[c]);
        for other in Cap::ALL {
            assert_eq!(
                only.allows(other),
                other == c,
                "{} leaked into {}",
                c.as_str(),
                other.as_str()
            );
        }
    }
}

/// AUTHORITY IS RE-DERIVED, NEVER TRUSTED FROM THE TOKEN — `auth.rs`'s own
/// rule 1, applied to capabilities. The token says what was granted when it was
/// minted; the roster says what the person still is. The signer gets the
/// intersection, so a capability taken away on the roster a second ago is gone
/// on the next call instead of at token expiry.
#[test]
fn admit_narrows_the_token_to_what_the_roster_still_grants() {
    assert_eq!(admit("advance", "kitchen"), Some(Caps::of(&[Cap::Advance])));

    // Minted with the till, moved to the kitchen since: nothing survives, so
    // there is no principal at all.
    assert_eq!(admit("open_till", "kitchen"), None);

    // Minted with more than the roster grants: the extra is stripped and the
    // rest stands.
    assert_eq!(
        admit("advance,open_till", "kitchen"),
        Some(Caps::of(&[Cap::Advance])),
        "the roster is the ceiling, not the token"
    );

    assert_eq!(
        admit("take_orders,void", "counter-manager"),
        Some(Caps::of(&[Cap::TakeOrders, Cap::Void]))
    );

    // A token cannot widen itself past the roster in the other direction
    // either: every capability in the answer is one BOTH sides named.
    let got = admit("advance,take_orders,take_payment,void,open_till", "counter-manager")
        .expect("counter-manager holds four of the five");
    assert_eq!(got, Preset::CounterManager.caps());
    assert!(!got.allows(Cap::Advance));
}

/// A token minted for a capability the set does not name is refused even when
/// the roster word is perfectly good — the parse fails before the roster is
/// consulted at all.
#[test]
fn admit_refuses_an_unknown_capability_before_it_reads_the_roster() {
    assert_eq!(admit("teleport", "owner"), None);
    assert_eq!(admit("advance,teleport", "owner"), None);
}

/// THE REFUSAL THIS WHOLE MODULE EXISTS FOR. A signed, in-date, correctly
/// scoped token that names no capability is not a signer. Delete the emptiness
/// check in `admit` and this test fails.
#[test]
fn a_token_without_a_capability_is_refused() {
    assert_eq!(admit("", "counter-manager"), None);
    assert_eq!(admit("", "owner"), None);
    assert_eq!(admit("", "kitchen"), None);
    assert_eq!(admit("", ""), None);
    // A real capability against an unruled word is still a refusal.
    assert_eq!(admit("take_orders", "server"), None);
}

/// A waiter's capabilities are take_orders and take_payment, exactly.
/// A token minted with void for a waiter role narrows to nothing (intersection).
#[test]
fn admit_with_a_token_minted_with_void_for_a_roster_role_waiter_narrows_it_away() {
    // Waiter is a valid preset that grants take_orders and take_payment.
    assert_eq!(
        admit("take_orders,take_payment", "waiter"),
        Some(Preset::Waiter.caps())
    );
    // A token minted with void (which a waiter does not have) narrows to nothing.
    assert_eq!(admit("void", "waiter"), None);
    // A token minted with Advance (which a waiter does not have) narrows to nothing.
    assert_eq!(admit("advance", "waiter"), None);
}

/// OPERATOR Q8 (2026-09-26): a kitchen login manages the menu and the shelf.
/// The kitchen preset holds `catalog` and `stock` beside `advance`, and still
/// nothing that touches money or the room.
#[test]
fn the_kitchen_holds_the_menu_and_the_shelf() {
    let kitchen = Preset::Kitchen.caps();
    for c in [Cap::Advance, Cap::Catalog, Cap::Stock] {
        assert!(kitchen.allows(c), "kitchen must hold {}", c.as_str());
    }
    for c in [Cap::TakeOrders, Cap::TakePayment, Cap::Void, Cap::OpenTill] {
        assert!(!kitchen.allows(c), "kitchen must not hold {}", c.as_str());
    }
    assert_eq!(kitchen.to_string(), "advance,catalog,stock");
    assert_eq!(
        admit("advance,catalog,stock", "kitchen"),
        Some(Caps::of(&[Cap::Advance, Cap::Catalog, Cap::Stock]))
    );
}

/// THE TWIN: a waiter and a counter-manager hold neither new word, and a token
/// minted with them is narrowed away by the roster.
#[test]
fn a_waiter_is_refused_the_menu_and_the_shelf() {
    for p in [Preset::Waiter, Preset::CounterManager] {
        let caps = p.caps();
        assert!(!caps.allows(Cap::Catalog), "{} must not hold catalog", p.as_str());
        assert!(!caps.allows(Cap::Stock), "{} must not hold stock", p.as_str());
    }
    assert_eq!(admit("catalog,stock", "waiter"), None, "nothing survives: no principal");
    assert_eq!(
        admit("take_orders,catalog,stock", "waiter"),
        Some(Caps::of(&[Cap::TakeOrders])),
        "the roster is the ceiling"
    );
    assert!(Preset::Owner.caps().allows(Cap::Catalog) && Preset::Owner.caps().allows(Cap::Stock));
    assert_eq!(Cap::from_str("catalog"), Some(Cap::Catalog));
    assert_eq!(Cap::from_str("stock"), Some(Cap::Stock));
}
