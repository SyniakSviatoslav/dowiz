//! Summaries as recurring entries: reconciled from the groups, never duplicated.
use super::*;
use crate::notify::route::{groups::Pii, Mode};
use dowiz_hub::tz::Dst;
use serde_json::json;

const UTC: Zone = Zone { standard_minutes: 0, dst: Dst::None };
/// 2026-01-15 10:00 UTC, a Thursday.
const NOW: i64 = 1_768_435_200_000 + 10 * 3_600_000;

fn g(id: &str, evs: &[(&str, Mode)]) -> Group {
    let mut g = Group::fresh(id.into(), "-1".into(), None, id.into(), "group".into(), "en");
    g.pii = Pii::None;
    g.subs = evs.iter().map(|(e, m)| (e.to_string(), *m)).collect();
    g
}

#[test]
fn a_group_that_asked_for_summaries_gets_one_entry_each_at_its_time() {
    let w = wanted(&[g("o", &[("digest.daily", Mode::Now), ("digest.weekly", Mode::Now)]), g("k", &[])], UTC, NOW);
    assert_eq!(w.iter().map(|e| (e.id.as_str(), e.kind.as_str(), e.to.as_str(), e.text.as_str())).collect::<Vec<_>>(),
        vec![("digest/o/daily", KIND, "o", "daily@540"), ("digest/o/weekly", KIND, "o", "weekly@540")]);
    assert_eq!(w[0].next_at_ms, NOW + 23 * 3_600_000, "tomorrow 09:00");
    assert!(weekly(&w[1]) && !weekly(&w[0]));
}

#[test]
fn reconcile_adds_what_is_missing_removes_what_was_switched_off_and_keeps_the_rest() {
    let want = wanted(&[g("o", &[("digest.daily", Mode::Now)])], UTC, NOW);
    let mut kept = want[0].clone();
    kept.next_at_ms = 5;
    let stale = Entry::new("digest/x/daily".into(), KIND, "x".into(), "daily@540".into(), 0);
    let (put, remove) = reconcile(&[&kept, &stale], &want);
    assert!(put.is_empty(), "an entry with the same text keeps its schedule");
    assert_eq!(remove, vec!["digest/x/daily".to_string()]);
    let mut moved = kept.clone();
    moved.text = "daily@600".into();
    let (put, remove) = reconcile(&[&moved], &want);
    assert_eq!((put.len(), remove.len()), (1, 0), "a changed time replaces the entry");
    let (put, _) = reconcile(&[], &want);
    assert_eq!(put, want);
}

#[test]
fn a_groups_lines_come_oldest_first_with_their_record_ids() {
    let recs = vec![
        ("o/b".to_string(), line_record("second", 20)),
        ("o/a".to_string(), line_record("first", 10)),
        ("other/a".to_string(), line_record("not mine", 5)),
        ("o/bad".to_string(), "garbage".to_string()),
    ];
    let (lines, ids) = lines_for(&recs, "o");
    assert_eq!(lines, vec!["first", "second"]);
    assert_eq!(ids, vec!["o/a", "o/b"]);
}

#[test]
fn the_summary_counts_the_last_day_and_names_the_dishes() {
    let o = |at: i64, total: i64| json!({ "id": format!("o{at}"), "created_at_ms": at, "status": "delivered", "total": total,
        "items": [{ "product_id": "p1", "quantity": 2, "unit_price": total / 2 }] });
    let orders = vec![o(NOW - 3_600_000, 1500), o(NOW - 2 * 86_400_000, 900)];
    let s = summary("Dubin", &orders, UTC, NOW, false, "ALL", &|id| format!("dish-{id}"));
    assert_eq!((s.orders, s.revenue.as_str()), (1, "1500 ALL"));
    assert_eq!(s.venue, "Dubin");
    let w = summary("Dubin", &orders, UTC, NOW, true, "ALL", &|id| format!("dish-{id}"));
    assert_eq!(w.orders, 2, "the week reaches the older order");
    assert_eq!(w.top.first().map(|t| t.0.as_str()), Some("dish-p1"));
}
