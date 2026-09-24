//! Which ids a firing reads, and how far the watermark may move.

use super::*;

/// Drive a walker against a platform where `exists` says which ids answer.
fn run(mut w: Walker, exists: impl Fn(i64) -> bool) -> (Vec<Step>, Walker) {
    let mut steps = Vec::new();
    loop {
        let s = w.step();
        steps.push(s);
        match s {
            Step::Done => return (steps, w),
            Step::Listed(_) => w.done(),
            Step::Fetch(id) if exists(id) => w.done(),
            Step::Fetch(_) => w.missing(),
        }
    }
}

/// The courses the list hides are fetched; the bills it shows are not.
#[test]
fn the_gaps_are_fetched_and_the_listed_bills_are_not() {
    // Listed: 8691 (bill) and 8693 (bill); 8689, 8690, 8692 are courses.
    let w = Walker::new(8688, &[8691, 8693], &[8691, 8693], BUDGET, 0, 0);
    let (steps, w) = run(w, |id| id <= 8693);
    assert_eq!(
        steps[..5],
        [Step::Fetch(8689), Step::Fetch(8690), Step::Listed(8691), Step::Fetch(8692), Step::Listed(8693)]
    );
    assert_eq!(steps[5], Step::Fetch(8694), "then one probe past the newest");
    assert_eq!(steps.last(), Some(&Step::Done));
    assert_eq!(w.handled, 8693, "8694 answered 404: the end so far, not handled");
    assert!(!w.backlog);
}

/// A course rung up at a table still open is newer than every listed bill.
#[test]
fn courses_beyond_the_newest_bill_are_probed_and_bounded() {
    let (steps, w) = run(Walker::new(100, &[100], &[100], BUDGET, 0, 0), |_| true);
    let fetched: Vec<i64> = steps.iter().filter_map(|s| if let Step::Fetch(i) = s { Some(*i) } else { None }).collect();
    assert_eq!(fetched, (101..=100 + PROBE).collect::<Vec<_>>());
    assert_eq!(w.handled, 100 + PROBE);
}

/// A 404 INSIDE the listed range is a deleted draft: handled, never retried.
#[test]
fn a_gap_that_does_not_exist_is_passed_not_retried() {
    let (_, w) = run(Walker::new(10, &[14], &[14], BUDGET, 0, 0), |id| id != 12);
    assert!(w.handled >= 14, "12 is passed over: {}", w.handled);
}

/// THE BUDGET: the watermark stops at the last id read, and a backlog says so.
#[test]
fn the_budget_bounds_a_firing_and_leaves_a_backlog() {
    let (steps, w) = run(Walker::new(0, &[1000, 1050], &[1050], 3, 0, 0), |_| true);
    assert_eq!(steps, vec![Step::Fetch(1000), Step::Fetch(1001), Step::Fetch(1002), Step::Done]);
    assert_eq!(w.handled, 1002);
    assert!(w.backlog);
}

/// A first run starts at the oldest listed id; with nothing listed it does
/// nothing at all rather than walk from id 1.
#[test]
fn a_first_run_starts_at_the_window_and_an_empty_one_does_nothing() {
    let (steps, _) = run(Walker::new(0, &[], &[], BUDGET, 0, 0), |_| true);
    assert_eq!(steps, vec![Step::Done]);
    let (steps, _) = run(Walker::new(0, &[500], &[500], BUDGET, 0, 0), |id| id <= 500);
    assert_eq!(steps[0], Step::Listed(500));
}

/// THE FIRST RUN READS BACK: `lead` ids before the window, marked as leads,
/// and the boundary kept for the firings that finish the catch-up.
#[test]
fn a_first_run_reads_leads_before_the_window() {
    let w = Walker::new(0, &[100, 110], &[110], BUDGET, 5, 0);
    assert_eq!(w.lead_below, 100);
    assert!(w.is_lead(99) && !w.is_lead(100));
    let (steps, w) = run(w, |id| id <= 110);
    assert_eq!(steps[0], Step::Fetch(95), "five ids back");
    assert_eq!(w.handled, 110);
    // A later firing keeps the boundary it was given; a venue whose first
    // run is long past has none.
    assert_eq!(Walker::new(110, &[120], &[], BUDGET, 5, 100).lead_below, 100);
    assert!(!Walker::new(110, &[120], &[], BUDGET, 5, 0).is_lead(50));
    // Never below id 1.
    let (steps, _) = run(Walker::new(0, &[3], &[3], BUDGET, 40, 0), |_| true);
    assert_eq!(steps[0], Step::Fetch(1));
}
