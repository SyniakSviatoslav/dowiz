//! The registry's own rules. The gate (`tools/gates/personal-data.sh`) proves
//! every store, host and browser key has a row; these prove every row says
//! something a reader can act on.

use super::*;

/// P1's CHECK: a store kept on purpose must say why, and a store nothing
/// erases yet must name the roadmap row that will.
#[test]
fn a_retained_store_carries_its_reason_and_a_missing_eraser_names_its_row() {
    for s in stores() {
        match s.erase {
            Eraser::Retain(why) => assert!(why.trim().len() >= 20, "{}: Retain needs a real reason, got {why:?}", s.image),
            Eraser::Missing(row) => assert!(
                row.starts_with('P') && row[1..].parse::<u32>().is_ok(),
                "{}: Missing must name a roadmap row like \"P2\", got {row:?}", s.image
            ),
            Eraser::Redact(f) | Eraser::Remove(f) | Eraser::Expires(f) | Eraser::NotPersonal(f) => {
                assert!(!f.trim().is_empty(), "{}: the eraser must name what does it", s.image)
            }
        }
    }
}

/// The rule the test above enforces, shown refusing: an empty reason is caught
/// by the same predicate the walk uses (the positive twin is every real row).
#[test]
fn an_empty_retain_reason_is_what_the_rule_refuses() {
    let bad = Eraser::Retain("  ");
    let refused = matches!(bad, Eraser::Retain(why) if why.trim().len() < 20);
    assert!(refused);
    let good = Eraser::Retain("fiscal documents are a legal obligation under the tax law");
    assert!(!matches!(good, Eraser::Retain(why) if why.trim().len() < 20));
}

#[test]
fn every_image_has_exactly_one_row() {
    let mut seen: Vec<&str> = Vec::new();
    for s in stores() {
        assert!(!seen.contains(&s.image), "two rows for image {:?}", s.image);
        seen.push(s.image);
    }
}

/// A personal store says whose it is and what it holds; a store marked not
/// personal holds nothing.
#[test]
fn personal_rows_name_their_subjects_and_non_personal_rows_hold_nothing() {
    for s in stores() {
        if s.basis == Basis::NotPersonal {
            assert!(s.holds.is_empty() && s.subjects.is_empty(), "{} is NotPersonal but lists data", s.image);
            assert!(!s.personal());
        } else {
            assert!(s.personal(), "{} has a basis but holds nothing", s.image);
            assert!(!s.subjects.is_empty(), "{} names no subject", s.image);
        }
    }
}

/// The constants that are reachable from here resolve to rows -- so renaming
/// one of these images without its row fails the build's tests, not only the
/// gate.
#[test]
fn the_crates_image_constants_resolve_to_rows() {
    for image in [
        crate::booking::IMAGE_BOOKINGS,
        crate::social::IMAGE_THREADS,
        crate::wallet::IMAGE_LEDGER,
        crate::channels::IMAGE_INBOX,
        crate::rail::IMAGE_RAILS,
        crate::outbox::IMAGE_OUTBOX,
        crate::command::till::IMAGE_TILL,
        crate::idempotency::IMAGE_IDEMPOTENCY,
        crate::services::customers::consent_log::IMAGE_CONSENT,
        crate::services::campaigns::campaign::IMAGE_CAMPAIGN,
        crate::witness::IMAGE,
        crate::fiscal::wire::IMAGE,
        crate::hubstore::IMAGE_OPS,
        crate::hubstore::IMAGE_PEOPLE,
        crate::hubstore::IMAGE_I18N,
        crate::hubstore::IMAGE_AUDIT,
        crate::hubstore::IMAGE_SETTINGS,
        crate::hubstore::IMAGE_STOCK,
        crate::platform_store::REGISTRY,
        crate::platform_store::IDENTITY,
        crate::platform_store::SESSIONS,
        crate::platform_store::COURIERS,
        crate::platform_store::WAITLIST,
        crate::platform_store::ERASURES,
        crate::platform_store::ERRORS,
    ] {
        assert!(store(image).is_some(), "image {image:?} has no registry row");
    }
    assert!(store("no-such-image").is_none());
    for kind in [
        crate::identity_store::K_USER,
        crate::identity_store::K_LOC,
        crate::identity_store::K_COURIER,
        crate::services::customers::forget::register::KIND,
    ] {
        assert!(stores().any(|s| s.kinds.contains(&kind)), "kind {kind:?} has no row");
    }
}

/// Every host that names a processor names one that exists, and every
/// processor says where it is and on what terms.
#[test]
fn hosts_point_at_real_processors_and_processors_are_complete() {
    for h in HOSTS {
        if let Recipient::Processor(id) = h.recipient {
            assert!(processor(id).is_some(), "host {} names unknown processor {id}", h.host);
        }
    }
    for p in PROCESSORS {
        assert!(!p.location.is_empty() && !p.safeguard.is_empty() && !p.terms.is_empty(), "{} is incomplete", p.id);
    }
    assert!(processor("nobody").is_none());
}

/// P11 row: the AI endpoint is listed, and says it never receives a name or phone.
#[test]
fn the_ai_endpoint_is_a_processor_that_receives_no_name_or_phone() {
    let ai = processor("ai").expect("the venue AI endpoint is in the register");
    assert_eq!(ai.switch, Switch::Ai);
    assert!(!ai.receives.contains(&Data::Name) && !ai.receives.contains(&Data::Phone));
}

#[test]
fn browser_keys_are_unique_and_prefixes_look_like_prefixes() {
    let mut seen: Vec<&str> = Vec::new();
    for k in BROWSER {
        assert!(!seen.contains(&k.key), "browser key {} twice", k.key);
        seen.push(k.key);
        if k.prefix {
            assert!(k.key.ends_with('_') || k.key.ends_with('.'), "{} is a prefix", k.key);
        }
    }
}

/// A personal store either names the row that will export it or says there is
/// nothing of the person's to export (pseudonyms, expiring copies).
#[test]
fn a_personal_store_says_how_it_will_be_exported() {
    for s in stores().filter(|s| s.personal()) {
        match s.export {
            Exporter::Missing(row) => assert!(row.starts_with('P'), "{}: {row}", s.image),
            Exporter::NotPersonal => assert!(
                matches!(s.erase, Eraser::Expires(_) | Eraser::Retain(_)),
                "{}: only a pseudonymous or expiring store may skip the export", s.image
            ),
        }
    }
}
