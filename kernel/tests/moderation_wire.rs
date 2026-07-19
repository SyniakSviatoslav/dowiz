//! P74 wiring gate — `moderation::decide_report` must ride the real commit path
//! through a production caller (`moderation::commit_report`), which wires it as the
//! validate-before-persist Law pole of `EventLog::commit_after_decide`.
//!
//! RED before wiring: `commit_report` does not exist → this test fails to compile.
//! GREEN after wiring: a malformed report payload → `CommitError` and NOTHING
//! persists; a well-formed report → persisted (log len 1).

use dowiz_kernel::event_log::{AppendOutcome, EventLog, MemEventStore, MeshEvent};
use dowiz_kernel::moderation::{
    commit_report, ReportPayload, ReportReason, ReportTarget, DOMAIN_REPORT,
};

fn reporter() -> [u8; 32] {
    [7u8; 32]
}

#[test]
fn p74_valid_report_persists_through_commit_report() {
    let mut log = EventLog::new(MemEventStore::new());
    let p = ReportPayload {
        target: ReportTarget::Actor([9u8; 32]),
        reason: ReportReason::Harassment,
        note: b"flagging repeated abuse".to_vec(),
    };
    let ev = MeshEvent {
        prev: [0u8; 32],
        actor_pubkey: reporter(),
        actor_seq: 1,
        payload: p.encode(),
    };
    let out = commit_report(&mut log, ev).expect("valid report must commit");
    assert!(matches!(out, AppendOutcome::Committed(_)));
    assert_eq!(log.len(), 1, "one logical report = one persisted row");
}

#[test]
fn p74_malformed_report_rejected_nothing_persists() {
    let mut log = EventLog::new(MemEventStore::new());
    // Adversarial: reason byte 100 is outside the abuse enum (a §16.59 quality-code
    // smuggle). The Law pole must reject and persist nothing.
    let mut bytes = DOMAIN_REPORT.to_vec();
    bytes.push(0); // Hub target
    bytes.push(100); // UnknownReason
    bytes.extend_from_slice(&0u32.to_le_bytes()); // empty note
    let ev = MeshEvent {
        prev: [0u8; 32],
        actor_pubkey: reporter(),
        actor_seq: 1,
        payload: bytes,
    };
    let res = commit_report(&mut log, ev);
    assert!(res.is_err(), "malformed report must be Law-rejected");
    assert_eq!(log.len(), 0, "rejected report persists nothing");
}
