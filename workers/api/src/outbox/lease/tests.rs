//! W-FIX O4: two overlapping drains, through the real table operations.

use super::{give_back, take, LEASE_MS};
use dowiz_hub::table::Table;

const T0: i64 = 1_790_000_000_000;
const MIN: i64 = 60_000;

fn table() -> Table {
    Table::create(1 << 20).unwrap()
}

/// THE DEFECT. Minute N's drain is still awaiting a rail when minute N+1's
/// arrives: the second one must send nothing.
#[test]
fn a_second_drain_inside_the_lease_is_refused() {
    let mut t = table();
    assert!(take(&mut t, T0).unwrap());
    assert!(!take(&mut t, T0 + MIN).unwrap(), "the next minute overlaps the first");
    assert!(!take(&mut t, T0 + LEASE_MS - 1).unwrap());
}

/// The twin: a drain that finished gives the lease back and the next minute runs.
#[test]
fn a_finished_drain_lets_the_next_one_run() {
    let mut t = table();
    assert!(take(&mut t, T0).unwrap());
    give_back(&mut t, T0);
    assert!(take(&mut t, T0 + MIN).unwrap());
}

/// A drain cut off while holding the lease is outlived by it.
#[test]
fn an_abandoned_lease_expires() {
    let mut t = table();
    assert!(take(&mut t, T0).unwrap());
    assert!(take(&mut t, T0 + LEASE_MS).unwrap());
}

/// A drain whose lease ran out and was taken by a later drain does not give
/// back the later drain's lease when it finally finishes.
#[test]
fn a_late_drain_gives_back_only_its_own_lease() {
    let mut t = table();
    assert!(take(&mut t, T0).unwrap());
    assert!(take(&mut t, T0 + LEASE_MS).unwrap(), "the first one ran out");
    give_back(&mut t, T0);
    assert!(!take(&mut t, T0 + LEASE_MS + MIN).unwrap(), "the second still holds it");
    give_back(&mut t, T0 + LEASE_MS);
    assert!(take(&mut t, T0 + LEASE_MS + MIN).unwrap());
}
