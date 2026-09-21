//! The law, under every night it is meant to survive and every night it is
//! meant to catch. Pure, because `super::contradictions` is.

use super::*;

fn census(records: usize, tip: &str, seals: Vec<Seal>) -> Census {
    let mut c = Census {
        at_ms: 1,
        venue: "v".into(),
        records,
        tip: Some(tip.into()),
        generation: 3,
        seals,
        total: 0,
        found: Vec::new(),
    };
    c.recount();
    c
}

/// THE NIGHT NOTHING HAPPENED. A witness that reports on a quiet night is
/// a witness nobody reads by the third week.
#[test]
fn two_nights_of_an_untouched_log_agree() {
    let a = census(10, "aa", vec![]);
    let b = census(10, "aa", vec![]);
    assert!(contradictions(&a, &b, Some("log")).is_empty());
}

/// And a NORMAL night: orders were placed, so the log is longer and the tip
/// has moved. The old tip is still in the log.
#[test]
fn a_night_of_ordinary_trade_agrees() {
    let a = census(10, "aa", vec![]);
    let b = census(16, "bb", vec![]);
    assert!(contradictions(&a, &b, Some("log")).is_empty(), "{:?}", contradictions(&a, &b, Some("log")));
}

/// THE ROTATION, which is the false alarm this instrument exists to not
/// raise. Twelve records leave the hot log for a new archive; the hot log
/// is shorter, the total is unchanged, and last night's tip is held by the
/// archive rather than by the log.
#[test]
fn a_rotation_is_not_a_truncation() {
    let a = census(20, "aa", vec![]);
    let b = census(8, "bb", vec![Seal { id: "log@4".into(), records: 12, tip: Some("aa".into()) }]);
    assert_eq!(b.total, 20, "a rotation moves records; it does not lose them");
    assert!(contradictions(&a, &b, Some("log@4")).is_empty());
}

/// A PLAIN TRUNCATION: four records removed from the end. The chain still
/// verifies — that is the whole problem — and the total is the tell.
#[test]
fn records_removed_from_the_end_are_named() {
    let a = census(20, "aa", vec![]);
    let b = census(16, "bb", vec![]);
    let out = contradictions(&a, &b, None);
    assert_eq!(out.len(), 2, "{out:?}");
    assert!(out[0].contains("the history shrank: 20"));
    assert!(out[1].contains("is in neither the log nor any archive"));
}

/// THE REBUILT LOG, and it is the reason the tip is witnessed at all. The
/// count is right, the chain is perfect, every id recomputed — and the
/// record that was the tip last night is nowhere in it.
#[test]
fn a_log_rebuilt_from_scratch_keeps_the_count_and_loses_the_tip() {
    let a = census(20, "aa", vec![]);
    let b = census(20, "zz", vec![]);
    let out = contradictions(&a, &b, None);
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(out[0].contains("aa"), "the witnessed tip is named: {out:?}");
}

/// A DELETED ARCHIVE: the hot log is untouched and a year of history is
/// gone. Both the missing seal and the fallen total say so.
#[test]
fn a_deleted_archive_is_named_even_though_the_hot_log_is_perfect() {
    let a = census(8, "bb", vec![Seal { id: "log@4".into(), records: 12, tip: Some("aa".into()) }]);
    let b = census(8, "bb", vec![]);
    let out = contradictions(&a, &b, Some("log"));
    assert_eq!(out.len(), 2, "{out:?}");
    assert!(out[0].contains("shrank"));
    assert!(out[1].contains("log@4"), "the archive is named: {out:?}");
}

/// AN EDITED ARCHIVE. A cold image cannot legitimately change at all, so a
/// different count or a different tip under the same name is a rewrite —
/// and it may leave the total looking untouched, which is why the seals are
/// compared one by one rather than only summed.
#[test]
fn an_archive_rewritten_under_its_own_name_is_named() {
    let a = census(8, "bb", vec![Seal { id: "log@4".into(), records: 12, tip: Some("aa".into()) }]);
    let b = census(12, "bb", vec![Seal { id: "log@4".into(), records: 8, tip: Some("cc".into()) }]);
    assert_eq!(a.total, b.total, "the totals agree, and the archive was still rewritten");
    let out = contradictions(&a, &b, Some("log"));
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(out[0].contains("held 12 records"), "{out:?}");
}

/// A VENUE WITH NO HISTORY YET has no tip to witness, and that is not a
/// contradiction — it is a venue that has taken no orders.
#[test]
fn an_empty_log_witnesses_nothing_and_accuses_nobody() {
    let empty = Census { venue: "v".into(), ..Default::default() };
    assert!(contradictions(&empty, &empty, None).is_empty());
}

/// The stored total is never the only place it exists, because a figure
/// that is only ever read back is a figure nobody can contradict.
#[test]
fn the_total_is_recomputed_from_its_parts() {
    let mut c = census(8, "bb", vec![Seal { id: "log@4".into(), records: 12, tip: None }]);
    c.total = 999;
    c.recount();
    assert_eq!(c.total, 20);
}
