//! /api/version: what the deploy script writes comes back verbatim; anything
//! else comes back as "unknown" -- each refusal beside its positive twin.

use super::*;

const SHA: &str = "c0262881a3b4c5d6e7f80912a3b4c5d6e7f80912";

#[test]
fn a_full_sha_and_a_utc_time_come_back_verbatim() {
    let b = body(Some(SHA), Some("2026-09-30T12:48:40Z"));
    assert_eq!(b["commit"], SHA);
    assert_eq!(b["built_at"], "2026-09-30T12:48:40Z");
    assert_eq!(b.as_object().unwrap().len(), 2, "no other field: {b}");
}

#[test]
fn an_unset_build_says_unknown_not_empty() {
    let b = body(None, None);
    assert_eq!(b["commit"], "unknown");
    assert_eq!(b["built_at"], "unknown");
}

#[test]
fn a_short_uppercase_or_padded_commit_is_unknown() {
    assert_eq!(commit_or_unknown(Some(SHA)), SHA);
    assert_eq!(commit_or_unknown(Some(&SHA[..12])), "unknown");
    assert_eq!(commit_or_unknown(Some(&SHA.to_uppercase())), "unknown");
    assert_eq!(commit_or_unknown(Some(&format!("{SHA}\n"))), "unknown");
    assert_eq!(commit_or_unknown(Some("c0262881a3b4c5d6e7f80912a3b4c5d6e7f8091g")), "unknown");
}

#[test]
fn a_time_in_another_shape_is_unknown() {
    assert_eq!(built_at_or_unknown(Some("2026-09-30T12:48:40Z")), "2026-09-30T12:48:40Z");
    assert_eq!(built_at_or_unknown(Some("2026-09-30 12:48:40Z")), "unknown");
    assert_eq!(built_at_or_unknown(Some("2026-09-30T12:48:40+02:00")), "unknown");
    assert_eq!(built_at_or_unknown(Some("<script>alert(1)</script>")), "unknown");
}

#[test]
fn a_test_build_reports_what_its_environment_set() {
    // `cargo test` runs without DOWIZ_COMMIT unless someone exported it; either
    // way the served body is exactly body() of the compiled-in constants.
    let b = body(COMMIT, BUILT_AT);
    assert_eq!(b["commit"], commit_or_unknown(COMMIT));
}
