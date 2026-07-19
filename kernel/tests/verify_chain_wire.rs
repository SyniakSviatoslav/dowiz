//! P-H (audit hash-chain) — kernel-local caller wiring (RED→GREEN).
//!
//! Wires `event_log::verify_chain_before_trust` — a real production caller for
//! the `EventLog::verify_chain` mesh-event-log verifier that had ZERO callers
//! (only module tests exercised it). The caller verifies a persisted/replayed
//! audit chain BEFORE trusting it, per P-H "no chain ⇒ no mesh". Before this
//! wiring the caller did not exist ⇒ the test fails to compile (RED). After
//! wiring it compiles and passes (GREEN).

use dowiz_kernel::event_log::{verify_chain_before_trust, ChainDefect, EventLog, MemEventStore, MeshEvent};

fn genesis_event(payload: &[u8]) -> MeshEvent {
    MeshEvent {
        prev: [0u8; 32],
        actor_pubkey: [7u8; 32],
        actor_seq: 0,
        payload: payload.to_vec(),
    }
}

fn chained(prev: [u8; 32], seq: u64, payload: &[u8]) -> MeshEvent {
    MeshEvent {
        prev,
        actor_pubkey: [7u8; 32],
        actor_seq: seq,
        payload: payload.to_vec(),
    }
}

#[test]
fn ph_verify_chain_before_trust_accepts_valid_chain() {
    let mut log = EventLog::new(MemEventStore::new());
    let e0 = genesis_event(b"genesis");
    let p0 = e0.event_id();
    let e1 = chained(p0, 1, b"append-1");
    let p1 = e1.event_id();
    let e2 = chained(p1, 2, b"append-2");
    let tip = e2.event_id();

    let _ = log.append_raw(e0);
    let _ = log.append_raw(e1);
    let _ = log.append_raw(e2);

    // The kernel-local caller verifies the persisted chain before trusting it.
    let head = verify_chain_before_trust(&log).expect("valid chain must verify");
    // Trusted head is the event-id of the final event (the chain tip).
    assert_eq!(head, tip);
}

#[test]
fn ph_verify_chain_before_trust_rejects_broken_prev() {
    let mut good = EventLog::new(MemEventStore::new());
    let e0 = genesis_event(b"genesis");
    let _ = good.append_raw(e0);
    // A separate, corrupt chain: prev points at a non-existent hash.
    let mut bad = EventLog::new(MemEventStore::new());
    let mut broken = chained([0x99u8; 32], 1, b"append-1");
    broken.prev = [0x99u8; 32];
    let _ = bad.append_raw(broken);

    // The good chain verifies; the broken one is rejected (fail-closed).
    assert!(verify_chain_before_trust(&good).is_ok());
    assert!(
        matches!(
            verify_chain_before_trust(&bad),
            Err(ChainDefect::BrokenPrev { .. })
        ),
        "broken prev must be rejected, not trusted"
    );
}
