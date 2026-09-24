//! G1 / D1 -- the retry after a REFUSAL, through the same table operations
//! `begin` (`claim`) and `Guard::{refused, answered, release}` (`record`,
//! `release`) run inside the venue's object.

use super::*;

const T0: i64 = 1_790_000_000_000;

fn table() -> Table {
    Table::create(IDEMPOTENCY_BYTES).unwrap()
}

/// THE DEFECT. A waiter's pay is refused ("payment sum exceeds the total"),
/// then the outbox retries the same key. Before G1 nothing was recorded and
/// the retry was told "still running" for a day; now it is given the refusal.
#[test]
fn a_refused_call_is_replayed_as_the_refusal_not_as_still_running() {
    let mut t = table();
    assert_eq!(claim(&mut t, "k", "p", T0).unwrap(), Seen::Claimed);
    assert!(keeps(409));
    record(&mut t, "k", "p", T0, 409, "payment sum 1001 exceeds the total 1000", "text/plain").unwrap();
    let again = claim(&mut t, "k", "p", T0 + 1_000).unwrap();
    assert_eq!(
        again,
        Seen::Answered { status: 409, body: "payment sum 1001 exceeds the total 1000".into(), ctype: "text/plain".into() }
    );
}

/// The positive twin: a success replays as the success, JSON by default for
/// records written before the content type was stored.
#[test]
fn a_success_replays_as_the_success() {
    let mut t = table();
    claim(&mut t, "k", "p", T0).unwrap();
    t.put(KIND, "k", r#"{"print":"p","at_ms":1,"done":true,"status":200,"body":"{\"seq\":3}"}"#, &[], &[]).unwrap();
    assert_eq!(
        claim(&mut t, "k", "p", T0).unwrap(),
        Seen::Answered { status: 200, body: r#"{"seq":3}"#.into(), ctype: "application/json".into() }
    );
}

/// A 5xx is NOT kept: the claim is given back and the retry runs again.
#[test]
fn a_server_failure_gives_the_claim_back_and_the_retry_runs() {
    let mut t = table();
    claim(&mut t, "k", "p", T0).unwrap();
    assert!(!keeps(500) && !keeps(503));
    release(&mut t, "k");
    assert_eq!(claim(&mut t, "k", "p", T0 + 10).unwrap(), Seen::Claimed);
}

/// `release` never erases an ANSWER -- a late release after a recorded
/// refusal must not turn the refusal back into a fresh run.
#[test]
fn release_does_not_erase_a_recorded_answer() {
    let mut t = table();
    claim(&mut t, "k", "p", T0).unwrap();
    record(&mut t, "k", "p", T0, 422, "no", "text/plain").unwrap();
    release(&mut t, "k");
    assert!(matches!(claim(&mut t, "k", "p", T0).unwrap(), Seen::Answered { status: 422, .. }));
}

/// Rule 4 still holds inside the lease...
#[test]
fn a_retry_inside_the_lease_is_told_still_running() {
    let mut t = table();
    claim(&mut t, "k", "p", T0).unwrap();
    assert_eq!(claim(&mut t, "k", "p", T0 + LEASE_MS - 1).unwrap(), Seen::Running);
}

/// ...and a claim a cut-off Worker never finished is abandoned after it, not
/// after the 24-hour keep window.
#[test]
fn an_abandoned_claim_is_reclaimed_after_the_lease_not_after_a_day() {
    let mut t = table();
    claim(&mut t, "k", "p", T0).unwrap();
    assert!(LEASE_MS < super::super::KEEP_MS);
    assert_eq!(claim(&mut t, "k", "p", T0 + LEASE_MS).unwrap(), Seen::Claimed);
    // Re-claimed with a fresh stamp: the next retry is inside the new lease.
    assert_eq!(claim(&mut t, "k", "p", T0 + LEASE_MS + 1).unwrap(), Seen::Running);
}

/// Rule 3: a different body under the same key is refused, answered or not.
#[test]
fn a_different_body_under_the_key_is_a_mismatch() {
    let mut t = table();
    claim(&mut t, "k", "p", T0).unwrap();
    assert_eq!(claim(&mut t, "k", "other", T0).unwrap(), Seen::Mismatch);
    record(&mut t, "k", "p", T0, 409, "no", "text/plain").unwrap();
    assert_eq!(claim(&mut t, "k", "other", T0).unwrap(), Seen::Mismatch);
}
