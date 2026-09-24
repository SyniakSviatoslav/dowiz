//! Where an order came from, over nothing but a string. Every refusal has its
//! positive twin beside it.

use super::*;
use serde_json::json;

/// THE SET IS CLOSED, and the near misses are misses.
#[test]
fn a_source_nobody_has_heard_of_is_not_a_source() {
    for c in ALL {
        assert!(known(c), "{c} is a member");
    }
    for c in ["web", "app", "fax", "Wolt", "uber_eats", "bolt_food", "STOREFRONT", ""] {
        assert!(!known(c), "{c} is not a member");
    }
}

/// THE WORDS ARE THE BLUEPRINT'S, IN ITS ORDER (§3.6). A rename here is a
/// migration of every stored order, not an edit.
#[test]
fn the_set_is_the_blueprints_words_plus_the_three_marketplaces_of_2_9() {
    assert_eq!(ALL, ["storefront", "console", "whatsapp", "instagram", "ebills", "wolt", "glovo", "baboon"]);
}

/// AN ORDER FROM BEFORE THE FIELD IS A STOREFRONT ORDER — absent and the
/// kernel's explicit `null` alike (the compatibility rule).
#[test]
fn an_order_from_before_this_existed_is_storefront() {
    assert_eq!(of(&json!({ "id": "old", "status": "PICKED_UP" })), Ok(STOREFRONT));
    assert_eq!(of(&json!({ "channel": null })), Ok(STOREFRONT));
}

/// A MEMBER IS READ AS ITSELF — the twin of every refusal below.
#[test]
fn a_member_is_read_as_itself() {
    for c in ALL {
        assert_eq!(of(&json!({ "channel": c })), Ok(c));
    }
}

/// AN UNKNOWN WORD IS REFUSED, NOT FILED AS STOREFRONT. This is the rule the
/// Haiku draft broke: `of` defaulted `"banana"` to `"storefront"`, which would
/// have given an unrecognised source first-party trust.
#[test]
fn an_unknown_word_is_refused_not_defaulted() {
    assert_eq!(of(&json!({ "channel": "banana" })), Err(Unknown("banana".into())));
    assert_eq!(of(&json!({ "channel": "web" })), Err(Unknown("web".into())));
    assert!(of(&json!({ "channel": 7 })).is_err(), "a number is not a channel");
    assert!(of(&json!({ "channel": { "x": 1 } })).is_err());
}

/// ONLY A MEMBER HAS A PROFILE. There is no default trust model.
#[test]
fn only_a_member_has_a_profile() {
    for c in ALL {
        assert!(profile(c).is_some(), "{c}");
    }
    assert_eq!(profile("uber_eats"), None);
    assert_eq!(profile("banana"), None);
}

/// THE FIRST-PARTY SOURCES are priced, fiscalised and owned by the venue and
/// pay nobody a commission.
#[test]
fn first_party_sources_are_ours_on_every_axis() {
    for c in [STOREFRONT, CONSOLE, WHATSAPP, INSTAGRAM] {
        let p = profile(c).unwrap();
        assert!(p.first_party && p.priced_by_us && p.fiscalised_by_us && p.owns_customer, "{c}");
        assert_eq!(p.commission_ppm, 0, "{c}");
    }
}

/// AN EBILLS IMPORT is the venue's own till: its guest, no commission — but
/// the till priced it and already fiscalised it, so dowiz must neither
/// re-price nor re-fiscalise it (`price_trusted: false`).
#[test]
fn an_ebills_import_was_priced_and_fiscalised_elsewhere() {
    let p = profile(EBILLS).unwrap();
    assert!(!p.priced_by_us && !p.fiscalised_by_us);
    assert!(p.first_party && p.owns_customer);
    assert_eq!(p.commission_ppm, 0);
}

/// THE PRINCIPAL DECIDES: a staff-signed round is `console`, a guest is
/// `storefront`.
#[test]
fn the_principal_decides_the_source() {
    assert_eq!(for_placement(true), CONSOLE);
    assert_eq!(for_placement(false), STOREFRONT);
}

/// A CHANNEL IN THE BODY IS DISCARDED: `stamp` overwrites it with the
/// handler's word, and what it wrote reads back through `of`.
#[test]
fn stamp_overwrites_what_the_request_said() {
    let mut env = json!({ "id": "o1", "channel": "fax" });
    assert_eq!(stamp(&mut env, STOREFRONT), Ok(()));
    assert_eq!(env["channel"], json!("storefront"));
    assert_eq!(of(&env), Ok(STOREFRONT));

    let mut env = json!({ "id": "o2" });
    assert_eq!(stamp(&mut env, CONSOLE), Ok(()));
    assert_eq!(of(&env), Ok(CONSOLE));
}

/// `stamp` NEVER WRITES A NON-MEMBER, and leaves the envelope as it found it.
#[test]
fn stamp_refuses_a_word_outside_the_set() {
    let mut env = json!({ "id": "o1", "channel": "storefront" });
    assert_eq!(stamp(&mut env, "fax"), Err(Unknown("fax".into())));
    assert_eq!(env["channel"], json!("storefront"), "untouched on refusal");
    assert!(stamp(&mut json!([1, 2]), STOREFRONT).is_err(), "not an object");
}

/// §2.9: a marketplace is somebody else's customer, priced and carried by them.
#[test]
fn a_marketplace_is_priced_and_delivered_by_the_platform() {
    for c in [WOLT, GLOVO, BABOON] {
        let p = profile(c).expect("a member");
        assert!(marketplace(c));
        assert!(!p.first_party && !p.priced_by_us && !p.owns_customer && !p.delivered_by_us, "{c}");
    }
    // Twin: no first-party source and not the venue's own till is a marketplace.
    for c in [STOREFRONT, CONSOLE, WHATSAPP, INSTAGRAM, EBILLS] {
        assert!(!marketplace(c), "{c}");
        assert!(profile(c).unwrap().delivered_by_us);
    }
    assert!(!marketplace("uber_eats"), "not a member, not a marketplace");
}

/// The aggregator form's body picks a platform; what is stamped is the SET's
/// word, and only a marketplace's. Every first-party word, a case variant and
/// a stranger get nothing.
#[test]
fn a_body_can_name_only_a_marketplace_and_gets_the_sets_word() {
    assert_eq!(marketplace_word("wolt"), Some(WOLT));
    assert_eq!(marketplace_word("glovo"), Some(GLOVO));
    assert_eq!(marketplace_word("baboon"), Some(BABOON));
    for c in [STOREFRONT, CONSOLE, WHATSAPP, INSTAGRAM, EBILLS, "Wolt", "wolt ", "uber_eats", ""] {
        assert_eq!(marketplace_word(c), None, "{c:?}");
    }
}
