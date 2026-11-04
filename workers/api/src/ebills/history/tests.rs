//! A first run starts at the month's oldest sale; every later run is the walk's own.

use super::*;
use crate::ebills::state::{Plan, HISTORY_DAYS};
use crate::ebills::tests::bill_listed;
use serde_json::json;

fn list_of(ids: &[i64]) -> SaleList {
    let rows: Vec<_> = ids.iter().map(|id| { let mut b = bill_listed(); b["id"] = json!(id); b }).collect();
    super::super::parse_list(&json!({ "sales": rows, "total": 0.0 }).to_string()).unwrap()
}

/// Only a venue that has handled nothing and kept no lead boundary is on its first run.
#[test]
fn a_first_run_is_nothing_handled_and_no_boundary() {
    assert!(first_run(&Plan::default()));
    assert!(!first_run(&Plan { watermark: 8601, ..Plan::default() }), "the walk has begun");
    assert!(!first_run(&Plan { lead_below: 8601, ..Plan::default() }), "a first firing that moved nothing still fixed the boundary");
}

/// The month's oldest id joins the window's when it is older; an empty month
/// or one the window already reaches adds nothing.
#[test]
fn the_months_oldest_sale_is_added_only_when_it_is_older_than_the_window() {
    assert_eq!(floor_of(&[8601, 8602], &list_of(&[9329])), None, "the window reaches it");
    assert_eq!(floor_of(&[9400, 9402], &list_of(&[9329])), Some(9329));
    assert_eq!(floor_of(&[], &list_of(&[9329])), Some(9329), "a closed day still has a month behind it");
    assert_eq!(floor_of(&[9400], &list_of(&[])), None, "a till with no history");
    assert_eq!(floor_of(&[9400], &list_of(&[9329, 9330])), Some(9329), "the smallest, whatever the order");
}

/// The plan carries the window a first run asks for: thirty days back, on the venue's wall clock.
#[test]
fn the_plan_names_thirty_days_back() {
    use crate::ebills::state::{plan, Config, State};
    let cfg = Config { enabled: true, pos_id: 1, user: "u".into(), secret: "s".into() };
    // 2026-11-03T12:00Z (1_793_880_000_000 is 11-05, two days on); local = UTC here, so
    // `history_from` is 2026-10-04 -- the window the live probe of 2026-11-03 read 855 sales in.
    const NOON: i64 = 1_793_880_000_000 - 2 * 86_400_000;
    let p = plan(Some(&cfg), &State::default(), NOON, true, NOON);
    assert_eq!(HISTORY_DAYS, 30);
    assert_eq!((p.today.as_str(), p.history_from.as_str()), ("2026-11-03", "2026-10-04"));
}
